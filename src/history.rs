use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone)]
pub struct History {
    directory: PathBuf,
}

impl History {
    pub fn new(directory: impl Into<PathBuf>) -> Result<Self> {
        let directory = directory.into();

        fs::create_dir_all(&directory).with_context(|| {
            format!(
                "Konnte History-Ordner nicht erstellen: {}",
                directory.display()
            )
        })?;

        Ok(Self { directory })
    }

    pub fn is_done(&self, release_dir: &Path) -> bool {
        self.marker_path(release_dir).exists()
    }

    pub fn mark_done(&self, release_dir: &Path) -> Result<()> {
        let marker = self.marker_path(release_dir);

        let content = format!(
            "status=done\nrelease={}\ncompleted_at_unix={}\n",
            release_dir.display(),
            current_unix_timestamp()?
        );

        fs::write(&marker, content).with_context(|| {
            format!("Konnte History-Datei nicht schreiben: {}", marker.display())
        })?;

        self.clear_failed(release_dir)?;

        Ok(())
    }

    pub fn mark_failed(&self, release_dir: &Path, error: &str) -> Result<()> {
        let marker = self.failed_marker_path(release_dir);
        let attempts = self.failed_attempts(release_dir).unwrap_or(0) + 1;

        let clean_error = error.replace('\n', "\\n").replace('\r', "\\r");

        let content = format!(
            "status=failed\nrelease={}\nattempts={}\nlast_failed_at_unix={}\nerror={}\n",
            release_dir.display(),
            attempts,
            current_unix_timestamp()?,
            clean_error
        );

        fs::write(&marker, content).with_context(|| {
            format!(
                "Konnte Failed-History-Datei nicht schreiben: {}",
                marker.display()
            )
        })?;

        Ok(())
    }

    pub fn failed_attempts(&self, release_dir: &Path) -> Result<u64> {
        let marker = self.failed_marker_path(release_dir);

        let content = fs::read_to_string(&marker).with_context(|| {
            format!(
                "Konnte Failed-History-Datei nicht lesen: {}",
                marker.display()
            )
        })?;

        for line in content.lines() {
            if let Some(value) = line.strip_prefix("attempts=") {
                return value
                    .parse::<u64>()
                    .with_context(|| format!("Ungültiger attempts-Wert: {}", value));
            }
        }

        Ok(0)
    }

    pub fn clear_failed(&self, release_dir: &Path) -> Result<()> {
        let marker = self.failed_marker_path(release_dir);

        match fs::remove_file(&marker) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err).with_context(|| {
                format!(
                    "Konnte Failed-History-Datei nicht löschen: {}",
                    marker.display()
                )
            }),
        }
    }

    pub fn marker_path(&self, release_dir: &Path) -> PathBuf {
        self.marker_path_with_extension(release_dir, "done")
    }

    pub fn failed_marker_path(&self, release_dir: &Path) -> PathBuf {
        self.marker_path_with_extension(release_dir, "failed")
    }

    fn marker_path_with_extension(&self, release_dir: &Path, extension: &str) -> PathBuf {
        let release_name = release_dir
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let safe_name = sanitize_name(&release_name);
        let hash = short_hash(&release_dir.to_string_lossy());

        self.directory
            .join(format!("{}-{}.{}", safe_name, hash, extension))
    }
}

fn sanitize_name(name: &str) -> String {
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

fn short_hash(input: &str) -> String {
    let hash = Sha256::digest(input.as_bytes());
    format!("{:x}", hash)[0..12].to_string()
}

fn current_unix_timestamp() -> Result<u64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("Systemzeit liegt vor UNIX_EPOCH")?
        .as_secs())
}
