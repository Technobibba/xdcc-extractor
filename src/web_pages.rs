use crate::config::Config;
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
input[type="number"],
input[type="text"],
input[type="password"],
textarea {{
  width: 100%;
  min-width: 0;
  padding: 10px;
  border-radius: 10px;
  border: 1px solid var(--border);
  background: #11131a;
  color: var(--text);
}}
.field {{
  min-width: 0;
}}
.field.full {{
  grid-column: 1 / -1;
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
.button.danger {{
  background: #4f2828;
}}
textarea {{
  min-height: 120px;
  resize: vertical;
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
