use crate::config::Config;
use crate::scan;
use std::path::Path;

fn checked(value: bool) -> &'static str {
    if value { "checked" } else { "" }
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub fn logs_page_html() -> String {
    format!(
        r#"<!doctype html>
<html lang="de">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>XDCC Extractor Logs</title>
<link rel="stylesheet" href="/assets/logs.css">
</head>
<body>
<main>
  <h1>Logs</h1>
  <div class="sub">XDCC Extractor Version {version}</div>

    <div class="actions nav" style="margin-top: 22px; margin-bottom: 30px;">
    <a class="button" href="/">Dashboard</a>
    <a class="button" href="/settings">Einstellungen</a>
    <a class="button" href="/settings/edit">Bearbeiten</a>
    <a class="button" href="/logs">Logs</a>
    <button id="refresh-logs" class="button" type="button">Logs aktualisieren</button>
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

pub fn settings_edit_page_html(
    config: &Config,
    config_path: &Path,
    message: Option<&str>,
) -> String {
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
<link rel="stylesheet" href="/assets/settings-edit.css">
</head>
<body>
<main>
  <h1>Einstellungen bearbeiten</h1>
  <div class="sub">Config: <code>{config_path}</code></div>

    <div class="actions nav" style="margin-top: 22px; margin-bottom: 30px;">
    <a class="button" href="/">Dashboard</a>
    <a class="button" href="/settings">Einstellungen</a>
    <a class="button" href="/settings/edit">Bearbeiten</a>
    <a class="button" href="/logs">Logs</a>
  </div>

  {message_html}

  <form method="post" action="/settings/edit">
    <section class="card">
      <h2>Watch</h2>
      <div class="grid">
        <div>
          <label for="stable_after">Wartezeit bis Verarbeitung in Sekunden</label>
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
          <label for="retry_base_delay">Erste Wiederholung nach Sekunden</label>
          <input id="retry_base_delay" name="retry_base_delay" type="number" min="1" value="{retry_base_delay}">
        </div>
        <div>
          <label for="retry_max_delay">Maximale Wiederholungs-Wartezeit in Sekunden</label>
          <input id="retry_max_delay" name="retry_max_delay" type="number" min="1" value="{retry_max_delay}">
        </div>
      </div>
      <label class="check"><input type="checkbox" name="startup_scan_existing" {startup_scan_existing}> Beim Start vorhandene Releases scannen</label>
    </section>

    <section class="card">
      <h2>Gotify</h2>
      <label class="check"><input type="checkbox" name="gotify_enabled" {gotify_enabled}> Gotify aktiv</label>
      <div class="grid">
        <div class="field full">
          <label for="gotify_url">Gotify URL neu setzen</label>
          <input id="gotify_url" name="gotify_url" type="text" value="" placeholder="Leer lassen = bestehende URL behalten" autocomplete="off">
        </div>
        <div class="field full">
          <label for="gotify_token">Gotify Token neu setzen</label>
          <input id="gotify_token" name="gotify_token" type="password" value="" placeholder="Leer lassen = bestehenden Token behalten" autocomplete="new-password">
        </div>
      </div>
      <div class="small">Gotify URL und Token werden nie angezeigt. Leere Felder behalten die bestehenden Werte.</div>

      <div class="grid">
        <div>
          <label for="gotify_priority_success">Priorität bei Erfolg</label>
          <input id="gotify_priority_success" name="gotify_priority_success" type="number" value="{gotify_priority_success}">
        </div>
        <div>
          <label for="gotify_priority_error">Priorität bei Fehler</label>
          <input id="gotify_priority_error" name="gotify_priority_error" type="number" value="{gotify_priority_error}">
        </div>
        <div>
          <label for="gotify_notify_after_attempts">Fehler melden nach Versuchen</label>
          <input id="gotify_notify_after_attempts" name="gotify_notify_after_attempts" type="number" min="1" value="{gotify_notify_after_attempts}">
        </div>
      </div>
      <label class="check"><input type="checkbox" name="gotify_notify_on_success" {gotify_notify_on_success}> Erfolg melden</label>
      <label class="check"><input type="checkbox" name="gotify_notify_on_error" {gotify_notify_on_error}> Fehler melden</label>
      <label class="check"><input type="checkbox" name="gotify_notify_on_every_error" {gotify_notify_on_every_error}> Jeden Fehler melden</label>
      <div class="small">Gotify URL und Token werden nicht angezeigt. Beide können im Bearbeiten-Modus neu gesetzt werden.</div>
    </section>

    <div class="actions">
      <button class="button" type="submit">Speichern</button>
      <button id="restart-worker" class="button" type="button">Neustart</button>
      <a class="button" href="/settings">Abbrechen</a>
    </div>
    <div id="restart-status" class="small"></div>
  </form>


  <section class="card">
    <h2>History zurücksetzen</h2>
    <div class="small">Löscht alle <code>.done</code>- und <code>.failed</code>-Marker. Vorher wird ein Backup unter <code>/state/history-backups</code> erstellt.</div>
    <form method="post" action="/settings/history/reset">
      <label class="check"><input type="checkbox" name="confirm" value="RESET"> Ich verstehe, dass die History zurückgesetzt wird</label>
      <button class="button danger" type="submit">History löschen/resetten</button>
    </form>
  </section>

  <section class="card">
    <h2>Passwortliste verwalten</h2>
    <div class="small">Passwörter werden nicht angezeigt. Vor Änderungen wird ein Backup unter <code>/state/password-backups</code> erstellt.</div>

    <form method="post" action="/settings/passwords/add">
      <label for="password_add">Passwort hinzufügen</label>
      <input id="password_add" name="password" type="password" value="" autocomplete="new-password">
      <div class="actions">
        <button class="button" type="submit">Passwort hinzufügen</button>
      </div>
    </form>

    <form method="post" action="/settings/passwords/replace">
      <label for="passwords_replace">Passwortliste komplett ersetzen</label>
      <textarea id="passwords_replace" name="passwords" placeholder="Ein Passwort pro Zeile"></textarea>
      <label class="check"><input type="checkbox" name="confirm" value="REPLACE"> Ich verstehe, dass die bisherige Passwortliste ersetzt wird</label>
      <div class="actions">
        <button class="button danger" type="submit">Passwortliste ersetzen</button>
      </div>
    </form>
  </section>

</main>

<script>
document.addEventListener('DOMContentLoaded', () => {{
  const button = document.getElementById('restart-worker');
  const status = document.getElementById('restart-status');

  if (!button) {{
    return;
  }}

  button.addEventListener('click', async () => {{
    const ok = confirm('Worker jetzt neu starten? Die WebUI ist kurz nicht erreichbar.');
    if (!ok) {{
      return;
    }}

    button.disabled = true;
    status.textContent = 'Neustart wird ausgelöst. Seite lädt gleich neu...';

    try {{
      await fetch('/api/restart', {{ method: 'POST', cache: 'no-store' }});
    }} catch (error) {{
      // Beim Neustart kann die Verbindung abbrechen. Das ist hier erwartbar.
    }}

    setTimeout(() => {{
      window.location.href = '/settings/edit';
    }}, 12000);
  }});
}});
</script>

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

pub fn settings_page_html(config: &Config) -> String {
    let gotify_enabled = if config.notifications.gotify.enabled {
        r#"<span class="badge ok">aktiv</span>"#
    } else {
        r#"<span class="badge muted">aus</span>"#
    };

    let dry_run = if config.extract.dry_run {
        r#"<span class="badge ok">aktiv</span>"#
    } else {
        r#"<span class="badge warn">aus</span>"#
    };

    let delete_archives = if config.extract.delete_archives {
        r#"<span class="badge warn">aktiv</span>"#
    } else {
        r#"<span class="badge muted">aus</span>"#
    };

    let keep_failed = if config.extract.keep_failed {
        r#"<span class="badge ok">aktiv</span>"#
    } else {
        r#"<span class="badge muted">aus</span>"#
    };

    let startup_scan = if config.startup.scan_existing {
        r#"<span class="badge warn">aktiv</span>"#
    } else {
        r#"<span class="badge muted">aus</span>"#
    };

    let allow_root_archives = if config.watch.allow_root_archives {
        r#"<span class="badge ok">aktiv</span>"#
    } else {
        r#"<span class="badge muted">aus</span>"#
    };

    let notify_on_success = if config.notifications.gotify.notify_on_success {
        r#"<span class="badge ok">aktiv</span>"#
    } else {
        r#"<span class="badge muted">aus</span>"#
    };

    let notify_on_error = if config.notifications.gotify.notify_on_error {
        r#"<span class="badge ok">aktiv</span>"#
    } else {
        r#"<span class="badge muted">aus</span>"#
    };

    let notify_on_every_error = if config.notifications.gotify.notify_on_every_error {
        r#"<span class="badge warn">aktiv</span>"#
    } else {
        r#"<span class="badge muted">aus</span>"#
    };

    let web_enabled = if config.web.enabled {
        r#"<span class="badge ok">aktiv</span>"#
    } else {
        r#"<span class="badge muted">aus</span>"#
    };

    let gotify_url_configured = if config.notifications.gotify.url.trim().is_empty() {
        r#"<span class="badge bad">nein</span>"#
    } else {
        r#"<span class="badge ok">ja</span>"#
    };

    let token_configured = if config.notifications.gotify.token.trim().is_empty() {
        r#"<span class="badge bad">nein</span>"#
    } else {
        r#"<span class="badge ok">ja</span>"#
    };

    let password_file_configured = if config.extract.password_file.trim().is_empty() {
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
<link rel="stylesheet" href="/assets/settings.css">
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
        watch_dir = escape_html(&config.watch.directory),
        stable_after = config.watch.stable_after,
        allow_root_archives = allow_root_archives,
        dry_run = dry_run,
        delete_archives = delete_archives,
        keep_failed = keep_failed,
        password_file_configured = password_file_configured,
        password_file = escape_html(&config.extract.password_file),
        output_dir = escape_html(&config.output.directory),
        history_dir = escape_html(&config.history.directory),
        base_delay = config.retry.base_delay,
        max_delay = config.retry.max_delay,
        startup_scan = startup_scan,
        gotify_enabled = gotify_enabled,
        token_configured = token_configured,
        priority_success = config.notifications.gotify.priority_success,
        priority_error = config.notifications.gotify.priority_error,
        notify_on_success = notify_on_success,
        notify_on_error = notify_on_error,
        notify_on_every_error = notify_on_every_error,
        notify_after_attempts = config.notifications.gotify.notify_after_attempts,
        web_enabled = web_enabled,
        web_bind = escape_html(&config.web.bind),
    );

    html
}

pub fn dashboard_page_html(config: &Config) -> String {
    let history = crate::web_history::history_counts(&config.history.directory);
    let scan_html = scan_summary_html(config);
    let failures_html = failures_html(config);

    let dry_run_badge = if config.extract.dry_run {
        r#"<span class="badge ok">aktiv</span>"#
    } else {
        r#"<span class="badge warn">aus</span>"#
    };

    let gotify_badge = if config.notifications.gotify.enabled {
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
<link rel="stylesheet" href="/assets/dashboard.css">
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
        delete_archives = config.extract.delete_archives,
        gotify_badge = gotify_badge,
        done = history.0,
        failed = history.1,
        history_dir = escape_html(&config.history.directory),
        watch_dir = escape_html(&config.watch.directory),
        output_dir = escape_html(&config.output.directory),
        allow_root_archives = config.watch.allow_root_archives,
        scan_html = scan_html,
        failures_html = failures_html,
    );

    html
}

fn scan_summary_html(config: &Config) -> String {
    let candidates = match scan::scan_candidates_with_history(config) {
        Ok(candidates) => candidates,
        Err(err) => {
            return format!(
                r#"<div class="error">Scan-Fehler: {}</div>"#,
                escape_html(&format!("{:?}", err))
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
            escape_html(label),
            escape_html(&candidate.path.display().to_string()),
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
            escape_html(path)
        ),
        "new" => format!(
            r#"<button class="button small-button" type="button" data-action="process" data-path="{}">Verarbeiten</button>"#,
            escape_html(path)
        ),
        _ => String::new(),
    }
}

fn failures_html(config: &Config) -> String {
    let entries = match crate::web_history::failure_entries(&config.history.directory, 10) {
        Ok(entries) => entries,
        Err(err) => {
            return format!(
                r#"<div class="error">Fehlerliste konnte nicht geladen werden: {}</div>"#,
                escape_html(&format!("{:?}", err))
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
            escape_html(&entry.error_class),
            entry.attempts,
            escape_html(&entry.path),
            escape_html(&entry.reason),
            escape_html(&entry.path),
        ));
    }

    html.push_str("</div>");
    html
}
