use axum::response::IntoResponse;

pub async fn app_js() -> impl IntoResponse {
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
      confirmText = `Fehlerstatus für dieses Release zurücksetzen?\n\n${path}`;
    }

    if (action === 'process') {
      endpoint = '/api/process';
      confirmText = `Dieses Release jetzt manuell verarbeiten?\n\n${path}`;
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
    target.innerHTML = `<div class="error">Releases konnten nicht neu geprüft werden: ${escapeHtml(String(error))}</div>`;
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
    return `<div class="error">Prüfung fehlgeschlagen: ${escapeHtml(data.error || 'Unbekannter Fehler')}</div>`;
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
      <span class="badge muted">erledigt: ${counts.done}</span>
      <span class="badge bad">fehlgeschlagen: ${counts.failed}</span>
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
    html += `<div class="small">Weitere ${candidates.length - 25} Einträge werden nicht angezeigt.</div>`;
  }

  html += `</div>`;

  return html;
}

function renderFailures(data) {
  if (!data.ok) {
    return `<div class="error">Fehlerliste konnte nicht geladen werden: ${escapeHtml(data.error || 'Unbekannter Fehler')}</div>`;
  }

  const failures = data.failures || [];

  let html = `
    <div class="scan-summary">
      <span class="badge bad">fehlgeschlagen: ${failures.length}</span>
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
          <button class="button small-button danger-button" type="button" data-action="clear-failed" data-path="${escapeHtml(path)}">Fehlerstatus zurücksetzen</button>
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
    return `<button class="button small-button danger-button" type="button" data-action="clear-failed" data-path="${escapedPath}">Fehlerstatus zurücksetzen</button>`;
  }

  if (state === 'new') {
    return `<button class="button small-button" type="button" data-action="process" data-path="${escapedPath}">Jetzt verarbeiten</button>`;
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
