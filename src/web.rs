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
    Form(form): Form<crate::web_settings::SettingsForm>,
) -> impl IntoResponse {
    let message = match crate::web_settings::apply_settings_to_config_file(
        &state.config_path,
        &form,
    ) {
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
    let history = crate::web_history::history_counts(&state.config.history.directory);

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
    match crate::web_history::failure_entries(&state.config.history.directory, 25) {
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
    Html(crate::web_pages::settings_page_html(&state.config))
}

async fn index(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Html(crate::web_pages::dashboard_page_html(&state.config))
}
