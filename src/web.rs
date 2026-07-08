use crate::{config::Config, history::History, log_buffer, manual_process, scan};
use anyhow::{Context, Result};
use axum::{
    Json, Router,
    extract::{Form, Request, State},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use base64::Engine as _;
use serde::Deserialize;
use serde_json::json;
use std::{
    fs,
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
    time::SystemTime,
};
use tracing::{info, warn};

#[derive(Clone)]
struct AppState {
    config: Config,
    config_path: PathBuf,
}

#[derive(Debug, Deserialize)]
struct PathRequest {
    path: String,
}

#[derive(Debug, Deserialize)]
struct SettingsForm {
    stable_after: u64,
    allow_root_archives: Option<String>,
    delete_archives: Option<String>,
    dry_run: Option<String>,
    keep_failed: Option<String>,
    retry_base_delay: u64,
    retry_max_delay: u64,
    startup_scan_existing: Option<String>,
    gotify_enabled: Option<String>,
    gotify_url: String,
    gotify_token: String,
    gotify_priority_success: i32,
    gotify_priority_error: i32,
    gotify_notify_on_success: Option<String>,
    gotify_notify_on_error: Option<String>,
    gotify_notify_on_every_error: Option<String>,
    gotify_notify_after_attempts: u64,
}

#[derive(Debug, Deserialize)]
struct HistoryResetForm {
    confirm: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PasswordAddForm {
    password: String,
}

#[derive(Debug, Deserialize)]
struct PasswordReplaceForm {
    passwords: String,
    confirm: Option<String>,
}

pub fn start(config: Config, config_path: impl Into<PathBuf>) -> Result<()> {
    if !config.web.enabled {
        info!("WebUI deaktiviert");
        return Ok(());
    }

    let config_path = config_path.into();

    let bind = config.web.bind.clone();
    let addr: SocketAddr = bind
        .parse()
        .with_context(|| format!("Ungültige WebUI bind-Adresse: {}", bind))?;

    std::thread::spawn(move || {
        let runtime = match tokio::runtime::Runtime::new() {
            Ok(runtime) => runtime,
            Err(err) => {
                warn!("WebUI Runtime konnte nicht gestartet werden: {:?}", err);
                return;
            }
        };

        runtime.block_on(async move {
            let state = Arc::new(AppState {
                config,
                config_path,
            });

            let protected_routes = Router::new()
                .route("/", get(index))
                .route("/settings", get(settings))
                .route("/settings/edit", get(settings_edit).post(update_settings))
                .route("/settings/history/reset", post(settings_history_reset))
                .route("/settings/passwords/add", post(settings_password_add))
                .route(
                    "/settings/passwords/replace",
                    post(settings_password_replace),
                )
                .route("/logs", get(logs))
                .route("/api/status", get(api_status))
                .route("/api/config", get(api_config))
                .route("/api/scan", get(api_scan))
                .route("/api/failures", get(api_failures))
                .route("/api/logs", get(api_logs))
                .route("/api/clear-failed", post(api_clear_failed))
                .route("/api/process", post(api_process))
                .route("/api/restart", post(api_restart))
                .route("/assets/app.js", get(crate::web_assets::app_js))
                .route_layer(middleware::from_fn_with_state(state.clone(), require_auth));

            let app = Router::new()
                .route("/health", get(health))
                .merge(protected_routes)
                .with_state(state);

            let listener = match tokio::net::TcpListener::bind(addr).await {
                Ok(listener) => listener,
                Err(err) => {
                    warn!("WebUI konnte nicht auf {} starten: {:?}", addr, err);
                    return;
                }
            };

            info!("WebUI läuft auf http://{}", addr);

            if let Err(err) = axum::serve(listener, app).await {
                warn!("WebUI Serverfehler: {:?}", err);
            }
        });
    });

    Ok(())
}

async fn require_auth(
    State(_state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Response {
    let expected_password = match std::env::var("XDCC_WEB_AUTH_PASSWORD") {
        Ok(value) if !value.is_empty() => value,
        _ => {
            warn!("WebUI Auth ist nicht konfiguriert: XDCC_WEB_AUTH_PASSWORD fehlt");
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                "WebUI Auth ist nicht konfiguriert.",
            )
                .into_response();
        }
    };

    let expected_user = std::env::var("XDCC_WEB_AUTH_USER").unwrap_or_else(|_| "admin".to_string());

    let authorized = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Basic "))
        .and_then(|encoded| {
            base64::engine::general_purpose::STANDARD
                .decode(encoded)
                .ok()
        })
        .and_then(|decoded| String::from_utf8(decoded).ok())
        .and_then(|decoded| {
            decoded
                .split_once(':')
                .map(|(user, password)| (user.to_string(), password.to_string()))
        })
        .map(|(user, password)| user == expected_user && password == expected_password)
        .unwrap_or(false);

    if authorized {
        next.run(request).await
    } else {
        let mut response =
            (StatusCode::UNAUTHORIZED, "Authentifizierung erforderlich.").into_response();

        response.headers_mut().insert(
            header::WWW_AUTHENTICATE,
            header::HeaderValue::from_static(r#"Basic realm="XDCC Extractor""#),
        );

        response
    }
}

async fn health() -> &'static str {
    "ok"
}

async fn settings_edit(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let config = match Config::load(&state.config_path) {
        Ok(config) => config,
        Err(err) => {
            warn!(
                "Konnte Config für Settings-Editor nicht frisch laden: {:?}",
                err
            );
            state.config.clone()
        }
    };

    Html(crate::web_pages::settings_edit_page_html(
        &config,
        &state.config_path,
        None,
    ))
}

async fn update_settings(
    State(state): State<Arc<AppState>>,
    Form(form): Form<SettingsForm>,
) -> impl IntoResponse {
    let message = match apply_settings_to_config_file(&state.config_path, &form) {
        Ok(backup_path) => {
            info!(
                "WebUI Einstellungen gespeichert: {} Backup: {}",
                state.config_path.display(),
                backup_path.display()
            );
            "Gespeichert. Backup wurde erstellt. Neustart erforderlich, damit der Worker die neuen Werte übernimmt."
                .to_string()
        }
        Err(err) => format!("Speichern fehlgeschlagen: {err:?}"),
    };

    let config = match Config::load(&state.config_path) {
        Ok(config) => config,
        Err(err) => {
            warn!("Konnte Config nach Settings-Update nicht laden: {:?}", err);
            state.config.clone()
        }
    };

    Html(crate::web_pages::settings_edit_page_html(
        &config,
        &state.config_path,
        Some(&message),
    ))
}

fn apply_settings_to_config_file(path: &Path, form: &SettingsForm) -> Result<PathBuf> {
    if form.stable_after == 0 {
        anyhow::bail!("stable_after muss größer als 0 sein");
    }

    if form.retry_base_delay == 0 {
        anyhow::bail!("retry.base_delay muss größer als 0 sein");
    }

    if form.retry_max_delay < form.retry_base_delay {
        anyhow::bail!("retry.max_delay muss größer oder gleich retry.base_delay sein");
    }

    if form.gotify_notify_after_attempts == 0 {
        anyhow::bail!("notify_after_attempts muss größer als 0 sein");
    }

    let mut content = fs::read_to_string(path)
        .with_context(|| format!("Konnte Config nicht lesen: {}", path.display()))?;

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
        "dry_run",
        toml_bool(form.dry_run.is_some()),
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
        "notifications.gotify",
        "enabled",
        toml_bool(form.gotify_enabled.is_some()),
    );
    if !form.gotify_url.trim().is_empty() {
        content = set_toml_value(
            content,
            "notifications.gotify",
            "url",
            &toml_string(form.gotify_url.trim()),
        );
    }

    if !form.gotify_token.trim().is_empty() {
        content = set_toml_value(
            content,
            "notifications.gotify",
            "token",
            &toml_string(form.gotify_token.trim()),
        );
    }

    content = set_toml_value(
        content,
        "notifications.gotify",
        "priority_success",
        &form.gotify_priority_success.to_string(),
    );
    content = set_toml_value(
        content,
        "notifications.gotify",
        "priority_error",
        &form.gotify_priority_error.to_string(),
    );
    content = set_toml_value(
        content,
        "notifications.gotify",
        "notify_on_success",
        toml_bool(form.gotify_notify_on_success.is_some()),
    );
    content = set_toml_value(
        content,
        "notifications.gotify",
        "notify_on_error",
        toml_bool(form.gotify_notify_on_error.is_some()),
    );
    content = set_toml_value(
        content,
        "notifications.gotify",
        "notify_on_every_error",
        toml_bool(form.gotify_notify_on_every_error.is_some()),
    );
    content = set_toml_value(
        content,
        "notifications.gotify",
        "notify_after_attempts",
        &form.gotify_notify_after_attempts.to_string(),
    );

    let _parsed: Config = toml::from_str(&content)
        .context("Geänderte Config ist ungültig und wurde nicht gespeichert")?;

    let backup_path = backup_config_file(path)?;

    fs::write(path, content)
        .with_context(|| format!("Konnte Config nicht schreiben: {}", path.display()))?;

    Ok(backup_path)
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

async fn settings_history_reset(
    State(state): State<Arc<AppState>>,
    Form(form): Form<HistoryResetForm>,
) -> impl IntoResponse {
    let config = current_config_for_page(&state);

    let message = match form.confirm.as_deref() {
        Some("RESET") => match reset_history_files(&config.history.directory) {
            Ok((removed, backup_path)) => {
                info!(
                    "History über WebUI zurückgesetzt: {} Marker entfernt, Backup: {}",
                    removed,
                    backup_path.display()
                );

                format!(
                    "History wurde zurückgesetzt. {} Marker entfernt. Backup wurde erstellt. Neustart empfohlen.",
                    removed
                )
            }
            Err(err) => format!("History konnte nicht zurückgesetzt werden: {err:?}"),
        },
        _ => "History wurde nicht gelöscht. Bitte Bestätigung aktivieren.".to_string(),
    };

    let config = current_config_for_page(&state);
    Html(crate::web_pages::settings_edit_page_html(
        &config,
        &state.config_path,
        Some(&message),
    ))
}

async fn settings_password_add(
    State(state): State<Arc<AppState>>,
    Form(form): Form<PasswordAddForm>,
) -> impl IntoResponse {
    let config = current_config_for_page(&state);

    let message = match append_password_to_file(&config, &form.password) {
        Ok(_) => {
            info!("Passwortliste über WebUI erweitert.");
            "Passwort wurde hinzugefügt. Backup wurde erstellt. Neustart erforderlich, damit der Worker die neue Passwortliste lädt.".to_string()
        }
        Err(err) => format!("Passwort konnte nicht hinzugefügt werden: {err:?}"),
    };

    let config = current_config_for_page(&state);
    Html(crate::web_pages::settings_edit_page_html(
        &config,
        &state.config_path,
        Some(&message),
    ))
}

async fn settings_password_replace(
    State(state): State<Arc<AppState>>,
    Form(form): Form<PasswordReplaceForm>,
) -> impl IntoResponse {
    let config = current_config_for_page(&state);

    let message = match form.confirm.as_deref() {
        Some("REPLACE") => match replace_password_file(&config, &form.passwords) {
            Ok(count) => {
                info!("Passwortliste über WebUI ersetzt: {} Einträge.", count);
                format!(
                    "Passwortliste wurde ersetzt. {} Passwort/Passwörter gespeichert. Backup wurde erstellt. Neustart erforderlich.",
                    count
                )
            }
            Err(err) => format!("Passwortliste konnte nicht ersetzt werden: {err:?}"),
        },
        _ => "Passwortliste wurde nicht ersetzt. Bitte Bestätigung aktivieren.".to_string(),
    };

    let config = current_config_for_page(&state);
    Html(crate::web_pages::settings_edit_page_html(
        &config,
        &state.config_path,
        Some(&message),
    ))
}

fn current_config_for_page(state: &AppState) -> Config {
    match Config::load(&state.config_path) {
        Ok(config) => config,
        Err(err) => {
            warn!("Konnte Config für WebUI nicht frisch laden: {:?}", err);
            state.config.clone()
        }
    }
}

fn reset_history_files(history_dir: &str) -> Result<(usize, PathBuf)> {
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

        fs::remove_file(&path)
            .with_context(|| format!("Konnte History-Marker nicht löschen: {}", path.display()))?;

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
                format!("Konnte History-Marker nicht sichern: {}", path.display())
            })?;
        }
    }

    Ok(backup_path)
}

fn append_password_to_file(config: &Config, password: &str) -> Result<Option<PathBuf>> {
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

fn replace_password_file(config: &Config, passwords: &str) -> Result<usize> {
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

async fn logs() -> Html<String> {
    Html(crate::web_pages::logs_page_html())
}

async fn api_logs() -> Json<serde_json::Value> {
    Json(json!({
        "lines": log_buffer::recent(300),
        "limit": 300
    }))
}

async fn api_config(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(json!({
        "ok": true,
        "version": env!("CARGO_PKG_VERSION"),
        "watch": {
            "directory": state.config.watch.directory,
            "stable_after": state.config.watch.stable_after,
            "allow_root_archives": state.config.watch.allow_root_archives,
        },
        "extract": {
            "delete_archives": state.config.extract.delete_archives,
            "dry_run": state.config.extract.dry_run,
            "keep_failed": state.config.extract.keep_failed,
            "password_file_configured": !state.config.extract.password_file.trim().is_empty(),
        },
        "output": {
            "directory": state.config.output.directory,
        },
        "history": {
            "directory": state.config.history.directory,
        },
        "retry": {
            "base_delay": state.config.retry.base_delay,
            "max_delay": state.config.retry.max_delay,
        },
        "startup": {
            "scan_existing": state.config.startup.scan_existing,
        },
        "notifications": {
            "gotify": {
                "enabled": state.config.notifications.gotify.enabled,
                "url_configured": !state.config.notifications.gotify.url.trim().is_empty(),
                "token_configured": !state.config.notifications.gotify.token.trim().is_empty(),
                "priority_success": state.config.notifications.gotify.priority_success,
                "priority_error": state.config.notifications.gotify.priority_error,
                "notify_on_success": state.config.notifications.gotify.notify_on_success,
                "notify_on_error": state.config.notifications.gotify.notify_on_error,
                "notify_on_every_error": state.config.notifications.gotify.notify_on_every_error,
                "notify_after_attempts": state.config.notifications.gotify.notify_after_attempts,
            }
        },
        "web": {
            "enabled": state.config.web.enabled,
            "bind": state.config.web.bind,
        },
        "secrets": {
            "gotify_token_visible": false,
            "password_file_content_visible": false,
        }
    }))
}

async fn api_status(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let history = history_counts(&state.config.history.directory);

    Json(json!({
        "version": env!("CARGO_PKG_VERSION"),
        "watch_directory": state.config.watch.directory,
        "output_directory": state.config.output.directory,
        "history_directory": state.config.history.directory,
        "dry_run": state.config.extract.dry_run,
        "delete_archives": state.config.extract.delete_archives,
        "keep_failed": state.config.extract.keep_failed,
        "allow_root_archives": state.config.watch.allow_root_archives,
        "gotify_enabled": state.config.notifications.gotify.enabled,
        "web_enabled": state.config.web.enabled,
        "web_bind": state.config.web.bind,
        "history_done": history.0,
        "history_failed": history.1,
    }))
}

async fn api_scan(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    match scan::scan_candidates_with_history(&state.config) {
        Ok(candidates) => {
            let items = candidates
                .iter()
                .map(|candidate| {
                    json!({
                        "path": candidate.path.display().to_string(),
                        "state": candidate.state.label(),
                    })
                })
                .collect::<Vec<_>>();

            Json(json!({
                "ok": true,
                "count": items.len(),
                "candidates": items,
            }))
        }
        Err(err) => Json(json!({
            "ok": false,
            "error": format!("{:?}", err),
        })),
    }
}

async fn api_failures(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    match failure_entries(&state.config.history.directory, 25) {
        Ok(entries) => {
            let items = entries
                .iter()
                .map(|entry| {
                    json!({
                        "marker": entry.marker.display().to_string(),
                        "path": entry.path,
                        "attempts": entry.attempts,
                        "error_class": entry.error_class,
                        "reason": entry.reason,
                    })
                })
                .collect::<Vec<_>>();

            Json(json!({
                "ok": true,
                "count": items.len(),
                "failures": items,
            }))
        }
        Err(err) => Json(json!({
            "ok": false,
            "error": format!("{:?}", err),
        })),
    }
}

async fn api_clear_failed(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PathRequest>,
) -> Json<serde_json::Value> {
    let path = Path::new(&payload.path);

    let history = match History::new(&state.config.history.directory) {
        Ok(history) => history,
        Err(err) => {
            return Json(json!({
                "ok": false,
                "error": format!("{:?}", err),
            }));
        }
    };

    match history.clear_failed(path) {
        Ok(removed) => Json(json!({
            "ok": true,
            "removed": removed,
            "path": payload.path,
        })),
        Err(err) => Json(json!({
            "ok": false,
            "error": format!("{:?}", err),
        })),
    }
}

async fn api_process(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PathRequest>,
) -> Json<serde_json::Value> {
    let path = Path::new(&payload.path);

    match manual_process::run_process("web", &state.config, path) {
        Ok(()) => Json(json!({
            "ok": true,
            "path": payload.path,
        })),
        Err(err) => Json(json!({
            "ok": false,
            "error": format!("{:?}", err),
        })),
    }
}

async fn api_restart() -> Json<serde_json::Value> {
    info!("WebUI Neustart wurde angefordert.");

    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_millis(900));
        std::process::exit(0);
    });

    Json(json!({
        "ok": true,
        "message": "Neustart wird ausgelöst"
    }))
}

async fn settings(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let gotify_enabled = if state.config.notifications.gotify.enabled {
        r#"<span class="badge ok">aktiv</span>"#
    } else {
        r#"<span class="badge muted">aus</span>"#
    };

    let dry_run = if state.config.extract.dry_run {
        r#"<span class="badge ok">aktiv</span>"#
    } else {
        r#"<span class="badge warn">aus</span>"#
    };

    let delete_archives = if state.config.extract.delete_archives {
        r#"<span class="badge warn">aktiv</span>"#
    } else {
        r#"<span class="badge muted">aus</span>"#
    };

    let keep_failed = if state.config.extract.keep_failed {
        r#"<span class="badge ok">aktiv</span>"#
    } else {
        r#"<span class="badge muted">aus</span>"#
    };

    let startup_scan = if state.config.startup.scan_existing {
        r#"<span class="badge warn">aktiv</span>"#
    } else {
        r#"<span class="badge muted">aus</span>"#
    };

    let allow_root_archives = if state.config.watch.allow_root_archives {
        r#"<span class="badge ok">aktiv</span>"#
    } else {
        r#"<span class="badge muted">aus</span>"#
    };

    let notify_on_success = if state.config.notifications.gotify.notify_on_success {
        r#"<span class="badge ok">aktiv</span>"#
    } else {
        r#"<span class="badge muted">aus</span>"#
    };

    let notify_on_error = if state.config.notifications.gotify.notify_on_error {
        r#"<span class="badge ok">aktiv</span>"#
    } else {
        r#"<span class="badge muted">aus</span>"#
    };

    let notify_on_every_error = if state.config.notifications.gotify.notify_on_every_error {
        r#"<span class="badge warn">aktiv</span>"#
    } else {
        r#"<span class="badge muted">aus</span>"#
    };

    let web_enabled = if state.config.web.enabled {
        r#"<span class="badge ok">aktiv</span>"#
    } else {
        r#"<span class="badge muted">aus</span>"#
    };

    let gotify_url_configured = if state.config.notifications.gotify.url.trim().is_empty() {
        r#"<span class="badge bad">nein</span>"#
    } else {
        r#"<span class="badge ok">ja</span>"#
    };

    let token_configured = if state.config.notifications.gotify.token.trim().is_empty() {
        r#"<span class="badge bad">nein</span>"#
    } else {
        r#"<span class="badge ok">ja</span>"#
    };

    let password_file_configured = if state.config.extract.password_file.trim().is_empty() {
        r#"<span class="badge muted">nein</span>"#
    } else {
        r#"<span class="badge ok">ja</span>"#
    };

    let html = format!(
        r#"<!doctype html>
<html lang="de">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>XDCC Extractor Einstellungen</title>
<style>
:root {{
  color-scheme: dark;
  --bg: #0f1115;
  --panel: #171a21;
  --text: #e6e6e6;
  --muted: #9aa4b2;
  --border: #2a2f3a;
  --ok: #25c26e;
  --warn: #f0a020;
  --bad: #ff5c5c;
}}
body {{
  margin: 0;
  font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  background: var(--bg);
  color: var(--text);
}}
main {{
  max-width: 1100px;
  margin: 0 auto;
  padding: 32px 20px;
}}
h1 {{
  margin: 0 0 6px;
  font-size: 32px;
}}
.sub {{
  color: var(--muted);
  margin-bottom: 28px;
}}
.grid {{
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 14px;
}}
.card {{
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 14px;
  padding: 18px;
}}
.card.wide {{
  grid-column: 1 / -1;
}}
.card h2 {{
  margin: 0 0 12px;
  font-size: 16px;
  color: var(--muted);
  font-weight: 600;
}}
.row {{
  display: grid;
  grid-template-columns: 180px 1fr;
  gap: 12px;
  padding: 8px 0;
  border-top: 1px solid var(--border);
}}
.row:first-of-type {{
  border-top: 0;
}}
.key {{
  color: var(--muted);
  font-size: 14px;
}}
.value {{
  font-size: 14px;
  word-break: break-word;
}}
.badge {{
  display: inline-block;
  padding: 5px 10px;
  border-radius: 999px;
  font-weight: 700;
  font-size: 14px;
}}
.badge.ok {{
  background: rgba(37, 194, 110, .15);
  color: var(--ok);
}}
.badge.warn {{
  background: rgba(240, 160, 32, .15);
  color: var(--warn);
}}
.badge.bad {{
  background: rgba(255, 92, 92, .15);
  color: var(--bad);
}}
.badge.muted {{
  background: rgba(154, 164, 178, .12);
  color: var(--muted);
}}
.actions {{
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  margin-bottom: 18px;
}}
.actions.nav {{
  margin-top: 22px;
  margin-bottom: 28px;
}}
.button {{
  display: inline-block;
  padding: 10px 14px;
  border-radius: 10px;
  border: 1px solid var(--border);
  background: #222735;
  color: var(--text);
  text-decoration: none;
  font-weight: 700;
  font-size: 14px;
}}
.button:hover {{
  background: #2b3142;
}}
code {{
  color: #cfd7e6;
}}
footer {{
  margin-top: 28px;
  color: var(--muted);
  font-size: 13px;
}}
@media (max-width: 720px) {{
  .row {{
    grid-template-columns: 1fr;
    gap: 4px;
  }}
}}
</style>
</head>
<body>
<main>
  <h1>Einstellungen</h1>
  <div class="sub">XDCC Extractor Version {version}</div>

    <div class="actions nav" style="margin-top: 22px; margin-bottom: 30px;">
    <a class="button" href="/">Dashboard</a>
    <a class="button" href="/settings">Einstellungen</a>
    <a class="button" href="/settings/edit">Bearbeiten</a>
    <a class="button" href="/logs">Logs</a>
  </div>

  <div class="grid">
    <section class="card">
      <h2>Watch</h2>
      <div class="row"><div class="key">Überwachter Ordner</div><div class="value"><code>{watch_dir}</code></div></div>
      <div class="row"><div class="key">Wartezeit bis Verarbeitung</div><div class="value">{stable_after}s</div></div>
      <div class="row"><div class="key">Root-Archive erlauben</div><div class="value">{allow_root_archives}</div></div>
    </section>

    <section class="card">
      <h2>Extract</h2>
      <div class="row"><div class="key">Testmodus</div><div class="value">{dry_run}</div></div>
      <div class="row"><div class="key">Archive nach Erfolg löschen</div><div class="value">{delete_archives}</div></div>
      <div class="row"><div class="key">Fehlerhafte Archive behalten</div><div class="value">{keep_failed}</div></div>
      <div class="row"><div class="key">Passwortliste konfiguriert</div><div class="value">{password_file_configured} <span class="key">Inhalt wird nicht angezeigt</span></div></div>
      <div class="row"><div class="key">Pfad zur Passwortliste</div><div class="value"><code>{password_file}</code></div></div>
    </section>

    <section class="card">
      <h2>Output / History</h2>
      <div class="row"><div class="key">Ausgabeordner</div><div class="value"><code>{output_dir}</code></div></div>
      <div class="row"><div class="key">History-Ordner</div><div class="value"><code>{history_dir}</code></div></div>
    </section>

    <section class="card">
      <h2>Retry / Startup</h2>
      <div class="row"><div class="key">Erste Wiederholung nach</div><div class="value">{base_delay}s</div></div>
      <div class="row"><div class="key">Maximale Wiederholungs-Wartezeit</div><div class="value">{max_delay}s</div></div>
      <div class="row"><div class="key">Vorhandene Releases beim Start scannen</div><div class="value">{startup_scan}</div></div>
    </section>

    <section class="card">
      <h2>Gotify</h2>
      <div class="row"><div class="key">Gotify aktiv</div><div class="value">{gotify_enabled}</div></div>
      <div class="row"><div class="key">Gotify URL konfiguriert</div><div class="value">{gotify_url_configured}</div></div>
      <div class="row"><div class="key">Token konfiguriert</div><div class="value">{token_configured}</div></div>
      <div class="row"><div class="key">Priorität bei Erfolg</div><div class="value">{priority_success}</div></div>
      <div class="row"><div class="key">Priorität bei Fehler</div><div class="value">{priority_error}</div></div>
      <div class="row"><div class="key">Erfolg melden</div><div class="value">{notify_on_success}</div></div>
      <div class="row"><div class="key">Fehler melden</div><div class="value">{notify_on_error}</div></div>
      <div class="row"><div class="key">Jeden Fehler melden</div><div class="value">{notify_on_every_error}</div></div>
      <div class="row"><div class="key">Fehler melden nach Versuchen</div><div class="value">{notify_after_attempts}</div></div>
    </section>

    <section class="card">
      <h2>WebUI</h2>
      <div class="row"><div class="key">WebUI aktiv</div><div class="value">{web_enabled}</div></div>
      <div class="row"><div class="key">Adresse / Port</div><div class="value"><code>{web_bind}</code></div></div>
    </section>

    <section class="card wide">
      <h2>Secrets</h2>
      <div class="row"><div class="key">Gotify Token</div><div class="value"><span class="badge muted">nicht sichtbar</span></div></div>
      <div class="row"><div class="key">Passwortliste</div><div class="value"><span class="badge muted">Inhalt nicht sichtbar</span></div></div>
    </section>
  </div>

  <footer>
    Diese Seite ist read-only. Config-Schreibfunktionen bauen wir später bewusst separat.
  </footer>
</main>
</body>
</html>"#,
        version = env!("CARGO_PKG_VERSION"),
        watch_dir = escape(&state.config.watch.directory),
        stable_after = state.config.watch.stable_after,
        allow_root_archives = allow_root_archives,
        dry_run = dry_run,
        delete_archives = delete_archives,
        keep_failed = keep_failed,
        password_file_configured = password_file_configured,
        password_file = escape(&state.config.extract.password_file),
        output_dir = escape(&state.config.output.directory),
        history_dir = escape(&state.config.history.directory),
        base_delay = state.config.retry.base_delay,
        max_delay = state.config.retry.max_delay,
        startup_scan = startup_scan,
        gotify_enabled = gotify_enabled,
        token_configured = token_configured,
        priority_success = state.config.notifications.gotify.priority_success,
        priority_error = state.config.notifications.gotify.priority_error,
        notify_on_success = notify_on_success,
        notify_on_error = notify_on_error,
        notify_on_every_error = notify_on_every_error,
        notify_after_attempts = state.config.notifications.gotify.notify_after_attempts,
        web_enabled = web_enabled,
        web_bind = escape(&state.config.web.bind),
    );

    Html(html)
}

async fn index(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let history = history_counts(&state.config.history.directory);
    let scan_html = scan_summary_html(&state.config);
    let failures_html = failures_html(&state.config);

    let dry_run_badge = if state.config.extract.dry_run {
        r#"<span class="badge ok">aktiv</span>"#
    } else {
        r#"<span class="badge warn">aus</span>"#
    };

    let gotify_badge = if state.config.notifications.gotify.enabled {
        r#"<span class="badge ok">aktiv</span>"#
    } else {
        r#"<span class="badge muted">aus</span>"#
    };

    let html = format!(
        r#"<!doctype html>
<html lang="de">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>XDCC Extractor</title>
<style>
:root {{
  color-scheme: dark;
  --bg: #0f1115;
  --panel: #171a21;
  --text: #e6e6e6;
  --muted: #9aa4b2;
  --border: #2a2f3a;
  --ok: #25c26e;
  --warn: #f0a020;
  --bad: #ff5c5c;
}}
body {{
  margin: 0;
  font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  background: var(--bg);
  color: var(--text);
}}
main {{
  max-width: 1100px;
  margin: 0 auto;
  padding: 32px 20px;
}}
h1 {{
  margin: 0 0 6px;
  font-size: 32px;
}}
.sub {{
  color: var(--muted);
  margin-bottom: 28px;
}}
.grid {{
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(230px, 1fr));
  gap: 14px;
}}
.card {{
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 14px;
  padding: 18px;
}}
.card.wide {{
  grid-column: 1 / -1;
}}
.card h2 {{
  margin: 0 0 12px;
  font-size: 16px;
  color: var(--muted);
  font-weight: 600;
}}
.value {{
  font-size: 22px;
  font-weight: 700;
  word-break: break-word;
}}
.small {{
  font-size: 14px;
  color: var(--muted);
  word-break: break-word;
}}
.badge {{
  display: inline-block;
  padding: 5px 10px;
  border-radius: 999px;
  font-weight: 700;
  font-size: 14px;
}}
.badge.ok {{
  background: rgba(37, 194, 110, .15);
  color: var(--ok);
}}
.badge.warn {{
  background: rgba(240, 160, 32, .15);
  color: var(--warn);
}}
.badge.bad {{
  background: rgba(255, 92, 92, .15);
  color: var(--bad);
}}
.badge.muted {{
  background: rgba(154, 164, 178, .12);
  color: var(--muted);
}}
.scan-summary {{
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
  margin-bottom: 12px;
}}
.scan-list {{
  display: grid;
  gap: 8px;
}}
.scan-row {{
  display: grid;
  grid-template-columns: 88px 1fr auto;
  gap: 10px;
  align-items: center;
  padding: 8px 0;
  border-top: 1px solid var(--border);
}}
.scan-path {{
  color: var(--text);
  font-size: 14px;
  word-break: break-all;
}}
.scan-actions {{
  text-align: right;
}}
.failure-list {{
  display: grid;
  gap: 12px;
}}
.failure-row {{
  display: grid;
  gap: 8px;
  padding: 12px 0;
  border-top: 1px solid var(--border);
}}
.error {{
  color: var(--bad);
}}
.actions {{
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}}
.button {{
  display: inline-block;
  padding: 10px 14px;
  border-radius: 10px;
  border: 1px solid var(--border);
  background: #222735;
  color: var(--text);
  text-decoration: none;
  font-weight: 700;
  font-size: 14px;
}}
.button:hover {{
  background: #2b3142;
}}
button.button {{
  cursor: pointer;
  font-family: inherit;
}}
button.button:disabled {{
  opacity: .65;
  cursor: wait;
}}
.small-button {{
  padding: 7px 10px;
  font-size: 13px;
}}
.danger-button {{
  border-color: rgba(255, 92, 92, .35);
}}
.card-head {{
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
}}
.card-head h2 {{
  margin: 0;
}}
footer {{
  margin-top: 28px;
  color: var(--muted);
  font-size: 13px;
}}
code {{
  color: #cfd7e6;
}}
@media (max-width: 720px) {{
  .scan-row {{
    grid-template-columns: 1fr;
  }}
  .scan-actions {{
    text-align: left;
  }}
}}
</style>
</head>
<body>
<main>
  <h1>XDCC Extractor</h1>
  <div class="sub">Version {version}</div>

  <div class="actions nav" style="margin-top: 22px; margin-bottom: 30px;">
    <a class="button" href="/">Dashboard</a>
    <a class="button" href="/settings">Einstellungen</a>
    <a class="button" href="/settings/edit">Bearbeiten</a>
    <a class="button" href="/logs">Logs</a>
  </div>

  <div class="grid">
    <section class="card">
      <h2>Worker</h2>
      <div class="value">läuft</div>
      <div class="small">WebUI erreichbar</div>
    </section>

    <section class="card">
      <h2>Dry Run</h2>
      <div class="value">{dry_run_badge}</div>
      <div class="small">delete_archives: {delete_archives}</div>
    </section>

    <section class="card">
      <h2>Gotify</h2>
      <div class="value">{gotify_badge}</div>
      <div class="small">Token wird nicht angezeigt</div>
    </section>

    <section class="card">
      <h2>History</h2>
      <div class="value">{done} done / {failed} failed</div>
      <div class="small"><code>{history_dir}</code></div>
    </section>

    <section class="card">
      <h2>Watch-Ordner</h2>
      <div class="small"><code>{watch_dir}</code></div>
    </section>

    <section class="card">
      <h2>Output-Ordner</h2>
      <div class="small"><code>{output_dir}</code></div>
    </section>

    <section class="card">
      <h2>Root-Archive</h2>
      <div class="value">{allow_root_archives}</div>
      <div class="small">Botarr/XDCC Flat Downloads</div>
    </section>

    <section class="card">
      <h2>System</h2>
      <div class="value">bereit</div>
      <div class="small">WebUI geschützt</div>
      <div class="small">Healthcheck aktiv</div>
      <div class="small">Version {version}</div>
    </section>

    <section class="card wide">
      <div class="card-head">
        <h2>Scan</h2>
        <button id="refresh-scan" class="button" type="button">Scan aktualisieren</button>
      </div>
      <div id="scan-content">
        {scan_html}
      </div>
    </section>

    <section class="card wide">
      <div class="card-head">
        <h2>Letzte Fehler</h2>
        <button id="refresh-failures" class="button" type="button">Fehler aktualisieren</button>
      </div>
      <div id="failure-content">
        {failures_html}
      </div>
    </section>
  </div>

  <footer>
    Dashboard mit manuellen Aktionen. Verarbeitung respektiert dry_run, delete_archives, History und Gotify.
  </footer>
</main>
<script src="/assets/app.js"></script>
</body>
</html>"#,
        version = env!("CARGO_PKG_VERSION"),
        dry_run_badge = dry_run_badge,
        delete_archives = state.config.extract.delete_archives,
        gotify_badge = gotify_badge,
        done = history.0,
        failed = history.1,
        history_dir = escape(&state.config.history.directory),
        watch_dir = escape(&state.config.watch.directory),
        output_dir = escape(&state.config.output.directory),
        allow_root_archives = state.config.watch.allow_root_archives,
        scan_html = scan_html,
        failures_html = failures_html,
    );

    Html(html)
}

fn scan_summary_html(config: &Config) -> String {
    let candidates = match scan::scan_candidates_with_history(config) {
        Ok(candidates) => candidates,
        Err(err) => {
            return format!(
                r#"<div class="error">Scan-Fehler: {}</div>"#,
                escape(&format!("{:?}", err))
            );
        }
    };

    let mut new_count = 0;
    let mut done_count = 0;
    let mut failed_count = 0;

    for candidate in &candidates {
        match candidate.state.label() {
            "new" => new_count += 1,
            "done" => done_count += 1,
            "failed" => failed_count += 1,
            _ => {}
        }
    }

    let mut html = format!(
        r#"<div class="scan-summary">
<span class="badge ok">new: {}</span>
<span class="badge muted">done: {}</span>
<span class="badge bad">failed: {}</span>
<span class="badge muted">gesamt: {}</span>
</div>"#,
        new_count,
        done_count,
        failed_count,
        candidates.len()
    );

    if candidates.is_empty() {
        html.push_str(r#"<div class="small">Keine Kandidaten gefunden.</div>"#);
        return html;
    }

    html.push_str(r#"<div class="scan-list">"#);

    for candidate in candidates.iter().take(25) {
        let label = candidate.state.label();
        let class = match label {
            "new" => "ok",
            "done" => "muted",
            "failed" => "bad",
            _ => "muted",
        };

        html.push_str(&format!(
            r#"<div class="scan-row"><span class="badge {}">{}</span><span class="scan-path">{}</span><span class="scan-actions">{}</span></div>"#,
            class,
            escape(label),
            escape(&candidate.path.display().to_string()),
            action_button_html(label, &candidate.path.display().to_string())
        ));
    }

    if candidates.len() > 25 {
        html.push_str(&format!(
            r#"<div class="small">Weitere {} Kandidaten ausgeblendet. Vollständig über <code>/api/scan</code>.</div>"#,
            candidates.len() - 25
        ));
    }

    html.push_str("</div>");
    html
}

fn action_button_html(state: &str, path: &str) -> String {
    match state {
        "failed" => format!(
            r#"<button class="button small-button danger-button" type="button" data-action="clear-failed" data-path="{}">Failed zurücksetzen</button>"#,
            escape(path)
        ),
        "new" => format!(
            r#"<button class="button small-button" type="button" data-action="process" data-path="{}">Verarbeiten</button>"#,
            escape(path)
        ),
        _ => String::new(),
    }
}

#[derive(Debug, Clone)]
struct FailureEntry {
    marker: PathBuf,
    path: String,
    attempts: u64,
    error_class: String,
    reason: String,
}

fn failures_html(config: &Config) -> String {
    let entries = match failure_entries(&config.history.directory, 10) {
        Ok(entries) => entries,
        Err(err) => {
            return format!(
                r#"<div class="error">Fehlerliste konnte nicht geladen werden: {}</div>"#,
                escape(&format!("{:?}", err))
            );
        }
    };

    let mut html = format!(
        r#"<div class="scan-summary"><span class="badge bad">failed: {}</span></div>"#,
        entries.len()
    );

    if entries.is_empty() {
        html.push_str(r#"<div class="small">Keine fehlgeschlagenen Releases vorhanden.</div>"#);
        return html;
    }

    html.push_str(r#"<div class="failure-list">"#);

    for entry in entries {
        html.push_str(&format!(
            r#"<div class="failure-row">
<div><span class="badge bad">{}</span> <span class="small">Fehlversuche: {}</span></div>
<div class="scan-path">{}</div>
<div class="small">{}</div>
<div class="scan-actions"><button class="button small-button danger-button" type="button" data-action="clear-failed" data-path="{}">Failed zurücksetzen</button></div>
</div>"#,
            escape(&entry.error_class),
            entry.attempts,
            escape(&entry.path),
            escape(&entry.reason),
            escape(&entry.path),
        ));
    }

    html.push_str("</div>");
    html
}

fn failure_entries(history_dir: &str, limit: usize) -> Result<Vec<FailureEntry>> {
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

fn history_counts(history_dir: &str) -> (usize, usize) {
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

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#039;")
}
