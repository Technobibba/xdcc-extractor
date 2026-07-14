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
<link rel="stylesheet" href="/assets/common.css">
<link rel="stylesheet" href="/assets/logs.css">
</head>
<body>
<main>
  <h1>Logs</h1>

    <div class="actions nav" style="margin-top: 22px; margin-bottom: 30px;">
    <a class="button" href="/">Dashboard</a>
    <a class="button" href="/settings">Einstellungen</a>
    <a class="button" href="/settings/edit">Bearbeiten</a>
    <a class="button" href="/logs">Logs</a>
    <a class="button" href="/diagnostics">Diagnose</a>
    <button id="refresh-logs" class="button" type="button">Logs aktualisieren</button>
  </div>

  <section class="card">
    <pre id="log-output" class="logbox">Logs werden geladen...</pre>
    <div class="small">Zeigt die letzten 300 Logzeilen des laufenden Workers. Vertrauliche Daten werden nicht aus der Konfigurationsdatei oder der Passwortliste gelesen.</div>
  </section>
  <footer class="page-version">
    Version {version}
  </footer>
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
            let notice_class =
                if message.contains("Neustart") || message.contains("starte den Worker neu") {
                    "notice restart-required"
                } else if message.contains("fehlgeschlagen")
                    || message.contains("konnte nicht")
                    || message.contains("nicht zurückgesetzt")
                    || message.contains("nicht ersetzt")
                {
                    "notice notice-error"
                } else {
                    "notice notice-success"
                };

            format!(
                r#"<section class="{}" role="status">{}</section>"#,
                notice_class,
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
<link rel="stylesheet" href="/assets/common.css">
<link rel="stylesheet" href="/assets/settings-edit.css">
</head>
<body>
<main>
  <h1>Einstellungen bearbeiten</h1>
  <div class="sub">Konfigurationsdatei: <code>{config_path}</code></div>

    <div class="actions nav" style="margin-top: 22px; margin-bottom: 30px;">
    <a class="button" href="/">Dashboard</a>
    <a class="button" href="/settings">Einstellungen</a>
    <a class="button" href="/settings/edit">Bearbeiten</a>
    <a class="button" href="/logs">Logs</a>
    <a class="button" href="/diagnostics">Diagnose</a>
    <button id="restart-worker" class="button" type="button">Worker neu starten</button>
  </div>

  {message_html}
  <div id="restart-status" class="restart-status" role="status" aria-live="polite"></div>

  <form method="post" action="/settings/edit">
    <section class="card">
      <h2>Überwachung</h2>

      <div class="grid">
        <div class="field full">
          <label for="watch_directories">
            Überwachte Ordner
          </label>

          <textarea
            id="watch_directories"
            name="watch_directories"
            rows="6"
            required
            spellcheck="false"
          >{watch_directories}</textarea>

          <div class="small">
            Ein absoluter Container-Pfad pro Zeile.
            Leere Zeilen und doppelte Einträge werden
            beim Speichern automatisch entfernt.
          </div>
        </div>

        <div>
          <label for="stable_after">
            Wartezeit bis Verarbeitung in Sekunden
          </label>

          <input
            id="stable_after"
            name="stable_after"
            type="number"
            min="1"
            value="{stable_after}"
          >
        </div>
      </div>

      <label class="check">
        <input
          type="checkbox"
          name="allow_root_archives"
          {allow_root_archives}
        >
        Archive im Hauptverzeichnis erlauben
      </label>
    </section>

    <section class="card">
      <h2>Entpacken</h2>
      <label class="check"><input type="checkbox" name="delete_archives" {delete_archives}> Archive nach Erfolg löschen</label>
      <label class="check"><input type="checkbox" name="keep_failed" {keep_failed}> Fehlerhafte Archive behalten</label>

      <div class="settings-group">
        <h3>Passwortliste verwalten</h3>
        <div class="small">Passwörter werden nicht angezeigt. Vor Änderungen wird eine Sicherung unter <code>/state/password-backups</code> erstellt.</div>

        <label for="password_add">Passwort hinzufügen</label>
        <input id="password_add" name="password" type="password" value="" autocomplete="new-password">
        <div class="actions">
          <button class="button" type="submit" formaction="/settings/passwords/add" formmethod="post">Passwort hinzufügen</button>
        </div>

        <label for="passwords_replace">Gesamte Passwortliste ersetzen</label>
        <textarea id="passwords_replace" name="passwords" placeholder="Ein Passwort pro Zeile"></textarea>
        <label class="check"><input type="checkbox" name="confirm" value="REPLACE"> Ich bestätige, dass die bisherige Passwortliste ersetzt wird</label>
        <div class="actions">
          <button class="button danger" type="submit" formaction="/settings/passwords/replace" formmethod="post">Passwortliste ersetzen</button>
        </div>
      </div>
    </section>

    <section class="card">
      <h2>Wiederholung / Start</h2>
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
      <label class="check"><input type="checkbox" name="startup_scan_existing" {startup_scan_existing}> Vorhandene Releases beim Start prüfen</label>
    </section>

    <section class="card">
      <h2>Benachrichtigungen</h2>
      <label class="check"><input type="checkbox" name="notifications_enabled" {notifications_enabled}> ntfy-Benachrichtigungen aktivieren</label>

      <div class="settings-group">
        <h3>Verbindung</h3>
        <div class="grid">
          <div class="field full">
            <label for="ntfy_server">ntfy-Server-URL</label>
            <input id="ntfy_server" name="ntfy_server" type="url" value="" placeholder="https://ntfy.example.org" autocomplete="url" spellcheck="false">
            <div class="small">URL des ntfy-Servers. Leer lassen, um den gespeicherten Wert beizubehalten.</div>
          </div>
          <div class="field full">
            <label for="ntfy_topic">Thema (Topic)</label>
            <input id="ntfy_topic" name="ntfy_topic" type="text" value="" placeholder="homelab-downloads" autocomplete="off" spellcheck="false">
            <div class="small">Thema, an das der XDCC Extractor seine Benachrichtigungen sendet.</div>
          </div>
          <div class="field full">
            <label for="ntfy_token">Zugriffstoken</label>
            <input id="ntfy_token" name="ntfy_token" type="text" value="" placeholder="tk_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx" autocomplete="new-password" spellcheck="false">
            <div class="small">Ein neu eingegebener Zugriffstoken ist zur Kontrolle sichtbar. Leer lassen, um den gespeicherten Token beizubehalten.</div>
          </div>
        </div>
      </div>

      <div class="settings-group">
        <h3>Prioritäten</h3>
        <div class="grid ntfy-priority-grid">
        <div>
          <label for="ntfy_priority_success">Priorität bei Erfolg</label>
          <input id="ntfy_priority_success" name="ntfy_priority_success" type="number" min="1" max="5" value="{ntfy_priority_success}">
        </div>
        <div>
          <label for="ntfy_priority_error">Priorität bei Fehler</label>
          <input id="ntfy_priority_error" name="ntfy_priority_error" type="number" min="1" max="5" value="{ntfy_priority_error}">
        </div>
        <div>
          <label for="ntfy_notify_after_attempts">Fehler melden nach Versuchen</label>
          <input id="ntfy_notify_after_attempts" name="ntfy_notify_after_attempts" type="number" min="1" value="{ntfy_notify_after_attempts}">
        </div>
        </div>
      </div>

      <div class="settings-group">
        <h3>Ereignisse</h3>
      <label class="check"><input type="checkbox" name="ntfy_notify_on_worker_start" {ntfy_notify_on_worker_start}> Worker-Start melden</label>
      <label class="check"><input type="checkbox" name="ntfy_notify_on_processing_start" {ntfy_notify_on_processing_start}> Beginn einer Verarbeitung melden</label>
      <label class="check"><input type="checkbox" name="ntfy_notify_on_success" {ntfy_notify_on_success}> Erfolgreiche Verarbeitung melden</label>
      <label class="check"><input type="checkbox" name="ntfy_notify_on_error" {ntfy_notify_on_error}> Fehler melden</label>
      <label class="check"><input type="checkbox" name="ntfy_notify_on_every_error" {ntfy_notify_on_every_error}> Jeden Fehler melden</label>
      <div class="small">Server-URL, Thema und Zugriffstoken können hier geändert werden.</div>
    </section>
    <section class="card">
      <h2>ntfy testen</h2>
      <div class="small">Der Test verwendet die zuletzt gespeicherte Konfiguration. Speichere Änderungen zuerst und starte anschließend den Test.</div>
      <button type="submit" formaction="/settings/notifications/test" formmethod="post" style="margin-top: 16px;">ntfy-Verbindung testen</button>
    </section>

    <div class="actions">
      <button class="button" type="submit">Änderungen speichern</button>
      <a class="button" href="/settings">Abbrechen</a>
    </div>
  </form>


  <section class="card">
    <h2>Verlauf zurücksetzen</h2>
    <div class="small">Entfernt alle Verlaufsmarkierungen für erfolgreiche und fehlgeschlagene Releases. Vorher wird eine Sicherung unter <code>/state/history-backups</code> erstellt.</div>
    <form method="post" action="/settings/history/reset">
      <label class="check"><input type="checkbox" name="confirm" value="RESET"> Ich bestätige, dass der Verlauf zurückgesetzt wird</label>
      <button class="button danger" type="submit">Verlauf zurücksetzen</button>
    </form>
  </section>

</main>

<script>
document.addEventListener('DOMContentLoaded', () => {{
  document.querySelectorAll('form').forEach((form) => {{
    form.addEventListener('submit', () => {{
      const submit = form.querySelector(
        'button[type="submit"]'
      );

      if (!submit) {{
        return;
      }}

      const text = submit.textContent || '';

      submit.disabled = true;
      submit.classList.add('is-loading');

      submit.textContent = text
        .toLowerCase()
        .includes('speichern')
        ? 'Wird gespeichert …'
        : 'Wird ausgeführt …';
    }});
  }});

  const button = document.getElementById('restart-worker');
  const status = document.getElementById('restart-status');

  if (!button) {{
    return;
  }}

  button.addEventListener('click', async () => {{
    const ok = confirm('Worker jetzt neu starten? Die WebUI ist für kurze Zeit nicht erreichbar.');
    if (!ok) {{
      return;
    }}

    button.disabled = true;
    button.classList.add('is-loading');
    button.textContent = 'Neustart läuft …';

    status.className = 'restart-status restart-pending';
    status.textContent =
      'Neustart wird ausgelöst. Die WebUI wird automatisch neu geladen …';

    try {{
      await fetch('/api/restart', {{ method: 'POST', cache: 'no-store' }});
    }} catch (error) {{
      // Beim Neustart kann die Verbindung abbrechen. Das ist hier erwartbar.
    }}

    setTimeout(() => {{
      status.textContent =
        'Worker startet neu. Warte auf die WebUI …';
    }}, 2500);

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
        watch_directories = escape_html(&config.watch.resolved_directories().join("\n")),
        stable_after = config.watch.stable_after,
        allow_root_archives = checked(config.watch.allow_root_archives),
        delete_archives = checked(config.extract.delete_archives),
        keep_failed = checked(config.extract.keep_failed),
        retry_base_delay = config.retry.base_delay,
        retry_max_delay = config.retry.max_delay,
        startup_scan_existing = checked(config.startup.scan_existing),
        notifications_enabled = checked(config.notifications.enabled),
        ntfy_priority_success = config.notifications.ntfy.priority_success,
        ntfy_priority_error = config.notifications.ntfy.priority_error,
        ntfy_notify_on_worker_start = checked(config.notifications.ntfy.notify_on_worker_start),
        ntfy_notify_on_processing_start =
            checked(config.notifications.ntfy.notify_on_processing_start),
        ntfy_notify_on_success = checked(config.notifications.ntfy.notify_on_success),
        ntfy_notify_on_error = checked(config.notifications.ntfy.notify_on_error),
        ntfy_notify_on_every_error = checked(config.notifications.ntfy.notify_on_every_error),
        ntfy_notify_after_attempts = config.notifications.ntfy.notify_after_attempts,
    )
}

pub fn settings_page_html(config: &Config) -> String {
    let notifications_enabled = if config.notifications.enabled {
        r#"<span class="badge ok">aktiv</span>"#
    } else {
        r#"<span class="badge muted">aus</span>"#
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

    let notify_on_worker_start = if config.notifications.ntfy.notify_on_worker_start {
        r#"<span class="badge ok">aktiv</span>"#
    } else {
        r#"<span class="badge muted">aus</span>"#
    };

    let notify_on_processing_start = if config.notifications.ntfy.notify_on_processing_start {
        r#"<span class="badge ok">aktiv</span>"#
    } else {
        r#"<span class="badge muted">aus</span>"#
    };

    let notify_on_success = if config.notifications.ntfy.notify_on_success {
        r#"<span class="badge ok">aktiv</span>"#
    } else {
        r#"<span class="badge muted">aus</span>"#
    };

    let notify_on_error = if config.notifications.ntfy.notify_on_error {
        r#"<span class="badge ok">aktiv</span>"#
    } else {
        r#"<span class="badge muted">aus</span>"#
    };

    let notify_on_every_error = if config.notifications.ntfy.notify_on_every_error {
        r#"<span class="badge warn">aktiv</span>"#
    } else {
        r#"<span class="badge muted">aus</span>"#
    };

    let web_enabled = if config.web.enabled {
        r#"<span class="badge ok">aktiv</span>"#
    } else {
        r#"<span class="badge muted">aus</span>"#
    };

    let ntfy_server_configured = if config.notifications.ntfy.server.trim().is_empty() {
        r#"<span class="badge bad">nein</span>"#
    } else {
        r#"<span class="badge ok">ja</span>"#
    };

    let ntfy_topic_configured = if config.notifications.ntfy.topic.trim().is_empty() {
        r#"<span class="badge bad">nein</span>"#
    } else {
        r#"<span class="badge ok">ja</span>"#
    };

    let token_configured = if config.notifications.ntfy.token.trim().is_empty() {
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
<link rel="stylesheet" href="/assets/common.css">
<link rel="stylesheet" href="/assets/settings.css">
</head>
<body>
<main>
  <h1>Einstellungen</h1>

    <div class="actions nav" style="margin-top: 22px; margin-bottom: 30px;">
    <a class="button" href="/">Dashboard</a>
    <a class="button" href="/settings">Einstellungen</a>
    <a class="button" href="/settings/edit">Bearbeiten</a>
    <a class="button" href="/logs">Logs</a>
    <a class="button" href="/diagnostics">Diagnose</a>
  </div>

  <div class="grid">
    <section class="card">
      <h2>Überwachung</h2>
      <div class="row">
        <div class="key">
          Überwachte Ordner
        </div>

        <div class="value">
          <div class="watch-directory-list">
            {watch_directory_rows}
          </div>
        </div>
      </div>
      <div class="row"><div class="key">Wartezeit bis Verarbeitung</div><div class="value">{stable_after}s</div></div>
      <div class="row"><div class="key">Archive im Hauptverzeichnis erlauben</div><div class="value">{allow_root_archives}</div></div>
    </section>

    <section class="card">
      <h2>Entpacken</h2>
      <div class="row"><div class="key">Archive nach Erfolg löschen</div><div class="value">{delete_archives}</div></div>
      <div class="row"><div class="key">Fehlerhafte Archive behalten</div><div class="value">{keep_failed}</div></div>
      <div class="row"><div class="key">Passwortliste konfiguriert</div><div class="value">{password_file_configured}</div></div>
      <div class="row"><div class="key">Pfad zur Passwortliste</div><div class="value"><code>{password_file}</code></div></div>
    </section>

    <section class="card">
      <h2>Ausgabe / Verlauf</h2>
      <div class="row"><div class="key">Ausgabeordner</div><div class="value"><code>{output_dir}</code></div></div>
      <div class="row"><div class="key">Verlaufsordner</div><div class="value"><code>{history_dir}</code></div></div>
    </section>

    <section class="card">
      <h2>Wiederholung / Start</h2>
      <div class="row"><div class="key">Erste Wiederholung nach</div><div class="value">{base_delay}s</div></div>
      <div class="row"><div class="key">Maximale Wiederholungs-Wartezeit</div><div class="value">{max_delay}s</div></div>
      <div class="row"><div class="key">Vorhandene Releases beim Start scannen</div><div class="value">{startup_scan}</div></div>
    </section>

    <section class="card">
      <h2>ntfy</h2>
      <div class="row"><div class="key">Benachrichtigungen aktiv</div><div class="value">{notifications_enabled}</div></div>
      <div class="row"><div class="key">ntfy-Server konfiguriert</div><div class="value">{ntfy_server_configured}</div></div>
      <div class="row"><div class="key">Thema (Topic) konfiguriert</div><div class="value">{ntfy_topic_configured}</div></div>
      <div class="row"><div class="key">Token konfiguriert</div><div class="value">{token_configured}</div></div>
      <div class="row"><div class="key">Priorität bei Erfolg</div><div class="value">{priority_success}</div></div>
      <div class="row"><div class="key">Priorität bei Fehler</div><div class="value">{priority_error}</div></div>
      <div class="row"><div class="key">Worker-Start melden</div><div class="value">{notify_on_worker_start}</div></div>
      <div class="row"><div class="key">Verarbeitungsbeginn melden</div><div class="value">{notify_on_processing_start}</div></div>
      <div class="row"><div class="key">Erfolgreiche Verarbeitung melden</div><div class="value">{notify_on_success}</div></div>
      <div class="row"><div class="key">Fehler melden</div><div class="value">{notify_on_error}</div></div>
      <div class="row"><div class="key">Jeden Fehler melden</div><div class="value">{notify_on_every_error}</div></div>
      <div class="row"><div class="key">Fehler melden nach Versuchen</div><div class="value">{notify_after_attempts}</div></div>
    </section>

    <section class="card">
      <h2>WebUI</h2>
      <div class="row"><div class="key">WebUI aktiv</div><div class="value">{web_enabled}</div></div>
      <div class="row"><div class="key">Adresse / Port</div><div class="value"><code>{web_bind}</code></div></div>
    </section>

  </div>

  <footer>
    Diese Seite zeigt die aktuell geladene Konfiguration. Änderungen können im Bereich Bearbeiten vorgenommen werden und werden nach einem Neustart des Workers aktiv.
  </footer>
  <footer class="page-version">
    Version {version}
  </footer>
</main>
</body>
</html>"#,
        version = env!("CARGO_PKG_VERSION"),
        watch_directory_rows = watch_directory_list_html(config),
        stable_after = config.watch.stable_after,
        allow_root_archives = allow_root_archives,
        delete_archives = delete_archives,
        keep_failed = keep_failed,
        password_file_configured = password_file_configured,
        password_file = escape_html(&config.extract.password_file),
        output_dir = escape_html(&config.output.directory),
        history_dir = escape_html(&config.history.directory),
        base_delay = config.retry.base_delay,
        max_delay = config.retry.max_delay,
        startup_scan = startup_scan,
        notifications_enabled = notifications_enabled,
        ntfy_server_configured = ntfy_server_configured,
        ntfy_topic_configured = ntfy_topic_configured,
        token_configured = token_configured,
        priority_success = config.notifications.ntfy.priority_success,
        priority_error = config.notifications.ntfy.priority_error,
        notify_on_worker_start = notify_on_worker_start,
        notify_on_processing_start = notify_on_processing_start,
        notify_on_success = notify_on_success,
        notify_on_error = notify_on_error,
        notify_on_every_error = notify_on_every_error,
        notify_after_attempts = config.notifications.ntfy.notify_after_attempts,
        web_enabled = web_enabled,
        web_bind = escape_html(&config.web.bind),
    );

    html
}

pub fn dashboard_page_html(config: &Config) -> String {
    let history = crate::web_history::history_counts(&config.history.directory);
    let scan_html = scan_summary_html(config);
    let failures_html = failures_html(config);

    let ntfy_badge = if config.notifications.enabled {
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
<link rel="stylesheet" href="/assets/common.css">
<link rel="stylesheet" href="/assets/dashboard.css">
</head>
<body>
<main>
  <h1>XDCC Extractor</h1>
  <div class="actions nav" style="margin-top: 22px; margin-bottom: 30px;">
    <a class="button" href="/">Dashboard</a>
    <a class="button" href="/settings">Einstellungen</a>
    <a class="button" href="/settings/edit">Bearbeiten</a>
    <a class="button" href="/logs">Logs</a>
    <a class="button" href="/diagnostics">Diagnose</a>
  </div>

  <div
    id="toast-region"
    class="toast-region"
    aria-live="polite"
    aria-atomic="true"
  ></div>

  <div
    id="auto-refresh-status"
    class="auto-refresh-status"
    role="status"
    aria-live="polite"
  >
    Automatische Aktualisierung: alle 30 Sekunden.
  </div>

  <div class="dashboard-overview">
    <section class="card dashboard-primary-card">
      <h2>Worker &amp; System</h2>
      <div class="value">läuft</div>
      <div class="small">WebUI erreichbar</div>
      <div class="dashboard-card-divider"></div>
      <div class="small dashboard-card-label">Systemstatus</div>
      <div class="value dashboard-secondary-value">bereit</div>
      <div class="small">WebUI geschützt</div>
      <div class="small">Healthcheck aktiv</div>
    </section>

    <div class="dashboard-compact-grid">
      <section class="card">
        <h2>ntfy</h2>
        <div class="value">{ntfy_badge}</div>
      </section>

      <section class="card">
        <h2>Überwachter Ordner</h2>
        <div class="small">
          <div class="watch-directory-list">
            {watch_directory_rows}
          </div>
        </div>
      </section>

      <section class="card">
        <h2>Verlauf</h2>
        <div class="value history-counts">
          <div>{done} erledigt</div>
          <div>{failed} fehlgeschlagen</div>
        </div>
      </section>

      <section class="card">
        <h2>Ausgabeordner</h2>
        <div class="small"><code>{output_dir}</code></div>
      </section>
    </div>
  </div>

  <div class="grid dashboard-content-grid">
    <section class="card wide">
      <div class="card-head">
        <h2>Releases</h2>
        <button id="refresh-scan" class="button" type="button">Releases neu prüfen</button>
      </div>
      <div id="scan-content">
        {scan_html}
      </div>
    </section>

    <section class="card wide">
      <div class="card-head">
        <h2>Letzte Fehler</h2>
        <button id="refresh-failures" class="button" type="button">Fehlerliste aktualisieren</button>
      </div>
      <div id="failure-content">
        {failures_html}
      </div>
    </section>
  </div>

  <footer>
    Version {version}
  </footer>
</main>
<script src="/assets/app.js"></script>
</body>
</html>"#,
        version = env!("CARGO_PKG_VERSION"),
        ntfy_badge = ntfy_badge,
        done = history.0,
        failed = history.1,
        watch_directory_rows = watch_directory_list_html(config),
        output_dir = escape_html(&config.output.directory),
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
                r#"<div class="error">Releases konnten nicht geprüft werden: {}</div>"#,
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
<span class="badge ok">neu: {}</span>
<span class="badge muted">erledigt: {}</span>
<span class="badge bad">fehlgeschlagen: {}</span>
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
            r#"<div class="small">Weitere {} Einträge werden nicht angezeigt.</div>"#,
            candidates.len() - 25
        ));
    }

    html.push_str("</div>");
    html
}

fn action_button_html(state: &str, path: &str) -> String {
    match state {
        "failed" => format!(
            r#"<button class="button small-button danger-button" type="button" data-action="clear-failed" data-path="{}">Fehlerstatus zurücksetzen</button>"#,
            escape_html(path)
        ),
        "new" => format!(
            r#"<button class="button small-button" type="button" data-action="process" data-path="{}">Jetzt verarbeiten</button>"#,
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
        r#"<div class="scan-summary"><span class="badge bad">fehlgeschlagen: {}</span></div>"#,
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
<div class="scan-actions"><button class="button small-button danger-button" type="button" data-action="clear-failed" data-path="{}">Fehlerstatus zurücksetzen</button></div>
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

pub fn diagnostics_page_html(config: &Config) -> String {
    let availability_badge = |available: bool| -> String {
        if available {
            r#"<span class="badge ok">erreichbar</span>"#.to_string()
        } else {
            r#"<span class="badge bad">nicht erreichbar</span>"#.to_string()
        }
    };

    let yes_no_badge = |configured: bool| -> String {
        if configured {
            r#"<span class="badge ok">ja</span>"#.to_string()
        } else {
            r#"<span class="badge muted">nein</span>"#.to_string()
        }
    };

    let history_status = availability_badge(Path::new(&config.history.directory).is_dir());

    let password_configured = !config.extract.password_file.trim().is_empty();

    let password_exists = password_configured && Path::new(&config.extract.password_file).is_file();

    let password_status = if !password_configured {
        r#"<span class="badge muted">nicht konfiguriert</span>"#
    } else if password_exists {
        r#"<span class="badge ok">vorhanden</span>"#
    } else {
        r#"<span class="badge bad">nicht gefunden</span>"#
    };

    let password_path = if password_configured {
        escape_html(&config.extract.password_file)
    } else {
        "—".to_string()
    };

    let ntfy_server_configured = !config.notifications.ntfy.server.trim().is_empty();

    let ntfy_token_configured = !config.notifications.ntfy.token.trim().is_empty();

    let ntfy_topic_configured = !config.notifications.ntfy.topic.trim().is_empty();

    let ntfy_status = if !config.notifications.enabled {
        r#"<span class="badge muted">aus</span>"#
    } else if ntfy_server_configured && ntfy_topic_configured {
        r#"<span class="badge ok">bereit</span>"#
    } else {
        r#"<span class="badge bad">unvollständig</span>"#
    };

    let web_status = if config.web.enabled {
        r#"<span class="badge ok">aktiv</span>"#
    } else {
        r#"<span class="badge muted">aus</span>"#
    };

    let history = crate::web_history::history_counts(&config.history.directory);

    let backups = crate::web_backups::backup_overview(config);

    let backup_card_html = |title: &str, summary: &crate::web_backups::BackupSummary| -> String {
        let status = if !summary.directory_exists {
            r#"<span class="badge muted">noch nicht angelegt</span>"#
        } else if !summary.readable {
            r#"<span class="badge bad">nicht lesbar</span>"#
        } else {
            r#"<span class="badge ok">bereit</span>"#
        };

        let count_badge = if summary.count == 0 {
            r#"<span class="badge muted">0</span>"#.to_string()
        } else {
            format!(r#"<span class="badge ok">{}</span>"#, summary.count)
        };

        let latest = match (
            summary.latest_name.as_deref(),
            summary.latest_age.as_deref(),
        ) {
            (Some(name), Some(age)) => {
                format!("{}<br><code>{}</code>", escape_html(age), escape_html(name),)
            }
            _ => "noch keine Sicherung".to_string(),
        };

        format!(
            r#"<section class="card">
      <h2>{title}</h2>

      <div class="row">
        <div class="key">Status</div>
        <div class="value">{status}</div>
      </div>

      <div class="row">
        <div class="key">Anzahl</div>
        <div class="value">{count_badge}</div>
      </div>

      <div class="row">
        <div class="key">Letzte Sicherung</div>
        <div class="value">{latest}</div>
      </div>

      <div class="row">
        <div class="key">Speicherort</div>
        <div class="value"><code>{directory}</code></div>
      </div>
    </section>"#,
            title = escape_html(title),
            status = status,
            count_badge = count_badge,
            latest = latest,
            directory = escape_html(&summary.directory.display().to_string()),
        )
    };

    let backup_cards_html = [
        backup_card_html("Konfigurations-Sicherungen", &backups.config),
        backup_card_html("Verlaufs-Sicherungen", &backups.history),
        backup_card_html("Passwortlisten-Sicherungen", &backups.passwords),
    ]
    .join("\n");

    let disk_card_html = |title: &str, path: &str| -> String {
        match crate::web_disk::disk_usage(Path::new(path)) {
            Ok(usage) => {
                let css_class = usage.level.css_class();

                let meter_percent = usage.used_percent.clamp(0.0, 100.0);

                format!(
                    r#"<section class="card">
      <h2>{title}</h2>

      <div class="row">
        <div class="key">Erreichbarkeit</div>
        <div class="value">
          <span class="badge ok">
            erreichbar
          </span>
        </div>
      </div>

      <div class="row">
        <div class="key">Speicherstatus</div>
        <div class="value">
          <span class="badge {css_class}">
            {status_label}
          </span>
        </div>
      </div>

      <div class="row">
        <div class="key">Gesamt</div>
        <div class="value">{total}</div>
      </div>

      <div class="row">
        <div class="key">Belegt</div>
        <div class="value">{used}</div>
      </div>

      <div class="row">
        <div class="key">Frei</div>
        <div class="value">{available}</div>
      </div>

      <div class="row">
        <div class="key">Auslastung</div>
        <div class="value">
          {used_percent:.1} %
        </div>
      </div>

      <div
        class="disk-meter"
        role="progressbar"
        aria-label="Speicherauslastung"
        aria-valuemin="0"
        aria-valuemax="100"
        aria-valuenow="{meter_percent:.1}"
      >
        <div
          class="disk-meter-fill {css_class}"
          style="width: {meter_percent:.1}%;"
        ></div>
      </div>

      <div class="small disk-path">
        <code>{path}</code>
      </div>
    </section>"#,
                    title = escape_html(title),
                    css_class = css_class,
                    status_label = usage.level.label(),
                    total = crate::web_disk::format_bytes(usage.total_bytes),
                    used = crate::web_disk::format_bytes(usage.used_bytes),
                    available = crate::web_disk::format_bytes(usage.available_bytes),
                    used_percent = usage.used_percent,
                    meter_percent = meter_percent,
                    path = escape_html(path),
                )
            }
            Err(err) => format!(
                r#"<section class="card">
      <h2>{}</h2>

      <div class="row">
        <div class="key">Status</div>
        <div class="value">
          <span class="badge bad">
            nicht verfügbar
          </span>
        </div>
      </div>

      <div class="small">
        {}
      </div>

      <div class="small disk-path">
        <code>{}</code>
      </div>
    </section>"#,
                escape_html(title),
                escape_html(&format!("{err:#}")),
                escape_html(path),
            ),
        }
    };

    let watch_directories = config.watch.resolved_directories();

    let mut disk_cards = Vec::new();

    for (index, directory) in watch_directories.iter().enumerate() {
        let title = if watch_directories.len() == 1 {
            "Überwachter Ordner".to_string()
        } else {
            format!("Überwachter Ordner {}", index + 1)
        };

        disk_cards.push(disk_card_html(&title, directory));
    }

    disk_cards.push(disk_card_html("Ausgabeordner", &config.output.directory));

    disk_cards.push(disk_card_html(
        "Verlaufs-/State-Ordner",
        &config.history.directory,
    ));

    let disk_cards_html = disk_cards.join("\n");

    format!(
        r#"<!doctype html>
<html lang="de">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>XDCC Extractor Diagnose</title>
<link rel="stylesheet" href="/assets/common.css">
<link rel="stylesheet" href="/assets/settings.css">
</head>
<body>
<main>
  <h1>Diagnose</h1>
  <div class="actions nav" style="margin-top: 22px; margin-bottom: 30px;">
    <a class="button" href="/">Dashboard</a>
    <a class="button" href="/settings">Einstellungen</a>
    <a class="button" href="/settings/edit">Bearbeiten</a>
    <a class="button" href="/logs">Logs</a>
    <a class="button" href="/diagnostics">Diagnose</a>
  </div>

  <div class="grid">
    <section class="card">
      <h2>System</h2>

      <div class="row">
        <div class="key">WebUI</div>
        <div class="value">{web_status}</div>
      </div>

      <div class="row">
        <div class="key">Adresse / Port</div>
        <div class="value"><code>{web_bind}</code></div>
      </div>

    </section>

    <section class="card">
      <h2>Verlauf</h2>

      <div class="row">
        <div class="key">Verlaufsordner</div>
        <div class="value">
          {history_status}<br>
          <code>{history_dir}</code>
        </div>
      </div>

      <div class="row">
        <div class="key">Erledigt</div>
        <div class="value">
          <span class="badge ok">{history_done}</span>
        </div>
      </div>

      <div class="row">
        <div class="key">Fehlgeschlagen</div>
        <div class="value">
          <span class="badge bad">{history_failed}</span>
        </div>
      </div>
    </section>

    <section class="card">
      <h2>Passwortliste</h2>

      <div class="row">
        <div class="key">Status</div>
        <div class="value">{password_status}</div>
      </div>

      <div class="row">
        <div class="key">Pfad</div>
        <div class="value"><code>{password_path}</code></div>
      </div>

      <div class="row">
        <div class="key">Inhalt</div>
        <div class="value">
          <span class="badge muted">bleibt verborgen</span>
        </div>
      </div>
    </section>

    <section class="card">
      <h2>ntfy</h2>

      <div class="row">
        <div class="key">Status</div>
        <div class="value">{ntfy_status}</div>
      </div>

      <div class="row">
        <div class="key">URL konfiguriert</div>
        <div class="value">{ntfy_server_status}</div>
      </div>

      <div class="row">
        <div class="key">Thema (Topic) konfiguriert</div>
        <div class="value">{ntfy_topic_status}</div>
      </div>

      <div class="row">
        <div class="key">Token konfiguriert</div>
        <div class="value">{ntfy_token_status}</div>
      </div>

    </section>

    <section class="card wide">
      <h2>Speicherorte und Speicherplatz</h2>
      <div class="small">
        Die Werte beziehen sich auf das jeweilige Dateisystem.
        Identische Werte sind normal, wenn mehrere Ordner auf
        demselben Datenträger liegen.
      </div>
    </section>

    {disk_cards_html}

    <section class="card wide">
      <h2>Sicherungen</h2>
      <div class="small">
        Es werden nur Anzahl, Zeitpunkt und Speicherort angezeigt.
        Inhalte der Sicherungen bleiben verborgen.
      </div>
    </section>

    {backup_cards_html}
  </div>

  <footer class="page-version">
    Version {version}
  </footer>
</main>
</body>
</html>"#,
        version = env!("CARGO_PKG_VERSION"),
        web_status = web_status,
        web_bind = escape_html(&config.web.bind),
        history_status = history_status,
        history_dir = escape_html(&config.history.directory),
        history_done = history.0,
        history_failed = history.1,
        password_status = password_status,
        password_path = password_path,
        ntfy_status = ntfy_status,
        ntfy_server_status = yes_no_badge(ntfy_server_configured),
        ntfy_topic_status = yes_no_badge(ntfy_topic_configured),
        ntfy_token_status = yes_no_badge(ntfy_token_configured),
        disk_cards_html = disk_cards_html,
        backup_cards_html = backup_cards_html,
    )
}

fn watch_directory_list_html(config: &Config) -> String {
    config
        .watch
        .resolved_directories()
        .into_iter()
        .enumerate()
        .map(|(index, directory)| {
            format!(
                r#"<div class="watch-directory-item">
          <span class="watch-directory-index">
            {}
          </span>
          <code>{}</code>
        </div>"#,
                index + 1,
                escape_html(directory),
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}
