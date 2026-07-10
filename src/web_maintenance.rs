use crate::config::Config;
use anyhow::{Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

pub(crate) fn reset_history_files(history_dir: &str) -> Result<(usize, PathBuf)> {
    let history_path = Path::new(history_dir);
    fs::create_dir_all(history_path).with_context(|| {
        format!(
            "Konnte History-Ordner nicht erstellen: {}",
            history_path.display()
        )
    })?;

    let backup_path = backup_history_files(history_path)?;

    let mut removed = 0usize;

    for entry in fs::read_dir(history_path).with_context(|| {
        format!(
            "Konnte History-Ordner nicht lesen: {}",
            history_path.display()
        )
    })? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let is_marker = path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension == "done" || extension == "failed")
            .unwrap_or(false);

        if !is_marker {
            continue;
        }

        fs::remove_file(&path).with_context(|| {
            format!(
                "Verlaufsmarkierung konnte nicht gelöscht werden: {}",
                path.display()
            )
        })?;

        removed += 1;
    }

    Ok((removed, backup_path))
}

fn backup_history_files(history_path: &Path) -> Result<PathBuf> {
    let state_root = history_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("/state"));

    let backup_path = state_root
        .join("history-backups")
        .join(format!("history.bak.{}", unix_timestamp_secs()));

    fs::create_dir_all(&backup_path).with_context(|| {
        format!(
            "Konnte History-Backup-Ordner nicht erstellen: {}",
            backup_path.display()
        )
    })?;

    if history_path.exists() {
        for entry in fs::read_dir(history_path).with_context(|| {
            format!(
                "Konnte History-Ordner nicht lesen: {}",
                history_path.display()
            )
        })? {
            let entry = entry?;
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            let is_marker = path
                .extension()
                .and_then(|extension| extension.to_str())
                .map(|extension| extension == "done" || extension == "failed")
                .unwrap_or(false);

            if !is_marker {
                continue;
            }

            let Some(file_name) = path.file_name() else {
                continue;
            };

            fs::copy(&path, backup_path.join(file_name)).with_context(|| {
                format!(
                    "Verlaufsmarkierung konnte nicht gesichert werden: {}",
                    path.display()
                )
            })?;
        }
    }

    Ok(backup_path)
}

pub(crate) fn append_password_to_file(config: &Config, password: &str) -> Result<Option<PathBuf>> {
    let password = password.trim();

    if password.is_empty() {
        anyhow::bail!("Passwort darf nicht leer sein");
    }

    if password.contains('\n') || password.contains('\r') {
        anyhow::bail!("Ein einzelnes Passwort darf keinen Zeilenumbruch enthalten");
    }

    let path = password_file_path(config)?;
    ensure_password_path_is_safe(&path)?;

    let backup_path = backup_password_file(config, &path)?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "Konnte Passwortlisten-Ordner nicht erstellen: {}",
                parent.display()
            )
        })?;
    }

    let mut content = if path.exists() {
        fs::read_to_string(&path)
            .with_context(|| format!("Konnte Passwortliste nicht lesen: {}", path.display()))?
    } else {
        String::new()
    };

    let exists = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .any(|line| line == password);

    if exists {
        anyhow::bail!("Dieses Passwort ist bereits in der Liste");
    }

    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }

    content.push_str(password);
    content.push('\n');

    fs::write(&path, content)
        .with_context(|| format!("Konnte Passwortliste nicht schreiben: {}", path.display()))?;

    Ok(backup_path)
}

pub(crate) fn replace_password_file(config: &Config, passwords: &str) -> Result<usize> {
    let path = password_file_path(config)?;
    ensure_password_path_is_safe(&path)?;

    let cleaned = passwords
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| !line.starts_with('#'))
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    if cleaned.is_empty() {
        anyhow::bail!("Die neue Passwortliste darf nicht leer sein");
    }

    let _backup_path = backup_password_file(config, &path)?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "Konnte Passwortlisten-Ordner nicht erstellen: {}",
                parent.display()
            )
        })?;
    }

    let mut output = cleaned.join("\n");
    output.push('\n');

    fs::write(&path, output)
        .with_context(|| format!("Konnte Passwortliste nicht schreiben: {}", path.display()))?;

    Ok(cleaned.len())
}

fn password_file_path(config: &Config) -> Result<PathBuf> {
    if config.extract.password_file.trim().is_empty() {
        anyhow::bail!("Keine Passwortdatei konfiguriert");
    }

    Ok(PathBuf::from(&config.extract.password_file))
}

fn ensure_password_path_is_safe(path: &Path) -> Result<()> {
    if !path.is_absolute() {
        anyhow::bail!("Passwortdatei muss ein absoluter Pfad im Container sein");
    }

    if !path.starts_with("/config") {
        anyhow::bail!(
            "Aus Sicherheitsgründen kann die WebUI nur Passwortdateien unter /config bearbeiten"
        );
    }

    Ok(())
}

fn backup_password_file(config: &Config, password_path: &Path) -> Result<Option<PathBuf>> {
    let state_root = Path::new(&config.history.directory)
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("/state"));

    let backup_root = state_root.join("password-backups");

    fs::create_dir_all(&backup_root).with_context(|| {
        format!(
            "Konnte Passwort-Backup-Ordner nicht erstellen: {}",
            backup_root.display()
        )
    })?;

    if !password_path.exists() {
        return Ok(None);
    }

    let file_name = password_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("passwords.txt");

    let backup_path = backup_root.join(format!("{file_name}.bak.{}", unix_timestamp_secs()));

    fs::copy(password_path, &backup_path).with_context(|| {
        format!(
            "Konnte Passwortliste nicht sichern: {}",
            backup_path.display()
        )
    })?;

    Ok(Some(backup_path))
}

fn unix_timestamp_secs() -> u64 {
    SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}
