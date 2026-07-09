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
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
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
        Some("RESET") => {
            match crate::web_maintenance::reset_history_files(&config.history.directory) {
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
            }
        }
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

    let message = match crate::web_maintenance::append_password_to_file(&config, &form.password) {
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
        Some("REPLACE") => {
            match crate::web_maintenance::replace_password_file(&config, &form.passwords) {
                Ok(count) => {
                    info!("Passwortliste über WebUI ersetzt: {} Einträge.", count);
                    format!(
                        "Passwortliste wurde ersetzt. {} Passwort/Passwörter gespeichert. Backup wurde erstellt. Neustart erforderlich.",
                        count
                    )
                }
                Err(err) => format!("Passwortliste konnte nicht ersetzt werden: {err:?}"),
            }
        }
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
