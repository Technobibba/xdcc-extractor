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
      await refreshScan(refreshScanButton, true);
    });
  }

  if (refreshFailuresButton) {
    refreshFailuresButton.addEventListener('click', async () => {
      await refreshFailures(refreshFailuresButton, true);
    });
  }

  document.addEventListener('click', async (event) => {
    const target = event.target;

    if (!(target instanceof Element)) {
      return;
    }

    const button = target.closest(
      'button[data-action][data-path]'
    );

    if (!button || button.disabled) {
      return;
    }

    const action = button.dataset.action;
    const path = button.dataset.path;

    if (!action || !path) {
      return;
    }

    let endpoint = null;
    let confirmText = null;
    let busyText = 'Wird ausgeführt …';
    let successText = 'Aktion erfolgreich abgeschlossen.';

    if (action === 'clear-failed') {
      endpoint = '/api/clear-failed';
      confirmText =
        `Fehlerstatus für dieses Release zurücksetzen?\n\n${path}`;
      busyText = 'Wird zurückgesetzt …';
      successText = 'Der Fehlerstatus wurde zurückgesetzt.';
    }

    if (action === 'process') {
      endpoint = '/api/process';
      confirmText =
        `Dieses Release jetzt manuell verarbeiten?\n\n${path}`;
      busyText = 'Wird verarbeitet …';
      successText = 'Die Verarbeitung wurde gestartet.';
    }

    if (!endpoint) {
      return;
    }

    if (!window.confirm(confirmText)) {
      return;
    }

    setActionButtonsDisabled(true);
    setButtonBusy(button, true, busyText);

    try {
      const data = await fetchJson(endpoint, {
        method: 'POST',
        headers: {
          'content-type': 'application/json'
        },
        body: JSON.stringify({ path })
      });

      if (!data.ok) {
        throw new Error(
          data.error || 'Die Aktion ist fehlgeschlagen.'
        );
      }

      await Promise.all([
        refreshScan(null, false),
        refreshFailures(null, false)
      ]);

      showToast(
        data.message || successText,
        'success'
      );
    } catch (error) {
      showToast(
        `Aktion fehlgeschlagen: ${errorMessage(error)}`,
        'error',
        8000
      );
    } finally {
      setButtonBusy(button, false);
      setActionButtonsDisabled(false);
    }
  });
});

async function refreshScan(
  button = null,
  notify = false
) {
  const target = document.getElementById('scan-content');

  if (!target) {
    return;
  }

  setButtonBusy(
    button,
    true,
    'Releases werden geprüft …'
  );

  try {
    const data = await fetchJson(
      '/api/scan',
      { cache: 'no-store' }
    );

    if (!data.ok) {
      throw new Error(
        data.error || 'Unbekannter Fehler'
      );
    }

    target.innerHTML = renderScan(data);

    if (notify) {
      showToast(
        'Die Release-Liste wurde aktualisiert.',
        'success'
      );
    }
  } catch (error) {
    const message = errorMessage(error);

    target.innerHTML =
      `<div class="error">` +
      `Releases konnten nicht neu geprüft werden: ` +
      `${escapeHtml(message)}</div>`;

    if (notify) {
      showToast(
        `Releases konnten nicht aktualisiert werden: ${message}`,
        'error',
        8000
      );
    }
  } finally {
    setButtonBusy(button, false);
  }
}

async function refreshFailures(
  button = null,
  notify = false
) {
  const target = document.getElementById(
    'failure-content'
  );

  if (!target) {
    return;
  }

  setButtonBusy(
    button,
    true,
    'Fehlerliste wird geladen …'
  );

  try {
    const data = await fetchJson(
      '/api/failures',
      { cache: 'no-store' }
    );

    if (!data.ok) {
      throw new Error(
        data.error || 'Unbekannter Fehler'
      );
    }

    target.innerHTML = renderFailures(data);

    if (notify) {
      showToast(
        'Die Fehlerliste wurde aktualisiert.',
        'success'
      );
    }
  } catch (error) {
    const message = errorMessage(error);

    target.innerHTML =
      `<div class="error">` +
      `Fehlerliste konnte nicht aktualisiert werden: ` +
      `${escapeHtml(message)}</div>`;

    if (notify) {
      showToast(
        `Fehlerliste konnte nicht aktualisiert werden: ${message}`,
        'error',
        8000
      );
    }
  } finally {
    setButtonBusy(button, false);
  }
}

async function fetchJson(
  url,
  options = {}
) {
  const response = await fetch(url, options);
  const body = await response.text();

  let data = {};

  if (body.trim()) {
    try {
      data = JSON.parse(body);
    } catch (_error) {
      throw new Error(
        'Der Server hat eine ungültige Antwort geliefert.'
      );
    }
  }

  if (!response.ok) {
    throw new Error(
      data.error || `HTTP ${response.status}`
    );
  }

  return data;
}

function setButtonBusy(
  button,
  busy,
  busyText = 'Bitte warten …'
) {
  if (!button) {
    return;
  }

  if (busy) {
    if (!button.dataset.originalText) {
      button.dataset.originalText =
        button.textContent || '';
    }

    button.disabled = true;
    button.classList.add('is-loading');
    button.textContent = busyText;
    return;
  }

  button.disabled = false;
  button.classList.remove('is-loading');

  if (button.dataset.originalText) {
    button.textContent =
      button.dataset.originalText;

    delete button.dataset.originalText;
  }
}

function setActionButtonsDisabled(disabled) {
  document
    .querySelectorAll(
      'button[data-action][data-path]'
    )
    .forEach((button) => {
      button.disabled = disabled;
    });
}

function showToast(
  message,
  type = 'success',
  duration = 5000
) {
  let region = document.getElementById(
    'toast-region'
  );

  if (!region) {
    region = document.createElement('div');
    region.id = 'toast-region';
    region.className = 'toast-region';
    region.setAttribute('aria-live', 'polite');
    region.setAttribute('aria-atomic', 'true');

    document.body.appendChild(region);
  }

  const toast = document.createElement('div');
  toast.className = `toast toast-${type}`;
  toast.setAttribute(
    'role',
    type === 'error' ? 'alert' : 'status'
  );

  const text = document.createElement('span');
  text.className = 'toast-text';
  text.textContent = message;

  const close = document.createElement('button');
  close.className = 'toast-close';
  close.type = 'button';
  close.textContent = '×';
  close.setAttribute(
    'aria-label',
    'Meldung schließen'
  );

  const remove = () => {
    if (!toast.isConnected) {
      return;
    }

    toast.classList.add('is-hiding');

    window.setTimeout(() => {
      toast.remove();
    }, 180);
  };

  close.addEventListener('click', remove);

  toast.append(text, close);
  region.appendChild(toast);

  window.setTimeout(remove, duration);
}

function renderScan(data) {
  const candidates = data.candidates || [];

  const counts = {
    new: 0,
    done: 0,
    failed: 0
  };

  for (const candidate of candidates) {
    if (candidate.state === 'new') {
      counts.new += 1;
    }

    if (candidate.state === 'done') {
      counts.done += 1;
    }

    if (candidate.state === 'failed') {
      counts.failed += 1;
    }
  }

  let html = `
    <div class="scan-summary">
      <span class="badge ok">
        neu: ${counts.new}
      </span>
      <span class="badge muted">
        erledigt: ${counts.done}
      </span>
      <span class="badge bad">
        fehlgeschlagen: ${counts.failed}
      </span>
      <span class="badge muted">
        gesamt: ${candidates.length}
      </span>
    </div>
    <div class="small">
      Aktualisiert:
      ${escapeHtml(new Date().toLocaleTimeString())}
    </div>
  `;

  if (candidates.length === 0) {
    html +=
      `<div class="small">` +
      `Keine Kandidaten gefunden.` +
      `</div>`;

    return html;
  }

  html += `<div class="scan-list">`;

  for (const candidate of candidates.slice(0, 25)) {
    const state =
      candidate.state || 'unknown';

    const cssClass = stateClass(state);
    const path = candidate.path || '';

    html += `
      <div class="scan-row">
        <span class="badge ${cssClass}">
          ${escapeHtml(releaseStatusLabel(state))}
        </span>
        <span class="scan-path">
          ${escapeHtml(path)}
        </span>
        <span class="scan-actions">
          ${actionButton(state, path)}
        </span>
      </div>
    `;
  }

  if (candidates.length > 25) {
    html +=
      `<div class="small">` +
      `Weitere ${candidates.length - 25} ` +
      `Einträge werden nicht angezeigt.` +
      `</div>`;
  }

  html += `</div>`;

  return html;
}

function renderFailures(data) {
  const failures = data.failures || [];

  let html = `
    <div class="scan-summary">
      <span class="badge bad">
        fehlgeschlagen: ${failures.length}
      </span>
      <span class="badge muted">
        Aktualisiert:
        ${escapeHtml(new Date().toLocaleTimeString())}
      </span>
    </div>
  `;

  if (failures.length === 0) {
    html +=
      `<div class="small">` +
      `Keine fehlgeschlagenen Releases vorhanden.` +
      `</div>`;

    return html;
  }

  html += `<div class="failure-list">`;

  for (const failure of failures) {
    const path = failure.path || '';

    html += `
      <div class="failure-row">
        <div>
          <span class="badge bad">
            ${escapeHtml(
              failure.error_class || 'fehlgeschlagen'
            )}
          </span>
          <span class="small">
            Fehlversuche:
            ${escapeHtml(
              String(failure.attempts || 0)
            )}
          </span>
        </div>
        <div class="scan-path">
          ${escapeHtml(path)}
        </div>
        <div class="small">
          ${escapeHtml(
            failure.reason ||
            'Kein Grund gefunden'
          )}
        </div>
        <div class="scan-actions">
          <button
            class="button small-button danger-button"
            type="button"
            data-action="clear-failed"
            data-path="${escapeHtml(path)}"
          >
            Fehlerstatus zurücksetzen
          </button>
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
    return `
      <button
        class="button small-button danger-button"
        type="button"
        data-action="clear-failed"
        data-path="${escapedPath}"
      >
        Fehlerstatus zurücksetzen
      </button>
    `;
  }

  if (state === 'new') {
    return `
      <button
        class="button small-button"
        type="button"
        data-action="process"
        data-path="${escapedPath}"
      >
        Jetzt verarbeiten
      </button>
    `;
  }

  return '';
}

function releaseStatusLabel(state) {
  if (state === 'new') {
    return 'neu';
  }

  if (state === 'done') {
    return 'erledigt';
  }

  if (state === 'failed') {
    return 'fehlgeschlagen';
  }

  return state;
}

function stateClass(state) {
  if (state === 'new') {
    return 'ok';
  }

  if (state === 'failed') {
    return 'bad';
  }

  return 'muted';
}

function errorMessage(error) {
  if (error instanceof Error) {
    return error.message;
  }

  return String(error);
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
