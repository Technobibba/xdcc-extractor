use anyhow::{Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

#[derive(Debug, Clone)]
pub(crate) struct FailureEntry {
    pub(crate) marker: PathBuf,
    pub(crate) path: String,
    pub(crate) attempts: u64,
    pub(crate) error_class: String,
    pub(crate) reason: String,
}

pub(crate) fn failure_entries(history_dir: &str, limit: usize) -> Result<Vec<FailureEntry>> {
    let path = Path::new(history_dir);

    if !path.is_dir() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();

    for entry in fs::read_dir(path)
        .with_context(|| format!("Konnte History-Ordner nicht lesen: {}", history_dir))?
    {
        let entry = entry?;
        let marker = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();

        if !file_name.ends_with(".failed") {
            continue;
        }

        let modified = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);

        let content = fs::read_to_string(&marker).unwrap_or_default();
        let parsed = parse_failed_marker(&content, &file_name);

        entries.push((
            modified,
            FailureEntry {
                marker,
                path: parsed.0,
                attempts: parsed.1,
                error_class: parsed.2,
                reason: parsed.3,
            },
        ));
    }

    entries.sort_by(|a, b| b.0.cmp(&a.0));

    Ok(entries
        .into_iter()
        .take(limit)
        .map(|(_, entry)| entry)
        .collect())
}

fn parse_failed_marker(content: &str, fallback_name: &str) -> (String, u64, String, String) {
    let mut release = String::new();
    let mut attempts = 0;
    let mut error_class = "failed".to_string();
    let mut reason = String::new();

    for line in content.lines() {
        let trimmed = line.trim();

        if let Some(value) = trimmed.strip_prefix("release=") {
            release = value.trim().to_string();
        }

        if let Some(value) = trimmed.strip_prefix("attempts=") {
            attempts = value.trim().parse().unwrap_or(0);
        }

        if let Some(value) = trimmed.strip_prefix("Fehlerklasse:") {
            error_class = value.trim().to_string();
        }

        if let Some(value) = trimmed.strip_prefix("Grund:") {
            reason = value.trim().to_string();
        }
    }

    if release.is_empty() {
        release = fallback_name.to_string();
    }

    if reason.is_empty() {
        reason = first_non_empty_error_line(content)
            .unwrap_or_else(|| "Kein Grund gefunden".to_string());
    }

    (release, attempts, error_class, reason)
}

fn first_non_empty_error_line(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with("release=")
            || trimmed.starts_with("status=")
            || trimmed.starts_with("attempts=")
            || trimmed.starts_with("error=")
        {
            continue;
        }

        let mut value = trimmed.to_string();

        if value.chars().count() > 180 {
            value = value.chars().take(180).collect();
            value.push_str("...");
        }

        return Some(value);
    }

    None
}

pub(crate) fn history_counts(history_dir: &str) -> (usize, usize) {
    let path = Path::new(history_dir);

    let Ok(entries) = fs::read_dir(path) else {
        return (0, 0);
    };

    let mut done = 0;
    let mut failed = 0;

    for entry in entries.flatten() {
        let file_name = entry.file_name().to_string_lossy().to_string();

        if file_name.ends_with(".done") {
            done += 1;
        }

        if file_name.ends_with(".failed") {
            failed += 1;
        }
    }

    (done, failed)
}
