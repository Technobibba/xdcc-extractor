mod config;

use notify::{
    Config as NotifyConfig, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::mpsc::channel,
    time::{Duration, Instant},
};

use tracing::{error, info, warn};

#[derive(Debug, Clone)]
struct ReleaseCandidate {
    path: PathBuf,
    last_seen: Instant,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    let config = config::Config::load("config.toml")?;

    info!("XDCC Extractor startet...");
    info!("{:#?}", config);

    let watch_path = config.watch.directory.clone();
    let stable_after_seconds = config.watch.stable_after;
    let delete_archives = config.extract.delete_archives;
    let keep_failed = config.extract.keep_failed;

    info!("Überwache {}", watch_path);
    info!(
        "Release gilt nach {} Sekunden ohne Änderung als fertig",
        stable_after_seconds
    );
    info!("Archive nach Erfolg löschen: {}", delete_archives);
    info!("Fehlerhafte Archive behalten: {}", keep_failed);

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

    loop {
        match rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(event)) => handle_event(event, &mut releases),
            Ok(Err(e)) => error!("Watch Error: {:?}", e),
            Err(_) => {}
        }

        check_ready_releases(&releases, &mut known_ready, stable_after_seconds);
    }
}

fn handle_event(event: Event, releases: &mut Vec<ReleaseCandidate>) {
    match event.kind {
        EventKind::Create(_) | EventKind::Modify(_) => {
            for path in event.paths {
                if path.is_file() {
                    if let Some(release_dir) = detect_release_dir(&path) {
                        upsert_release(releases, release_dir);
                    }
                }
            }
        }
        _ => {}
    }
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
    stable_after_seconds: u64,
) {
    for release in releases {
        if known_ready.contains(&release.path) {
            continue;
        }

        let age = release.last_seen.elapsed().as_secs();

        if age >= stable_after_seconds {
            info!("Release ist bereit: {}", release.path.display());
            known_ready.insert(release.path.clone());
        } else {
            warn!("Release wartet noch: {} / {}s", release.path.display(), age);
        }
    }
}
