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
    gotify_priority_success: i32,
    gotify_priority_error: i32,
    gotify_notify_on_success: Option<String>,
    gotify_notify_on_error: Option<String>,
    gotify_notify_on_every_error: Option<String>,
    gotify_notify_after_attempts: u64,
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
                .route("/logs", get(logs))
                .route("/api/status", get(api_status))
                .route("/api/config", get(api_config))
                .route("/api/scan", get(api_scan))
                .route("/api/failures", get(api_failures))
                .route("/api/logs", get(api_logs))
                .route("/api/clear-failed", post(api_clear_failed))
                .route("/api/process", post(api_process))
                .route("/assets/app.js", get(app_js))
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

    Html(settings_edit_page_html(&config, &state.config_path, None))
}

async fn update_settings(
    State(state): State<Arc<AppState>>,
    Form(form): Form<SettingsForm>,
) -> impl IntoResponse {
    let message = match apply_settings_to_config_file(&state.config_path, &form) {
        Ok(()) => {
            info!(
                "WebUI Einstellungen gespeichert: {}",
                state.config_path.display()
            );
            "Gespeichert. Bitte Container neu starten, damit der Worker die neuen Werte übernimmt."
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

    Html(settings_edit_page_html(
        &config,
        &state.config_path,
        Some(&message),
    ))
}

fn apply_settings_to_config_file(path: &Path, form: &SettingsForm) -> Result<()> {
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

    fs::write(path, content)
        .with_context(|| format!("Konnte Config nicht schreiben: {}", path.display()))?;

    Ok(())
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

fn toml_bool(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

fn checked(value: bool) -> &'static str {
    if value { "checked" } else { "" }
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn settings_edit_page_html(config: &Config, config_path: &Path, message: Option<&str>) -> String {
    let message_html = message
        .map(|message| {
            format!(
                r#"<section class="notice">{}</section>"#,
                escape_html(message)
            )
        })
        .unwrap_or_default();

    format!(
        r#"<!doctype html>
<html lang="de">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>XDCC Extractor Einstellungen bearbeiten</title>
<style>
:root {{
  color-scheme: dark;
  --bg: #0f1115;
  --panel: #171a21;
  --text: #e6e6e6;
  --muted: #9aa4b2;
  --border: #2a2f3a;
  --ok: #25c26e;
}}
body {{
  margin: 0;
  font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  background: var(--bg);
  color: var(--text);
}}
main {{
  max-width: 900px;
  margin: 0 auto;
  padding: 32px 20px;
}}
h1 {{
  margin: 0 0 6px;
}}
.sub {{
  color: var(--muted);
  margin-bottom: 22px;
}}
.card, .notice {{
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 14px;
  padding: 18px;
  margin-bottom: 14px;
}}
.notice {{
  border-color: var(--ok);
}}
.grid {{
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 14px;
}}
label {{
  display: block;
  color: var(--muted);
  font-size: 14px;
  margin-bottom: 6px;
}}
input[type="number"] {{
  width: 100%;
  padding: 10px;
  border-radius: 10px;
  border: 1px solid var(--border);
  background: #11131a;
  color: var(--text);
}}
.check {{
  display: flex;
  gap: 10px;
  align-items: center;
  color: var(--text);
  margin: 10px 0;
}}
.actions {{
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
  margin: 18px 0;
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
  cursor: pointer;
}}
button.button {{
  background: #284f38;
}}
code {{
  color: #cfd7e6;
}}
.small {{
  color: var(--muted);
  font-size: 13px;
}}
@media (max-width: 720px) {{
  .grid {{
    grid-template-columns: 1fr;
  }}
}}
</style>
</head>
<body>
<main>
  <h1>Einstellungen bearbeiten</h1>
  <div class="sub">Config: <code>{config_path}</code></div>

  <div class="actions">
    <a class="button" href="/settings">Zurück</a>
    <a class="button" href="/">Dashboard</a>
    <a class="button" href="/logs">Logs</a>
  </div>

  {message_html}

  <form method="post" action="/settings/edit">
    <section class="card">
      <h2>Watch</h2>
      <div class="grid">
        <div>
          <label for="stable_after">stable_after Sekunden</label>
          <input id="stable_after" name="stable_after" type="number" min="1" value="{stable_after}">
        </div>
      </div>
      <label class="check"><input type="checkbox" name="allow_root_archives" {allow_root_archives}> Root-Archive erlauben</label>
    </section>

    <section class="card">
      <h2>Extract</h2>
      <label class="check"><input type="checkbox" name="dry_run" {dry_run}> Dry-Run aktiv</label>
      <label class="check"><input type="checkbox" name="delete_archives" {delete_archives}> Archive nach Erfolg löschen</label>
      <label class="check"><input type="checkbox" name="keep_failed" {keep_failed}> Fehlerhafte Archive behalten</label>
      <div class="small">Passwortdatei und Passwortlisten-Inhalt werden hier nicht bearbeitet.</div>
    </section>

    <section class="card">
      <h2>Retry / Startup</h2>
      <div class="grid">
        <div>
          <label for="retry_base_delay">base_delay Sekunden</label>
          <input id="retry_base_delay" name="retry_base_delay" type="number" min="1" value="{retry_base_delay}">
        </div>
        <div>
          <label for="retry_max_delay">max_delay Sekunden</label>
          <input id="retry_max_delay" name="retry_max_delay" type="number" min="1" value="{retry_max_delay}">
        </div>
      </div>
      <label class="check"><input type="checkbox" name="startup_scan_existing" {startup_scan_existing}> Beim Start vorhandene Releases scannen</label>
    </section>

    <section class="card">
      <h2>Gotify</h2>
      <label class="check"><input type="checkbox" name="gotify_enabled" {gotify_enabled}> Gotify aktiv</label>
      <div class="grid">
        <div>
          <label for="gotify_priority_success">priority_success</label>
          <input id="gotify_priority_success" name="gotify_priority_success" type="number" value="{gotify_priority_success}">
        </div>
        <div>
          <label for="gotify_priority_error">priority_error</label>
          <input id="gotify_priority_error" name="gotify_priority_error" type="number" value="{gotify_priority_error}">
        </div>
        <div>
          <label for="gotify_notify_after_attempts">notify_after_attempts</label>
          <input id="gotify_notify_after_attempts" name="gotify_notify_after_attempts" type="number" min="1" value="{gotify_notify_after_attempts}">
        </div>
      </div>
      <label class="check"><input type="checkbox" name="gotify_notify_on_success" {gotify_notify_on_success}> Erfolg melden</label>
      <label class="check"><input type="checkbox" name="gotify_notify_on_error" {gotify_notify_on_error}> Fehler melden</label>
      <label class="check"><input type="checkbox" name="gotify_notify_on_every_error" {gotify_notify_on_every_error}> Jeden Fehler melden</label>
      <div class="small">Gotify URL und Token werden hier nicht angezeigt oder bearbeitet.</div>
    </section>

    <div class="actions">
      <button class="button" type="submit">Speichern</button>
      <a class="button" href="/settings">Abbrechen</a>
    </div>
  </form>
</main>
</body>
</html>"#,
        config_path = escape_html(&config_path.display().to_string()),
        message_html = message_html,
        stable_after = config.watch.stable_after,
        allow_root_archives = checked(config.watch.allow_root_archives),
        dry_run = checked(config.extract.dry_run),
        delete_archives = checked(config.extract.delete_archives),
        keep_failed = checked(config.extract.keep_failed),
        retry_base_delay = config.retry.base_delay,
        retry_max_delay = config.retry.max_delay,
        startup_scan_existing = checked(config.startup.scan_existing),
        gotify_enabled = checked(config.notifications.gotify.enabled),
        gotify_priority_success = config.notifications.gotify.priority_success,
        gotify_priority_error = config.notifications.gotify.priority_error,
        gotify_notify_on_success = checked(config.notifications.gotify.notify_on_success),
        gotify_notify_on_error = checked(config.notifications.gotify.notify_on_error),
        gotify_notify_on_every_error = checked(config.notifications.gotify.notify_on_every_error),
        gotify_notify_after_attempts = config.notifications.gotify.notify_after_attempts,
    )
}

async fn logs() -> Html<String> {
    Html(logs_page_html())
}

async fn api_logs() -> Json<serde_json::Value> {
    Json(json!({
        "lines": log_buffer::recent(300),
        "limit": 300
    }))
}

fn logs_page_html() -> String {
    format!(
        r#"<!doctype html>
<html lang="de">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>XDCC Extractor Logs</title>
<style>
:root {{
  --bg: #11131a;
  --card: #181b24;
  --text: #e7ecf4;
  --muted: #9aa4b2;
  --border: #2a3040;
}}
* {{
  box-sizing: border-box;
}}
body {{
  margin: 0;
  background: var(--bg);
  color: var(--text);
  font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
}}
main {{
  max-width: 1200px;
  margin: 0 auto;
  padding: 28px;
}}
h1 {{
  margin: 0 0 6px;
}}
.sub {{
  color: var(--muted);
  margin-bottom: 18px;
}}
.actions {{
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  margin-bottom: 18px;
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
  cursor: pointer;
}}
.button:hover {{
  background: #2b3142;
}}
.card {{
  background: var(--card);
  border: 1px solid var(--border);
  border-radius: 16px;
  padding: 18px;
}}
.logbox {{
  min-height: 520px;
  max-height: 70vh;
  overflow: auto;
  white-space: pre-wrap;
  word-break: break-word;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", monospace;
  font-size: 13px;
  line-height: 1.45;
  color: #d8dee9;
}}
.small {{
  color: var(--muted);
  font-size: 13px;
  margin-top: 12px;
}}
code {{
  color: #cfd7e6;
}}
</style>
</head>
<body>
<main>
  <h1>Logs</h1>
  <div class="sub">XDCC Extractor Version {version}</div>

  <div class="actions">
    <a class="button" href="/">Zurück zum Dashboard</a>
    <a class="button" href="/settings">Einstellungen</a>
    <button id="refresh-logs" class="button" type="button">Logs aktualisieren</button>
    <a class="button" href="/api/logs" target="_blank" rel="noopener">Logs API öffnen</a>
  </div>

  <section class="card">
    <pre id="log-output" class="logbox">Lade Logs...</pre>
    <div class="small">Zeigt die letzten 300 Logzeilen aus dem laufenden Prozess. Secrets werden nicht aus Config oder Passwortliste gelesen.</div>
  </section>
</main>

<script>
async function refreshLogs() {{
  const output = document.getElementById('log-output');
  const button = document.getElementById('refresh-logs');

  if (button) {{
    button.disabled = true;
    button.textContent = 'Lade...';
  }}

  try {{
    const response = await fetch('/api/logs', {{ cache: 'no-store' }});

    if (!response.ok) {{
      throw new Error('HTTP ' + response.status);
    }}

    const data = await response.json();
    const lines = Array.isArray(data.lines) ? data.lines : [];

    output.textContent = lines.length
      ? lines.join('\n')
      : 'Noch keine Logs im WebUI-Puffer.';
    output.scrollTop = output.scrollHeight;
  }} catch (error) {{
    output.textContent = 'Logs konnten nicht geladen werden: ' + error.message;
  }} finally {{
    if (button) {{
      button.disabled = false;
      button.textContent = 'Logs aktualisieren';
    }}
  }}
}}

document.addEventListener('DOMContentLoaded', () => {{
  const button = document.getElementById('refresh-logs');

  if (button) {{
    button.addEventListener('click', refreshLogs);
  }}

  refreshLogs();
  setInterval(refreshLogs, 5000);
}});
</script>
</body>
</html>"#,
        version = env!("CARGO_PKG_VERSION")
    )
}

async fn app_js() -> impl IntoResponse {
    (
        [("content-type", "application/javascript; charset=utf-8")],
        r#"
document.addEventListener('DOMContentLoaded', () => {
  const refreshScanButton = document.getElementById('refresh-scan');
  const refreshFailuresButton = document.getElementById('refresh-failures');

  if (refreshScanButton) {
    refreshScanButton.addEventListener('click', async () => {
      await refreshScan(refreshScanButton);
    });
  }

  if (refreshFailuresButton) {
    refreshFailuresButton.addEventListener('click', async () => {
      await refreshFailures(refreshFailuresButton);
    });
  }

  document.addEventListener('click', async (event) => {
    const button = event.target.closest('button[data-action][data-path]');

    if (!button) {
      return;
    }

    const action = button.dataset.action;
    const path = button.dataset.path;

    if (!action || !path) {
      return;
    }

    let endpoint = null;
    let confirmText = null;

    if (action === 'clear-failed') {
      endpoint = '/api/clear-failed';
      confirmText = `Failed-Marker zurücksetzen?\n\n${path}`;
    }

    if (action === 'process') {
      endpoint = '/api/process';
      confirmText = `Release jetzt manuell verarbeiten?\n\n${path}`;
    }

    if (!endpoint) {
      return;
    }

    if (!window.confirm(confirmText)) {
      return;
    }

    button.disabled = true;
    const oldText = button.textContent;
    button.textContent = 'Läuft...';

    try {
      const response = await fetch(endpoint, {
        method: 'POST',
        headers: {
          'content-type': 'application/json'
        },
        body: JSON.stringify({ path })
      });

      const data = await response.json();

      if (!data.ok) {
        window.alert(data.error || 'Aktion fehlgeschlagen');
        return;
      }

      await refreshScan();
      await refreshFailures();
    } catch (error) {
      window.alert(`Aktion fehlgeschlagen: ${String(error)}`);
    } finally {
      button.disabled = false;
      button.textContent = oldText;
    }
  });
});

async function refreshScan(button) {
  const target = document.getElementById('scan-content');

  if (!target) {
    return;
  }

  let oldText = null;

  if (button) {
    button.disabled = true;
    oldText = button.textContent;
    button.textContent = 'Aktualisiere...';
  }

  try {
    const response = await fetch('/api/scan', { cache: 'no-store' });
    const data = await response.json();

    target.innerHTML = renderScan(data);
  } catch (error) {
    target.innerHTML = `<div class="error">Scan konnte nicht aktualisiert werden: ${escapeHtml(String(error))}</div>`;
  } finally {
    if (button) {
      button.disabled = false;
      button.textContent = oldText;
    }
  }
}

async function refreshFailures(button) {
  const target = document.getElementById('failure-content');

  if (!target) {
    return;
  }

  let oldText = null;

  if (button) {
    button.disabled = true;
    oldText = button.textContent;
    button.textContent = 'Aktualisiere...';
  }

  try {
    const response = await fetch('/api/failures', { cache: 'no-store' });
    const data = await response.json();

    target.innerHTML = renderFailures(data);
  } catch (error) {
    target.innerHTML = `<div class="error">Fehlerliste konnte nicht aktualisiert werden: ${escapeHtml(String(error))}</div>`;
  } finally {
    if (button) {
      button.disabled = false;
      button.textContent = oldText;
    }
  }
}

function renderScan(data) {
  if (!data.ok) {
    return `<div class="error">Scan-Fehler: ${escapeHtml(data.error || 'Unbekannter Fehler')}</div>`;
  }

  const candidates = data.candidates || [];
  const counts = {
    new: 0,
    done: 0,
    failed: 0
  };

  for (const candidate of candidates) {
    if (candidate.state === 'new') counts.new += 1;
    if (candidate.state === 'done') counts.done += 1;
    if (candidate.state === 'failed') counts.failed += 1;
  }

  let html = `
    <div class="scan-summary">
      <span class="badge ok">new: ${counts.new}</span>
      <span class="badge muted">done: ${counts.done}</span>
      <span class="badge bad">failed: ${counts.failed}</span>
      <span class="badge muted">gesamt: ${candidates.length}</span>
    </div>
    <div class="small">Aktualisiert: ${escapeHtml(new Date().toLocaleTimeString())}</div>
  `;

  if (candidates.length === 0) {
    html += `<div class="small">Keine Kandidaten gefunden.</div>`;
    return html;
  }

  html += `<div class="scan-list">`;

  for (const candidate of candidates.slice(0, 25)) {
    const state = candidate.state || 'unknown';
    const cssClass = stateClass(state);
    const path = candidate.path || '';

    html += `
      <div class="scan-row">
        <span class="badge ${cssClass}">${escapeHtml(state)}</span>
        <span class="scan-path">${escapeHtml(path)}</span>
        <span class="scan-actions">${actionButton(state, path)}</span>
      </div>
    `;
  }

  if (candidates.length > 25) {
    html += `<div class="small">Weitere ${candidates.length - 25} Kandidaten ausgeblendet. Vollständig über <code>/api/scan</code>.</div>`;
  }

  html += `</div>`;

  return html;
}

function renderFailures(data) {
  if (!data.ok) {
    return `<div class="error">Fehler-API: ${escapeHtml(data.error || 'Unbekannter Fehler')}</div>`;
  }

  const failures = data.failures || [];

  let html = `
    <div class="scan-summary">
      <span class="badge bad">failed: ${failures.length}</span>
      <span class="badge muted">Aktualisiert: ${escapeHtml(new Date().toLocaleTimeString())}</span>
    </div>
  `;

  if (failures.length === 0) {
    html += `<div class="small">Keine fehlgeschlagenen Releases vorhanden.</div>`;
    return html;
  }

  html += `<div class="failure-list">`;

  for (const failure of failures) {
    const path = failure.path || '';

    html += `
      <div class="failure-row">
        <div>
          <span class="badge bad">${escapeHtml(failure.error_class || 'failed')}</span>
          <span class="small">Fehlversuche: ${escapeHtml(String(failure.attempts || 0))}</span>
        </div>
        <div class="scan-path">${escapeHtml(path)}</div>
        <div class="small">${escapeHtml(failure.reason || 'Kein Grund gefunden')}</div>
        <div class="scan-actions">
          <button class="button small-button danger-button" type="button" data-action="clear-failed" data-path="${escapeHtml(path)}">Failed zurücksetzen</button>
        </div>
      </div>
    `;
  }

  html += `</div>`;

  return html;
}

function actionButton(state, path) {
  const escapedPath = escapeHtml(path);

  if (state === 'failed') {
    return `<button class="button small-button danger-button" type="button" data-action="clear-failed" data-path="${escapedPath}">Failed zurücksetzen</button>`;
  }

  if (state === 'new') {
    return `<button class="button small-button" type="button" data-action="process" data-path="${escapedPath}">Verarbeiten</button>`;
  }

  return '';
}

function stateClass(state) {
  if (state === 'new') return 'ok';
  if (state === 'failed') return 'bad';
  return 'muted';
}

function escapeHtml(value) {
  return String(value)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#039;');
}
"#,
    )
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

  <div class="actions">
    <a class="button" href="/">Zurück zum Dashboard</a>
    <a class="button" href="/logs">Logs</a>
    <a class="button" href="/settings/edit">Bearbeiten</a>
    <a class="button" href="/api/config" target="_blank" rel="noopener">Config API öffnen</a>
    <a class="button" href="/api/status" target="_blank" rel="noopener">Status API öffnen</a>
  </div>

  <div class="grid">
    <section class="card">
      <h2>Watch</h2>
      <div class="row"><div class="key">directory</div><div class="value"><code>{watch_dir}</code></div></div>
      <div class="row"><div class="key">stable_after</div><div class="value">{stable_after}s</div></div>
      <div class="row"><div class="key">allow_root_archives</div><div class="value">{allow_root_archives}</div></div>
    </section>

    <section class="card">
      <h2>Extract</h2>
      <div class="row"><div class="key">dry_run</div><div class="value">{dry_run}</div></div>
      <div class="row"><div class="key">delete_archives</div><div class="value">{delete_archives}</div></div>
      <div class="row"><div class="key">keep_failed</div><div class="value">{keep_failed}</div></div>
      <div class="row"><div class="key">password_file</div><div class="value">{password_file_configured} <span class="key">Pfad wird angezeigt, Inhalt nicht</span></div></div>
      <div class="row"><div class="key">password_file_path</div><div class="value"><code>{password_file}</code></div></div>
    </section>

    <section class="card">
      <h2>Output / History</h2>
      <div class="row"><div class="key">output.directory</div><div class="value"><code>{output_dir}</code></div></div>
      <div class="row"><div class="key">history.directory</div><div class="value"><code>{history_dir}</code></div></div>
    </section>

    <section class="card">
      <h2>Retry / Startup</h2>
      <div class="row"><div class="key">base_delay</div><div class="value">{base_delay}s</div></div>
      <div class="row"><div class="key">max_delay</div><div class="value">{max_delay}s</div></div>
      <div class="row"><div class="key">scan_existing</div><div class="value">{startup_scan}</div></div>
    </section>

    <section class="card">
      <h2>Gotify</h2>
      <div class="row"><div class="key">enabled</div><div class="value">{gotify_enabled}</div></div>
      <div class="row"><div class="key">url</div><div class="value"><code>{gotify_url}</code></div></div>
      <div class="row"><div class="key">token_configured</div><div class="value">{token_configured}</div></div>
      <div class="row"><div class="key">priority_success</div><div class="value">{priority_success}</div></div>
      <div class="row"><div class="key">priority_error</div><div class="value">{priority_error}</div></div>
      <div class="row"><div class="key">notify_on_success</div><div class="value">{notify_on_success}</div></div>
      <div class="row"><div class="key">notify_on_error</div><div class="value">{notify_on_error}</div></div>
      <div class="row"><div class="key">notify_on_every_error</div><div class="value">{notify_on_every_error}</div></div>
      <div class="row"><div class="key">notify_after_attempts</div><div class="value">{notify_after_attempts}</div></div>
    </section>

    <section class="card">
      <h2>WebUI</h2>
      <div class="row"><div class="key">enabled</div><div class="value">{web_enabled}</div></div>
      <div class="row"><div class="key">bind</div><div class="value"><code>{web_bind}</code></div></div>
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
        allow_root_archives = state.config.watch.allow_root_archives,
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
        gotify_url = escape(&state.config.notifications.gotify.url),
        token_configured = token_configured,
        priority_success = state.config.notifications.gotify.priority_success,
        priority_error = state.config.notifications.gotify.priority_error,
        notify_on_success = state.config.notifications.gotify.notify_on_success,
        notify_on_error = state.config.notifications.gotify.notify_on_error,
        notify_on_every_error = state.config.notifications.gotify.notify_on_every_error,
        notify_after_attempts = state.config.notifications.gotify.notify_after_attempts,
        web_enabled = state.config.web.enabled,
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

  <div class="grid">
    <section class="card">
      <h2>Worker</h2>
      <div class="value">läuft</div>
      <div class="small">WebUI erreichbar</div>
    </section>

    <section class="card">
      <h2>Aktionen</h2>
      <div class="actions">
        <a class="button" href="/">Aktualisieren</a>
        <a class="button" href="/settings">Einstellungen</a>
        <a class="button" href="/logs">Logs</a>
        <a class="button" href="/api/status" target="_blank" rel="noopener">Status API</a>
        <a class="button" href="/api/scan" target="_blank" rel="noopener">Scan API</a>
      </div>
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
      <h2>API</h2>
      <div class="small"><code>/api/status</code></div>
      <div class="small"><code>/api/config</code></div>
      <div class="small"><code>/api/scan</code></div>
      <div class="small"><code>/api/logs</code></div>
      <div class="small"><code>/api/clear-failed</code></div>
      <div class="small"><code>/api/process</code></div>
      <div class="small"><code>/health</code></div>
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
