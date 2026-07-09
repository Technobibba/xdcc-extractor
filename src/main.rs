mod config;
mod dry_run_check;
mod dry_run_report;
mod extractor;
mod history;
mod log_buffer;
mod maintenance;
mod manual_process;
mod notifications;
mod passwords;
mod queue;
mod scan;
mod status;
mod web;
mod web_api;
mod web_assets;
mod web_history;
mod web_maintenance;
mod web_pages;
mod web_settings;

use notify::{
    Config as NotifyConfig, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};

use queue::JobQueue;

use std::{
    collections::HashSet,
    path::{Component, Path, PathBuf},
    sync::mpsc::channel,
    time::{Duration, Instant},
};

use tracing::{error, info, warn};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
struct ReleaseCandidate {
    path: PathBuf,
    last_seen: Instant,
}

#[derive(Debug)]
enum JobResult {
    Success,
    Failed(PathBuf),
    NoJob,
}

#[derive(Debug, Clone, Copy)]
struct RetrySettings {
    base_delay: u64,
    max_delay: u64,
}

fn main() -> anyhow::Result<()> {
    status::validate_cli_args()?;
    maintenance::validate_clear_failed_args()?;
    manual_process::validate_process_args()?;

    if status::is_help_command() {
        status::print_help();
        return Ok(());
    }

    if status::is_version_command() {
        status::print_version();
        return Ok(());
    }

    if status::is_status_command() {
        return status::run_from_args();
    }

    if scan::is_scan_command() {
        return scan::run_from_args();
    }

    if maintenance::is_clear_failed_command() {
        return maintenance::run_clear_failed_from_args();
    }

    if manual_process::is_process_command() {
        return manual_process::run_from_args();
    }

    if dry_run_report::is_dry_run_report_command() {
        return dry_run_report::run_from_args();
    }

    if dry_run_check::is_dry_run_check_command() {
        return dry_run_check::run_from_args();
    }

    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(log_buffer::layer())
        .init();

    let config_path = status::config_path_from_args();
    let config = config::Config::load(&config_path)?;

    let watch_path = config.watch.directory.clone();
    let watch_root = PathBuf::from(&watch_path);
    let stable_after_seconds = config.watch.stable_after;
    let allow_root_archives = config.watch.allow_root_archives;
    let output_directory = PathBuf::from(&config.output.directory);

    let delete_archives = config.extract.delete_archives;
    let dry_run = config.extract.dry_run;
    let keep_failed = config.extract.keep_failed;
    let passwords = passwords::load_passwords(&config.extract.password_file)?;

    let startup_scan_existing = config.startup.scan_existing;

    let retry = RetrySettings {
        base_delay: config.retry.base_delay,
        max_delay: config.retry.max_delay,
    };

    let history = history::History::new(&config.history.directory)?;
    let notifications = notifications::Notifications::new(config.notifications.clone());

    info!("XDCC Extractor startet...");
    info!("Config-Datei: {}", config_path);
    info!("Überwache {}", watch_root.display());
    info!("Root-Archive erlaubt: {}", allow_root_archives);
    info!("Output-Ordner: {}", output_directory.display());
    info!(
        "Release gilt nach {} Sekunden ohne Änderung als fertig",
        stable_after_seconds
    );
    info!("Archive nach Erfolg löschen: {}", delete_archives);
    info!("Dry-Run aktiv: {}", dry_run);
    info!("Fehlerhafte Archive behalten: {}", keep_failed);
    info!("Passwortdatei: {}", config.extract.password_file);
    info!("Geladene Passwörter: {}", passwords.len());
    info!("History-Ordner: {}", config.history.directory);
    info!("Retry base_delay={}s", retry.base_delay);
    info!("Retry max_delay={}s", retry.max_delay);
    info!("Startup-Scan aktiviert: {}", startup_scan_existing);
    info!("Gotify aktiviert: {}", notifications.gotify_enabled());
    web::start(config.clone(), config_path.clone())?;

    let (tx, rx) = channel();

    let mut watcher = RecommendedWatcher::new(
        move |res| {
            let _ = tx.send(res);
        },
        NotifyConfig::default(),
    )?;

    watcher.watch(&watch_root, RecursiveMode::Recursive)?;

    let mut releases: Vec<ReleaseCandidate> = Vec::new();
    let mut known_ready: HashSet<PathBuf> = HashSet::new();
    let mut queue = JobQueue::new();

    if startup_scan_existing {
        scan_existing_releases(&watch_root, &mut releases, &history, allow_root_archives)?;
    } else {
        info!("Startup-Scan deaktiviert. Vorhandene Releases werden ignoriert.");
    }

    loop {
        match rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(event)) => handle_event(event, &mut releases, &watch_root, allow_root_archives),
            Ok(Err(e)) => error!("Watch Error: {:?}", e),
            Err(_) => {}
        }

        check_ready_releases(
            &releases,
            &mut known_ready,
            &mut queue,
            &history,
            stable_after_seconds,
            retry,
        );

        match process_next_job(
            &mut queue,
            &history,
            &notifications,
            &output_directory,
            delete_archives,
            dry_run,
            keep_failed,
            &passwords,
        ) {
            JobResult::Success | JobResult::NoJob => {}
            JobResult::Failed(path) => {
                warn!(
                    "Release wird nach Fehler später erneut geprüft: {}",
                    path.display()
                );

                known_ready.remove(&path);
                reset_release_timer(&mut releases, &path);
            }
        }
    }
}

fn scan_existing_releases(
    watch_path: &Path,
    releases: &mut Vec<ReleaseCandidate>,
    history: &history::History,
    allow_root_archives: bool,
) -> anyhow::Result<()> {
    info!("Scanne vorhandene Releases in {}", watch_path.display());

    for entry in WalkDir::new(watch_path)
        .follow_links(false)
        .into_iter()
        .filter_map(|entry| entry.ok())
    {
        let path = entry.path();

        if is_ignored_path(path) {
            continue;
        }

        if !path.is_file() {
            continue;
        }

        if !extractor::is_archive_related_file(path) {
            continue;
        }

        if let Some(target) = detect_release_target(path, watch_path, allow_root_archives) {
            if history.is_done(&target) {
                info!(
                    "Bereits verarbeitet, überspringe beim Startup-Scan: {}",
                    target.display()
                );
                continue;
            }

            upsert_release(releases, target);
        }
    }

    info!(
        "Startup-Scan abgeschlossen: {} Kandidat(en)",
        releases.len()
    );

    Ok(())
}

fn handle_event(
    event: Event,
    releases: &mut Vec<ReleaseCandidate>,
    watch_path: &Path,
    allow_root_archives: bool,
) {
    match event.kind {
        EventKind::Create(_) | EventKind::Modify(_) => {
            for path in event.paths {
                if is_ignored_path(&path) {
                    info!("Ignoriere internen Pfad: {}", path.display());
                    continue;
                }

                if !path.is_file() {
                    continue;
                }

                let Some(target) = detect_release_target(&path, watch_path, allow_root_archives)
                else {
                    continue;
                };

                let known_release = releases.iter().any(|release| release.path == target);
                let archive_related = extractor::is_archive_related_file(&path);

                if known_release || archive_related {
                    upsert_release(releases, target);
                } else {
                    info!("Ignoriere Nicht-Archiv-Datei: {}", path.display());
                }
            }
        }
        _ => {}
    }
}

fn is_ignored_path(path: &Path) -> bool {
    path.components().any(|component| match component {
        Component::Normal(name) => {
            let name = name.to_string_lossy();

            name == "_extracted"
                || name == "_failed"
                || name == "_processing"
                || name == ".xdcc-worker"
        }
        _ => false,
    })
}

fn detect_release_target(
    file: &Path,
    watch_path: &Path,
    allow_root_archives: bool,
) -> Option<PathBuf> {
    let parent = file.parent()?;

    if parent == watch_path {
        if !allow_root_archives {
            warn!(
                "Ignoriere Datei direkt im Watch-Root. allow_root_archives=false: {}",
                file.display()
            );
            return None;
        }

        let Some(target) = extractor::root_archive_target(file) else {
            info!(
                "Ignoriere Root-Datei, kein unterstütztes Archivformat: {}",
                file.display()
            );
            return None;
        };

        info!(
            "Root-Archiv erkannt: {} -> Ziel {}",
            file.display(),
            target.display()
        );

        return Some(target);
    }

    Some(parent.to_path_buf())
}

fn upsert_release(releases: &mut Vec<ReleaseCandidate>, release_target: PathBuf) {
    if let Some(existing) = releases.iter_mut().find(|r| r.path == release_target) {
        existing.last_seen = Instant::now();
        info!("Release aktualisiert: {}", existing.path.display());
    } else {
        info!("Neues Release erkannt: {}", release_target.display());

        releases.push(ReleaseCandidate {
            path: release_target,
            last_seen: Instant::now(),
        });
    }
}

fn reset_release_timer(releases: &mut Vec<ReleaseCandidate>, release_target: &Path) {
    if let Some(existing) = releases.iter_mut().find(|r| r.path == release_target) {
        existing.last_seen = Instant::now();
        info!("Release-Timer zurückgesetzt: {}", existing.path.display());
    }
}

fn retry_delay_seconds(attempts: u64, retry: RetrySettings) -> u64 {
    if attempts == 0 {
        return 0;
    }

    let multiplier = 2_u64.saturating_pow((attempts - 1).min(10) as u32);
    let delay = retry.base_delay.saturating_mul(multiplier);

    delay.min(retry.max_delay)
}

fn check_ready_releases(
    releases: &[ReleaseCandidate],
    known_ready: &mut HashSet<PathBuf>,
    queue: &mut JobQueue,
    history: &history::History,
    stable_after_seconds: u64,
    retry: RetrySettings,
) {
    for release in releases {
        if known_ready.contains(&release.path) {
            continue;
        }

        if history.is_done(&release.path) {
            info!(
                "Release wurde bereits verarbeitet, überspringe: {}",
                release.path.display()
            );
            known_ready.insert(release.path.clone());
            continue;
        }

        let age = release.last_seen.elapsed().as_secs();
        let attempts = history.failed_attempts(&release.path).unwrap_or(0);
        let retry_delay = retry_delay_seconds(attempts, retry);
        let required_wait = stable_after_seconds.max(retry_delay);

        if age < required_wait {
            if attempts > 0 {
                warn!(
                    "Release wartet nach Fehlversuch {} noch: {} / {}s",
                    attempts,
                    release.path.display(),
                    age
                );
            } else {
                warn!("Release wartet noch: {} / {}s", release.path.display(), age);
            }

            continue;
        }

        match extractor::has_archive_start(&release.path) {
            Ok(true) => {}
            Ok(false) => {
                warn!(
                    "Release hat noch kein Startarchiv, warte weiter: {}",
                    release.path.display()
                );
                continue;
            }
            Err(err) => {
                error!(
                    "Konnte Release nicht auf Startarchiv prüfen: {}",
                    release.path.display()
                );
                error!("{:?}", err);
                continue;
            }
        }

        info!("Release ist bereit: {}", release.path.display());

        let added = queue.push(release.path.clone());

        if added {
            info!("Release zur Queue hinzugefügt: {}", release.path.display());
        } else {
            info!(
                "Release war bereits in der Queue: {}",
                release.path.display()
            );
        }

        known_ready.insert(release.path.clone());
    }
}

fn process_next_job(
    queue: &mut JobQueue,
    history: &history::History,
    notifications: &notifications::Notifications,
    output_base: &Path,
    delete_archives: bool,
    dry_run: bool,
    keep_failed: bool,
    passwords: &[String],
) -> JobResult {
    if queue.is_empty() {
        return JobResult::NoJob;
    }

    info!("Queue enthält {} Job(s)", queue.len());

    let Some(job) = queue.pop() else {
        return JobResult::NoJob;
    };

    info!("Starte Job: {}", job.display());

    match extractor::process_release(
        &job,
        output_base,
        delete_archives,
        dry_run,
        keep_failed,
        passwords,
    ) {
        Ok(()) => {
            info!("Job abgeschlossen: {}", job.display());

            match history.mark_done(&job) {
                Ok(()) => {
                    info!(
                        "History gespeichert: {}",
                        history.marker_path(&job).display()
                    );
                }
                Err(err) => {
                    error!("Konnte History nicht speichern: {:?}", err);
                    return JobResult::Failed(job);
                }
            }

            notifications.send_success(&job);

            JobResult::Success
        }
        Err(err) => {
            let error_text = format!("{:?}", err);

            error!("Job fehlgeschlagen: {}", job.display());
            error!("{}", error_text);

            match history.mark_failed(&job, &error_text) {
                Ok(()) => {
                    let attempts = history.failed_attempts(&job).unwrap_or(0);

                    warn!(
                        "Fehlerstatus gespeichert: {}",
                        history.failed_marker_path(&job).display()
                    );
                    warn!("Fehlversuche bisher: {}", attempts);
                    notifications.send_failure(&job, attempts, &error_text);
                }
                Err(history_err) => {
                    error!("Konnte Fehlerstatus nicht speichern: {:?}", history_err);
                }
            }

            JobResult::Failed(job)
        }
    }
}
