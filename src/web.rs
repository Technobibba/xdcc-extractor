use crate::{config::Config, scan};
use anyhow::{Context, Result};
use axum::{
    Json, Router,
    extract::State,
    response::{Html, IntoResponse},
    routing::get,
};
use serde_json::json;
use std::{fs, net::SocketAddr, path::Path, sync::Arc};
use tracing::{info, warn};

#[derive(Clone)]
struct AppState {
    config: Config,
}

pub fn start(config: Config) -> Result<()> {
    if !config.web.enabled {
        info!("WebUI deaktiviert");
        return Ok(());
    }

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
            let state = Arc::new(AppState { config });

            let app = Router::new()
                .route("/", get(index))
                .route("/health", get(health))
                .route("/api/status", get(api_status))
                .route("/api/scan", get(api_scan))
                .route("/assets/app.js", get(app_js))
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

async fn health() -> &'static str {
    "ok"
}

async fn app_js() -> impl IntoResponse {
    (
        [("content-type", "application/javascript; charset=utf-8")],
        r#"
document.addEventListener('DOMContentLoaded', () => {
  const button = document.getElementById('refresh-scan');
  const target = document.getElementById('scan-content');

  if (!button || !target) {
    return;
  }

  button.addEventListener('click', async () => {
    button.disabled = true;
    const oldText = button.textContent;
    button.textContent = 'Aktualisiere...';

    try {
      const response = await fetch('/api/scan', { cache: 'no-store' });
      const data = await response.json();

      target.innerHTML = renderScan(data);
    } catch (error) {
      target.innerHTML = `<div class="error">Scan konnte nicht aktualisiert werden: ${escapeHtml(String(error))}</div>`;
    } finally {
      button.disabled = false;
      button.textContent = oldText;
    }
  });
});

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

    html += `
      <div class="scan-row">
        <span class="badge ${cssClass}">${escapeHtml(state)}</span>
        <span class="scan-path">${escapeHtml(candidate.path || '')}</span>
      </div>
    `;
  }

  if (candidates.length > 25) {
    html += `<div class="small">Weitere ${candidates.length - 25} Kandidaten ausgeblendet. Vollständig über <code>/api/scan</code>.</div>`;
  }

  html += `</div>`;

  return html;
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

async fn index(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let history = history_counts(&state.config.history.directory);
    let scan_html = scan_summary_html(&state.config);

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
  grid-template-columns: 88px 1fr;
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
.error {{
  color: var(--bad);
}}
button.button {{
  cursor: pointer;
  font-family: inherit;
}}
button.button:disabled {{
  opacity: .65;
  cursor: wait;
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
      <div class="small"><code>/api/scan</code></div>
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
  </div>

  <footer>
    Read-only Dashboard. Schreibaktionen wie Process, Clear-Failed und Config-Bearbeitung kommen später.
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
            r#"<div class="scan-row"><span class="badge {}">{}</span><span class="scan-path">{}</span></div>"#,
            class,
            escape(label),
            escape(&candidate.path.display().to_string())
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
}
