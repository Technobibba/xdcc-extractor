use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::{
    fs,
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

        let completed_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("Systemzeit liegt vor UNIX_EPOCH")?
            .as_secs();

        let content = format!(
            "status=done\nrelease={}\ncompleted_at_unix={}\n",
            release_dir.display(),
            completed_at
        );

        fs::write(&marker, content).with_context(|| {
            format!("Konnte History-Datei nicht schreiben: {}", marker.display())
        })?;

        Ok(())
    }

    pub fn marker_path(&self, release_dir: &Path) -> PathBuf {
        let release_name = release_dir
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let safe_name = sanitize_name(&release_name);
        let hash = short_hash(&release_dir.to_string_lossy());

        self.directory.join(format!("{}-{}.done", safe_name, hash))
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
