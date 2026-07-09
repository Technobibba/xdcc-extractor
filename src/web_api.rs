use crate::{history::History, log_buffer, manual_process, scan, web::AppState};
use axum::{Json, extract::State};
use serde::Deserialize;
use serde_json::json;
use std::{path::Path, sync::Arc};
use tracing::info;

#[derive(Debug, Deserialize)]
pub(crate) struct PathRequest {
    path: String,
}

pub(crate) async fn api_logs() -> Json<serde_json::Value> {
    Json(json!({
        "lines": log_buffer::recent(300),
        "limit": 300
    }))
}

pub(crate) async fn api_config(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
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

pub(crate) async fn api_status(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
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

pub(crate) async fn api_scan(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
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

pub(crate) async fn api_failures(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
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

pub(crate) async fn api_clear_failed(
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

pub(crate) async fn api_process(
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

pub(crate) async fn api_restart() -> Json<serde_json::Value> {
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
