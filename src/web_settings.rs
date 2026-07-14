use crate::config::Config;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

#[derive(Debug, Deserialize)]
pub(crate) struct SettingsForm {
    pub(crate) watch_directories: String,
    pub(crate) stable_after: u64,
    pub(crate) allow_root_archives: Option<String>,
    pub(crate) delete_archives: Option<String>,
    pub(crate) keep_failed: Option<String>,
    pub(crate) retry_base_delay: u64,
    pub(crate) retry_max_delay: u64,
    pub(crate) startup_scan_existing: Option<String>,
    pub(crate) notifications_enabled: Option<String>,
    pub(crate) ntfy_server: String,
    pub(crate) ntfy_topic: String,
    pub(crate) ntfy_token: String,
    pub(crate) ntfy_priority_success: u8,
    pub(crate) ntfy_priority_error: u8,
    pub(crate) ntfy_notify_on_worker_start: Option<String>,
    pub(crate) ntfy_notify_on_processing_start: Option<String>,
    pub(crate) ntfy_notify_on_success: Option<String>,
    pub(crate) ntfy_notify_on_error: Option<String>,
    pub(crate) ntfy_notify_on_every_error: Option<String>,
    pub(crate) ntfy_notify_after_attempts: u64,
}

pub(crate) fn apply_settings_to_config_file(path: &Path, form: &SettingsForm) -> Result<PathBuf> {
    let watch_directories = parse_watch_directories(&form.watch_directories)?;

    if form.stable_after == 0 {
        anyhow::bail!("Die Wartezeit bis zur Verarbeitung muss größer als 0 sein");
    }

    if form.retry_base_delay == 0 {
        anyhow::bail!("Die erste Wiederholungs-Wartezeit muss größer als 0 sein");
    }

    if form.retry_max_delay < form.retry_base_delay {
        anyhow::bail!(
            "Die maximale Wiederholungs-Wartezeit muss mindestens so groß sein wie die erste Wartezeit"
        );
    }

    if form.ntfy_notify_after_attempts == 0 {
        anyhow::bail!("Die Anzahl der Versuche vor einer Fehlermeldung muss größer als 0 sein");
    }

    let mut content = fs::read_to_string(path).with_context(|| {
        format!(
            "Konfigurationsdatei konnte nicht gelesen werden: {}",
            path.display()
        )
    })?;

    content = set_toml_value(
        content,
        "watch",
        "directory",
        &toml_string(&watch_directories[0]),
    );

    content = set_toml_value(
        content,
        "watch",
        "directories",
        &toml_string_array(&watch_directories[1..]),
    );

    content = set_toml_value(
        content,
        "watch",
        "stable_after",
        &form.stable_after.to_string(),
    );
    content = set_toml_value(
        content,
        "watch",
        "allow_root_archives",
        toml_bool(form.allow_root_archives.is_some()),
    );

    content = set_toml_value(
        content,
        "extract",
        "delete_archives",
        toml_bool(form.delete_archives.is_some()),
    );
    content = set_toml_value(
        content,
        "extract",
        "keep_failed",
        toml_bool(form.keep_failed.is_some()),
    );

    content = set_toml_value(
        content,
        "retry",
        "base_delay",
        &form.retry_base_delay.to_string(),
    );
    content = set_toml_value(
        content,
        "retry",
        "max_delay",
        &form.retry_max_delay.to_string(),
    );

    content = set_toml_value(
        content,
        "startup",
        "scan_existing",
        toml_bool(form.startup_scan_existing.is_some()),
    );

    content = set_toml_value(
        content,
        "notifications",
        "enabled",
        toml_bool(form.notifications_enabled.is_some()),
    );
    content = set_toml_value(content, "notifications", "provider", &toml_string("ntfy"));

    if !form.ntfy_server.trim().is_empty() {
        content = set_toml_value(
            content,
            "notifications.ntfy",
            "server",
            &toml_string(form.ntfy_server.trim()),
        );
    }

    if !form.ntfy_topic.trim().is_empty() {
        content = set_toml_value(
            content,
            "notifications.ntfy",
            "topic",
            &toml_string(form.ntfy_topic.trim()),
        );
    }

    if !form.ntfy_token.trim().is_empty() {
        content = set_toml_value(
            content,
            "notifications.ntfy",
            "token",
            &toml_string(form.ntfy_token.trim()),
        );
    }

    content = set_toml_value(
        content,
        "notifications.ntfy",
        "priority_success",
        &form.ntfy_priority_success.to_string(),
    );
    content = set_toml_value(
        content,
        "notifications.ntfy",
        "priority_error",
        &form.ntfy_priority_error.to_string(),
    );
    content = set_toml_value(
        content,
        "notifications.ntfy",
        "notify_on_worker_start",
        toml_bool(form.ntfy_notify_on_worker_start.is_some()),
    );
    content = set_toml_value(
        content,
        "notifications.ntfy",
        "notify_on_processing_start",
        toml_bool(form.ntfy_notify_on_processing_start.is_some()),
    );
    content = set_toml_value(
        content,
        "notifications.ntfy",
        "notify_on_success",
        toml_bool(form.ntfy_notify_on_success.is_some()),
    );
    content = set_toml_value(
        content,
        "notifications.ntfy",
        "notify_on_error",
        toml_bool(form.ntfy_notify_on_error.is_some()),
    );
    content = set_toml_value(
        content,
        "notifications.ntfy",
        "notify_on_every_error",
        toml_bool(form.ntfy_notify_on_every_error.is_some()),
    );
    content = set_toml_value(
        content,
        "notifications.ntfy",
        "notify_after_attempts",
        &form.ntfy_notify_after_attempts.to_string(),
    );

    let _parsed: Config = toml::from_str(&content)
        .context("Geänderte Config ist ungültig und wurde nicht gespeichert")?;

    let backup_path = backup_config_file(path)?;

    fs::write(path, content).with_context(|| {
        format!(
            "Konfigurationsdatei konnte nicht geschrieben werden: {}",
            path.display()
        )
    })?;

    Ok(backup_path)
}

fn parse_watch_directories(value: &str) -> Result<Vec<String>> {
    let mut directories = Vec::<String>::new();

    for line in value.lines() {
        let directory = line.trim();

        if directory.is_empty() {
            continue;
        }

        if !Path::new(directory).is_absolute() {
            anyhow::bail!(
                "Watch-Ordner müssen absolute \
Pfade sein: {}",
                directory
            );
        }

        if directories.iter().any(|existing| existing == directory) {
            continue;
        }

        directories.push(directory.to_string());
    }

    if directories.is_empty() {
        anyhow::bail!(
            "Mindestens ein überwachter \
Ordner muss eingetragen sein"
        );
    }

    Ok(directories)
}

fn toml_string_array(values: &[String]) -> String {
    let values = values
        .iter()
        .map(|value| toml_string(value))
        .collect::<Vec<_>>()
        .join(", ");

    format!("[{values}]")
}

fn set_toml_value(content: String, section: &str, key: &str, value: &str) -> String {
    let mut lines = content.lines().map(str::to_string).collect::<Vec<_>>();
    let had_trailing_newline = content.ends_with('\n');

    let mut in_target = false;
    let mut section_found = false;
    let mut insert_at = None;

    for index in 0..lines.len() {
        let trimmed = lines[index].trim();

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            if in_target && insert_at.is_none() {
                insert_at = Some(index);
            }

            in_target = &trimmed[1..trimmed.len() - 1] == section;

            if in_target {
                section_found = true;
            }

            continue;
        }

        if !in_target {
            continue;
        }

        let without_comment = lines[index].split('#').next().unwrap_or("");
        let candidate = without_comment.trim_start();

        if !candidate.starts_with(key) {
            continue;
        }

        let rest = &candidate[key.len()..];

        if rest.trim_start().starts_with('=') {
            let indent_len = lines[index].len() - lines[index].trim_start().len();
            let indent = &lines[index][..indent_len];
            let comment = lines[index]
                .find('#')
                .map(|pos| format!(" {}", lines[index][pos..].trim_start()))
                .unwrap_or_default();

            lines[index] = format!("{indent}{key}={value}{comment}");
            return finish_toml_lines(lines, had_trailing_newline);
        }
    }

    if section_found {
        let index = insert_at.unwrap_or(lines.len());
        lines.insert(index, format!("{key}={value}"));
    } else {
        if !lines.is_empty() {
            lines.push(String::new());
        }

        lines.push(format!("[{section}]"));
        lines.push(format!("{key}={value}"));
    }

    finish_toml_lines(lines, had_trailing_newline)
}

fn finish_toml_lines(lines: Vec<String>, trailing_newline: bool) -> String {
    let mut output = lines.join("\n");

    if trailing_newline {
        output.push('\n');
    }

    output
}

fn backup_config_file(path: &Path) -> Result<PathBuf> {
    let current_config = Config::load(path)?;

    let backup_root = Path::new(&current_config.history.directory)
        .parent()
        .map(|parent| parent.join("config-backups"))
        .unwrap_or_else(|| PathBuf::from("/state/config-backups"));

    fs::create_dir_all(&backup_root).with_context(|| {
        format!(
            "Konnte Config-Backup-Ordner nicht erstellen: {}",
            backup_root.display()
        )
    })?;

    let timestamp = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("config.toml");

    let backup_path = backup_root.join(format!("{file_name}.bak.{timestamp}"));

    fs::copy(path, &backup_path).with_context(|| {
        format!(
            "Konnte Config-Backup nicht schreiben: {}",
            backup_path.display()
        )
    })?;

    Ok(backup_path)
}

fn toml_string(value: &str) -> String {
    format!(
        "\"{}\"",
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
    )
}

fn toml_bool(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}
