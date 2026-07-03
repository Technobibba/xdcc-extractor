mod config;
mod extractor;
mod history;
mod queue;

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

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    let config = config::Config::load("config.toml")?;

    let watch_path = config.watch.directory.clone();
    let stable_after_seconds = config.watch.stable_after;

    let delete_archives = config.extract.delete_archives;
    let keep_failed = config.extract.keep_failed;

    let history = history::History::new(&config.history.directory)?;

    info!("XDCC Extractor startet...");
    info!("{:#?}", config);
    info!("Überwache {}", watch_path);
    info!(
        "Release gilt nach {} Sekunden ohne Änderung als fertig",
        stable_after_seconds
    );
    info!("Archive nach Erfolg löschen: {}", delete_archives);
    info!("Fehlerhafte Archive behalten: {}", keep_failed);
    info!("History-Ordner: {}", config.history.directory);

    let (tx, rx) = channel();

    let mut watcher = RecommendedWatcher::new(
        move |res| {
            let _ = tx.send(res);
        },
        NotifyConfig::default(),
    )?;

    watcher.watch(Path::new(&watch_path), RecursiveMode::Recursive)?;

    let mut releases: Vec<ReleaseCandidate> = Vec::new();
    let mut known_ready: HashSet<PathBuf> = HashSet::new();
    let mut queue = JobQueue::new();

    scan_existing_releases(Path::new(&watch_path), &mut releases, &history)?;

    loop {
        match rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(event)) => handle_event(event, &mut releases),
            Ok(Err(e)) => error!("Watch Error: {:?}", e),
            Err(_) => {}
        }

        check_ready_releases(
            &releases,
            &mut known_ready,
            &mut queue,
            &history,
            stable_after_seconds,
        );

        process_next_job(&mut queue, &history, delete_archives, keep_failed);
    }
}

fn scan_existing_releases(
    watch_path: &Path,
    releases: &mut Vec<ReleaseCandidate>,
    history: &history::History,
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

        if let Some(release_dir) = detect_release_dir(path) {
            if history.is_done(&release_dir) {
                info!(
                    "Bereits verarbeitet, überspringe beim Startup-Scan: {}",
                    release_dir.display()
                );
                continue;
            }

            upsert_release(releases, release_dir);
        }
    }

    info!(
        "Startup-Scan abgeschlossen: {} Kandidat(en)",
        releases.len()
    );

    Ok(())
}

fn handle_event(event: Event, releases: &mut Vec<ReleaseCandidate>) {
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

                let Some(release_dir) = detect_release_dir(&path) else {
                    continue;
                };

                let known_release = releases.iter().any(|release| release.path == release_dir);
                let archive_related = extractor::is_archive_related_file(&path);

                if known_release || archive_related {
                    upsert_release(releases, release_dir);
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

fn detect_release_dir(file: &Path) -> Option<PathBuf> {
    file.parent().map(|p| p.to_path_buf())
}

fn upsert_release(releases: &mut Vec<ReleaseCandidate>, release_dir: PathBuf) {
    if let Some(existing) = releases.iter_mut().find(|r| r.path == release_dir) {
        existing.last_seen = Instant::now();
        info!("Release aktualisiert: {}", existing.path.display());
    } else {
        info!("Neues Release erkannt: {}", release_dir.display());

        releases.push(ReleaseCandidate {
            path: release_dir,
            last_seen: Instant::now(),
        });
    }
}

fn check_ready_releases(
    releases: &[ReleaseCandidate],
    known_ready: &mut HashSet<PathBuf>,
    queue: &mut JobQueue,
    history: &history::History,
    stable_after_seconds: u64,
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

        if age >= stable_after_seconds {
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
        } else {
            warn!("Release wartet noch: {} / {}s", release.path.display(), age);
        }
    }
}

fn process_next_job(
    queue: &mut JobQueue,
    history: &history::History,
    delete_archives: bool,
    keep_failed: bool,
) {
    if queue.is_empty() {
        return;
    }

    info!("Queue enthält {} Job(s)", queue.len());

    let Some(job) = queue.pop() else {
        return;
    };

    info!("Starte Job: {}", job.display());

    match extractor::process_release(&job, delete_archives, keep_failed) {
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
                }
            }
        }
        Err(err) => {
            error!("Job fehlgeschlagen: {}", job.display());
            error!("{:?}", err);
        }
    }
}
