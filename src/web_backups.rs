use crate::config::Config;
use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

#[derive(Debug, Clone)]
pub(crate) struct BackupSummary {
    pub(crate) directory: PathBuf,
    pub(crate) directory_exists: bool,
    pub(crate) readable: bool,
    pub(crate) count: usize,
    pub(crate) latest_name: Option<String>,
    pub(crate) latest_age: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct BackupOverview {
    pub(crate) config: BackupSummary,
    pub(crate) history: BackupSummary,
    pub(crate) passwords: BackupSummary,
}

#[derive(Debug, Clone, Copy)]
enum BackupKind {
    Config,
    History,
    Passwords,
}

pub(crate) fn backup_overview(config: &Config) -> BackupOverview {
    let state_root = Path::new(&config.history.directory)
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("/state"));

    BackupOverview {
        config: summarize(state_root.join("config-backups"), BackupKind::Config),
        history: summarize(state_root.join("history-backups"), BackupKind::History),
        passwords: summarize(state_root.join("password-backups"), BackupKind::Passwords),
    }
}

fn summarize(directory: PathBuf, kind: BackupKind) -> BackupSummary {
    let directory_exists = directory.is_dir();

    if !directory_exists {
        return BackupSummary {
            directory,
            directory_exists: false,
            readable: false,
            count: 0,
            latest_name: None,
            latest_age: None,
        };
    }

    let entries = match fs::read_dir(&directory) {
        Ok(entries) => entries,
        Err(_) => {
            return BackupSummary {
                directory,
                directory_exists: true,
                readable: false,
                count: 0,
                latest_name: None,
                latest_age: None,
            };
        }
    };

    let mut count = 0;
    let mut latest: Option<(SystemTime, String)> = None;

    for entry in entries.flatten() {
        let path = entry.path();

        if !is_matching_backup(&path, kind) {
            continue;
        }

        count += 1;

        let Some(modified) = entry
            .metadata()
            .ok()
            .and_then(|metadata| metadata.modified().ok())
        else {
            continue;
        };

        let name = entry.file_name().to_string_lossy().into_owned();

        let replace_latest = latest
            .as_ref()
            .map(|(current, _)| modified > *current)
            .unwrap_or(true);

        if replace_latest {
            latest = Some((modified, name));
        }
    }

    let (latest_name, latest_age) = match latest {
        Some((modified, name)) => (Some(name), Some(relative_age(modified))),
        None => (None, None),
    };

    BackupSummary {
        directory,
        directory_exists: true,
        readable: true,
        count,
        latest_name,
        latest_age,
    }
}

fn is_matching_backup(path: &Path, kind: BackupKind) -> bool {
    let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };

    match kind {
        BackupKind::Config => path.is_file() && name.contains(".bak."),
        BackupKind::History => path.is_dir() && name.starts_with("history.bak."),
        BackupKind::Passwords => path.is_file() && name.contains(".bak."),
    }
}

fn relative_age(modified: SystemTime) -> String {
    let elapsed = match SystemTime::now().duration_since(modified) {
        Ok(duration) => duration,
        Err(_) => return "gerade eben".to_string(),
    };

    let seconds = elapsed.as_secs();

    if seconds < 60 {
        return "gerade eben".to_string();
    }

    let minutes = seconds / 60;

    if minutes < 60 {
        return ago(minutes, "Minute", "Minuten");
    }

    let hours = minutes / 60;

    if hours < 24 {
        return ago(hours, "Stunde", "Stunden");
    }

    let days = hours / 24;

    ago(days, "Tag", "Tagen")
}

fn ago(value: u64, singular: &str, plural: &str) -> String {
    let unit = if value == 1 { singular } else { plural };

    format!("vor {value} {unit}")
}
