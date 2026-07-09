use crate::config::Config;
use anyhow::{Context, Result};
use axum::{
    Router,
    extract::{Form, Request, State},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use base64::Engine as _;
use serde::Deserialize;
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tracing::{info, warn};

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) config: Config,
    pub(crate) config_path: PathBuf,
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
                .route("/api/status", get(crate::web_api::api_status))
                .route("/api/config", get(crate::web_api::api_config))
                .route("/api/scan", get(crate::web_api::api_scan))
                .route("/api/failures", get(crate::web_api::api_failures))
                .route("/api/logs", get(crate::web_api::api_logs))
                .route("/api/clear-failed", post(crate::web_api::api_clear_failed))
                .route("/api/process", post(crate::web_api::api_process))
                .route("/api/restart", post(crate::web_api::api_restart))
                .route("/assets/common.css", get(crate::web_styles::common_css))
                .route(
                    "/assets/dashboard.css",
                    get(crate::web_styles::dashboard_css),
                )
                .route("/assets/settings.css", get(crate::web_styles::settings_css))
                .route(
                    "/assets/settings-edit.css",
                    get(crate::web_styles::settings_edit_css),
                )
                .route("/assets/logs.css", get(crate::web_styles::logs_css))
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

async fn settings(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Html(crate::web_pages::settings_page_html(&state.config))
}

async fn index(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Html(crate::web_pages::dashboard_page_html(&state.config))
}
