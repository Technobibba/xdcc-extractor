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
