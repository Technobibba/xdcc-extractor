use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct History {
    directory: PathBuf,
}

impl History {
    pub fn new<P: AsRef<Path>>(directory: P) -> Result<Self> {
        let directory = directory.as_ref();

        fs::create_dir_all(directory).with_context(|| {
            format!(
                "Konnte History-Ordner nicht erstellen: {}",
                directory.display()
            )
        })?;

        Ok(Self {
            directory: directory.to_path_buf(),
        })
    }

    pub fn is_done(&self, release: &Path) -> bool {
        self.marker_path(release).exists()
    }

    pub fn marker_path(&self, release: &Path) -> PathBuf {
        self.directory.join(format!(
            "{}-{}.done",
            marker_name(release),
            marker_hash(release)
        ))
    }

    pub fn failed_marker_path(&self, release: &Path) -> PathBuf {
        self.directory.join(format!(
            "{}-{}.failed",
            marker_name(release),
            marker_hash(release)
        ))
    }

    pub fn mark_done(&self, release: &Path) -> Result<()> {
        let marker = self.marker_path(release);

        fs::write(
            &marker,
            format!("release={}\nstatus=done\n", release.display()),
        )
        .with_context(|| format!("Konnte Done-Marker nicht schreiben: {}", marker.display()))?;

        let failed_marker = self.failed_marker_path(release);

        if failed_marker.exists() {
            fs::remove_file(&failed_marker).with_context(|| {
                format!(
                    "Konnte Failed-Marker nach Erfolg nicht löschen: {}",
                    failed_marker.display()
                )
            })?;
        }

        Ok(())
    }

    pub fn mark_failed(&self, release: &Path, error: &str) -> Result<()> {
        let attempts = self.failed_attempts(release)? + 1;
        let marker = self.failed_marker_path(release);

        fs::write(
            &marker,
            format!(
                "release={}\nstatus=failed\nattempts={}\nerror={}\n",
                release.display(),
                attempts,
                error
            ),
        )
        .with_context(|| {
            format!(
                "Konnte Failed-History-Datei nicht schreiben: {}",
                marker.display()
            )
        })?;

        Ok(())
    }

    pub fn clear_failed(&self, release: &Path) -> Result<bool> {
        let marker = self.failed_marker_path(release);

        if !marker.exists() {
            return Ok(false);
        }

        fs::remove_file(&marker)
            .with_context(|| format!("Konnte Failed-Marker nicht löschen: {}", marker.display()))?;

        Ok(true)
    }

    pub fn failed_attempts(&self, release: &Path) -> Result<u64> {
        let marker = self.failed_marker_path(release);

        if !marker.exists() {
            return Ok(0);
        }

        let content = fs::read_to_string(&marker).with_context(|| {
            format!(
                "Konnte Failed-History-Datei nicht lesen: {}",
                marker.display()
            )
        })?;

        Ok(parse_attempts(&content).unwrap_or(0))
    }
}

fn parse_attempts(content: &str) -> Option<u64> {
    for line in content.lines() {
        let trimmed = line.trim();

        if let Some(value) = trimmed.strip_prefix("attempts=") {
            return value.trim().parse().ok();
        }

        if let Some(value) = trimmed.strip_prefix("attempts:") {
            return value.trim().parse().ok();
        }

        if let Some(value) = trimmed.strip_prefix("Fehlversuche:") {
            return value.trim().parse().ok();
        }
    }

    None
}

fn marker_name(release: &Path) -> String {
    let name = release
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    sanitize_marker_name(&name)
}

fn marker_hash(release: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(release.to_string_lossy().as_bytes());

    let hash = hasher.finalize();
    let hex = format!("{:x}", hash);

    hex.chars().take(12).collect()
}

fn sanitize_marker_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn history_marks_release_as_done() {
        let dir = tempdir().expect("tempdir");
        let history = History::new(dir.path()).expect("history");

        let release = Path::new("/downloads/Test.Release");

        assert!(!history.is_done(release));

        history.mark_done(release).expect("mark done");

        assert!(history.is_done(release));
        assert!(history.marker_path(release).exists());
    }

    #[test]
    fn history_tracks_failed_attempts() {
        let dir = tempdir().expect("tempdir");
        let history = History::new(dir.path()).expect("history");

        let release = Path::new("/downloads/Broken.Release");

        history
            .mark_failed(release, "first error")
            .expect("first failed");

        assert_eq!(history.failed_attempts(release).expect("attempts"), 1);

        history
            .mark_failed(release, "second error")
            .expect("second failed");

        assert_eq!(history.failed_attempts(release).expect("attempts"), 2);
        assert!(history.failed_marker_path(release).exists());
    }

    #[test]
    fn mark_done_clears_failed_marker() {
        let dir = tempdir().expect("tempdir");
        let history = History::new(dir.path()).expect("history");

        let release = Path::new("/downloads/Recovered.Release");

        history
            .mark_failed(release, "temporary error")
            .expect("mark failed");

        assert!(history.failed_marker_path(release).exists());

        history.mark_done(release).expect("mark done");

        assert!(history.marker_path(release).exists());
        assert!(!history.failed_marker_path(release).exists());
    }
}

#[cfg(test)]
mod clear_failed_tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn clear_failed_removes_failed_marker() {
        let dir = tempdir().expect("tempdir");
        let history = History::new(dir.path()).expect("history");

        let release = Path::new("/downloads/Broken.Release.zip");

        history
            .mark_failed(release, "test error")
            .expect("mark failed");

        assert!(history.failed_marker_path(release).exists());

        let removed = history.clear_failed(release).expect("clear failed");

        assert!(removed);
        assert!(!history.failed_marker_path(release).exists());
        assert_eq!(history.failed_attempts(release).expect("attempts"), 0);
    }

    #[test]
    fn clear_failed_returns_false_when_marker_does_not_exist() {
        let dir = tempdir().expect("tempdir");
        let history = History::new(dir.path()).expect("history");

        let release = Path::new("/downloads/Not.Failed.Release.zip");

        let removed = history.clear_failed(release).expect("clear failed");

        assert!(!removed);
    }
}
