use crate::types::ReleaseCandidate;
use std::{
    path::{Path, PathBuf},
    time::Instant,
};

pub fn detect_release_dir(file: &Path) -> Option<PathBuf> {
    file.parent().map(|p| p.to_path_buf())
}

pub fn update_release(
    releases: &mut Vec<ReleaseCandidate>,
    release_dir: PathBuf,
) {
    if let Some(existing) = releases.iter_mut().find(|r| r.path == release_dir)
    {
        existing.last_seen = Instant::now();
    } else {
        releases.push(ReleaseCandidate {
            path: release_dir,
            last_seen: Instant::now(),
        });
    }
}
