use axum::{http::header, response::IntoResponse};

const COMMON_CSS: &str = r###"code {
  color: #cfd7e6;
}

.watch-directory-list {
  display: grid;
  gap: 8px;
}

.watch-directory-item {
  display: flex;
  align-items: flex-start;
  gap: 9px;
  min-width: 0;
}

.watch-directory-index {
  display: inline-flex;
  flex: 0 0 auto;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border-radius: 999px;
  background: rgba(154, 164, 178, .12);
  color: #9aa4b2;
  font-size: 12px;
  font-weight: 700;
}

.watch-directory-item code {
  min-width: 0;
  padding-top: 2px;
  overflow-wrap: anywhere;
  word-break: normal;
}

.page-version {
  margin-top: 28px;
  color: var(--muted);
  font-size: 13px;
}
"###;

pub async fn common_css() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        COMMON_CSS,
    )
}

const LOGS_CSS: &str = r###":root {
  --bg: #11131a;
  --card: #181b24;
  --text: #e7ecf4;
  --muted: #9aa4b2;
  --border: #2a3040;
}

* {
  box-sizing: border-box;
}

body {
  margin: 0;
  background: var(--bg);
  color: var(--text);
  font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
}

main {
  max-width: 1200px;
  margin: 0 auto;
  padding: 28px;
}

h1 {
  margin: 0 0 6px;
}

.sub {
  color: var(--muted);
  margin-bottom: 18px;
}

.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  margin-bottom: 18px;
}

.actions.nav {
  margin-top: 22px;
  margin-bottom: 28px;
}

.button {
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
}

.button:hover {
  background: #2b3142;
}

.card {
  background: var(--card);
  border: 1px solid var(--border);
  border-radius: 16px;
  padding: 18px;
}

.logbox {
  min-height: 520px;
  max-height: 70vh;
  overflow: auto;
  white-space: pre-wrap;
  word-break: break-word;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", monospace;
  font-size: 13px;
  line-height: 1.45;
  color: #d8dee9;
}

.small {
  color: var(--muted);
  font-size: 13px;
  margin-top: 12px;
}"###;

pub async fn logs_css() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        LOGS_CSS,
    )
}

const SETTINGS_EDIT_CSS: &str = r###":root {
  color-scheme: dark;
  --bg: #0f1115;
  --panel: #171a21;
  --text: #e6e6e6;
  --muted: #9aa4b2;
  --border: #2a2f3a;
  --ok: #25c26e;
}

body {
  margin: 0;
  font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  background: var(--bg);
  color: var(--text);
}

main {
  max-width: 900px;
  margin: 0 auto;
  padding: 32px 20px;
}

h1 {
  margin: 0 0 6px;
}

.sub {
  color: var(--muted);
  margin-bottom: 22px;
}

.card, .notice {
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 14px;
  padding: 18px;
  margin-bottom: 14px;
}

.notice {
  border-color: var(--ok);
}

.grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 14px;
}

label {
  display: block;
  color: var(--muted);
  font-size: 14px;
  margin-bottom: 6px;
}

input[type="number"],
input[type="text"],
input[type="url"],
input[type="password"],
textarea {
  width: 100%;
  min-width: 0;
  padding: 10px;
  border-radius: 10px;
  border: 1px solid var(--border);
  background: #11131a;
  color: var(--text);
}

.field {
  min-width: 0;
}

.field.full {
  grid-column: 1 / -1;
}

.settings-group {
  margin-top: 18px;
  padding-top: 4px;
}

.settings-group + .settings-group {
  padding-top: 18px;
  border-top: 1px solid var(--border);
}

.settings-group h3 {
  margin: 0 0 14px;
}

.settings-group .field .small {
  margin-top: 7px;
  line-height: 1.45;
}

.connection-test .button {
  margin-top: 16px;
}

.check {
  display: flex;
  gap: 10px;
  align-items: center;
  color: var(--text);
  margin: 10px 0;
}

.actions {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
  margin: 18px 0;
}

.button {
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
}

button.button {
  background: #284f38;
}

.button.danger {
  background: #4f2828;
}

textarea {
  min-height: 120px;
  resize: vertical;
}

.small {
  color: var(--muted);
  font-size: 13px;
}

.ntfy-priority-grid {
  margin-top: 16px;
}


/* Verhindert, dass Eingabefelder durch Innenabstände über Karten hinausragen. */
*,
*::before,
*::after {
  box-sizing: border-box;
}

/* Grid-Inhalte dürfen innerhalb ihrer Spalte korrekt schrumpfen. */
.grid > * {
  min-width: 0;
}

/* Zahlenfelder kompakt darstellen. */
input[type="number"] {
  width: 180px;
  max-width: 100%;
}

@media (max-width: 720px) {
  .grid {
    grid-template-columns: 1fr;
  }
}

.notice {
  margin: 0 0 20px;
  padding: 14px 16px;
  border: 1px solid var(--border);
  border-radius: 12px;
  line-height: 1.5;
}

.notice-success {
  border-color: rgba(37, 194, 110, .5);
  background: rgba(37, 194, 110, .08);
}

.notice-error {
  border-color: rgba(255, 92, 92, .55);
  background: rgba(255, 92, 92, .08);
}

.notice.restart-required {
  border-color: rgba(240, 160, 32, .6);
  background: rgba(240, 160, 32, .1);
  color: var(--text);
}

.notice.restart-required::before {
  content: "Neustart erforderlich";
  display: block;
  margin-bottom: 5px;
  color: var(--warn);
  font-weight: 800;
}

.button.is-loading {
  position: relative;
}

.button.is-loading::before {
  content: "";
  display: inline-block;
  width: 13px;
  height: 13px;
  margin-right: 8px;
  border: 2px solid currentColor;
  border-right-color: transparent;
  border-radius: 50%;
  vertical-align: -2px;
  animation: settings-button-spin .7s linear infinite;
}

@keyframes settings-button-spin {
  to {
    transform: rotate(360deg);
  }
}

button.button:disabled {
  opacity: .65;
  cursor: wait;
}

.restart-status {
  min-height: 20px;
  margin: -6px 0 18px;
  color: var(--muted);
  font-size: 14px;
}

.restart-status.restart-pending {
  color: var(--warn);
  font-weight: 700;
}
"###;

pub async fn settings_edit_css() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        SETTINGS_EDIT_CSS,
    )
}

const SETTINGS_CSS: &str = r###":root {
  color-scheme: dark;
  --bg: #0f1115;
  --panel: #171a21;
  --text: #e6e6e6;
  --muted: #9aa4b2;
  --border: #2a2f3a;
  --ok: #25c26e;
  --warn: #f0a020;
  --bad: #ff5c5c;
}

body {
  margin: 0;
  font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  background: var(--bg);
  color: var(--text);
}

main {
  max-width: 1100px;
  margin: 0 auto;
  padding: 32px 20px;
}

h1 {
  margin: 0 0 6px;
  font-size: 32px;
}

.sub {
  color: var(--muted);
  margin-bottom: 28px;
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 14px;
}

.card {
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 14px;
  padding: 18px;
}

.card.wide {
  grid-column: 1 / -1;
}

.card h2 {
  margin: 0 0 12px;
  font-size: 16px;
  color: var(--muted);
  font-weight: 600;
}

.row {
  display: grid;
  grid-template-columns: 180px 1fr;
  gap: 12px;
  padding: 8px 0;
  border-top: 1px solid var(--border);
}

.row:first-of-type {
  border-top: 0;
}

.key {
  color: var(--muted);
  font-size: 14px;
}

.value {
  font-size: 14px;
  word-break: break-word;
}

.badge {
  display: inline-block;
  padding: 5px 10px;
  border-radius: 999px;
  font-weight: 700;
  font-size: 14px;
}

.badge.ok {
  background: rgba(37, 194, 110, .15);
  color: var(--ok);
}

.badge.warn {
  background: rgba(240, 160, 32, .15);
  color: var(--warn);
}

.badge.bad {
  background: rgba(255, 92, 92, .15);
  color: var(--bad);
}

.badge.muted {
  background: rgba(154, 164, 178, .12);
  color: var(--muted);
}

.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  margin-bottom: 18px;
}

.actions.nav {
  margin-top: 22px;
  margin-bottom: 28px;
}

.button {
  display: inline-block;
  padding: 10px 14px;
  border-radius: 10px;
  border: 1px solid var(--border);
  background: #222735;
  color: var(--text);
  text-decoration: none;
  font-weight: 700;
  font-size: 14px;
}

.button:hover {
  background: #2b3142;
}

footer {
  margin-top: 28px;
  color: var(--muted);
  font-size: 13px;
}

@media (max-width: 720px) {
  .row {
    grid-template-columns: 1fr;
    gap: 4px;
  }
}

.disk-meter {
  width: 100%;
  height: 10px;
  margin-top: 14px;
  overflow: hidden;
  border-radius: 999px;
  background: rgba(154, 164, 178, .14);
}

.disk-meter-fill {
  height: 100%;
  min-width: 2px;
  border-radius: inherit;
  transition: width .2s ease;
}

.disk-meter-fill.ok {
  background: var(--ok);
}

.disk-meter-fill.warn {
  background: var(--warn);
}

.disk-meter-fill.bad {
  background: var(--bad);
}

.disk-path {
  margin-top: 12px;
  word-break: break-all;
}

.value code,
.disk-path code {
  overflow-wrap: anywhere;
  word-break: normal;
}
"###;

pub async fn settings_css() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        SETTINGS_CSS,
    )
}

const DASHBOARD_CSS: &str = r###":root {
  color-scheme: dark;
  --bg: #0f1115;
  --panel: #171a21;
  --text: #e6e6e6;
  --muted: #9aa4b2;
  --border: #2a2f3a;
  --ok: #25c26e;
  --warn: #f0a020;
  --bad: #ff5c5c;
}

body {
  margin: 0;
  font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  background: var(--bg);
  color: var(--text);
}

main {
  max-width: 1100px;
  margin: 0 auto;
  padding: 32px 20px;
}

h1 {
  margin: 0 0 6px;
  font-size: 32px;
}

.sub {
  color: var(--muted);
  margin-bottom: 28px;
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(230px, 1fr));
  gap: 14px;
}

.card {
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 14px;
  padding: 18px;
}

.card.wide {
  grid-column: 1 / -1;
}

.card h2 {
  margin: 0 0 12px;
  font-size: 16px;
  color: var(--muted);
  font-weight: 600;
}

.value {
  font-size: 22px;
  font-weight: 700;
  word-break: break-word;
}

.small {
  font-size: 14px;
  color: var(--muted);
  word-break: break-word;
}

.badge {
  display: inline-block;
  padding: 5px 10px;
  border-radius: 999px;
  font-weight: 700;
  font-size: 14px;
}

.badge.ok {
  background: rgba(37, 194, 110, .15);
  color: var(--ok);
}

.badge.warn {
  background: rgba(240, 160, 32, .15);
  color: var(--warn);
}

.badge.bad {
  background: rgba(255, 92, 92, .15);
  color: var(--bad);
}

.badge.muted {
  background: rgba(154, 164, 178, .12);
  color: var(--muted);
}

.scan-summary {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
  margin-bottom: 12px;
}

.scan-list {
  display: grid;
  gap: 8px;
}

.scan-row {
  display: grid;
  grid-template-columns: 88px 1fr auto;
  gap: 10px;
  align-items: center;
  padding: 8px 0;
  border-top: 1px solid var(--border);
}

.scan-path {
  color: var(--text);
  font-size: 14px;
  word-break: break-all;
}

.scan-actions {
  text-align: right;
}

.failure-list {
  display: grid;
  gap: 12px;
}

.failure-row {
  display: grid;
  gap: 8px;
  padding: 12px 0;
  border-top: 1px solid var(--border);
}

.error {
  color: var(--bad);
}

.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.button {
  display: inline-block;
  padding: 10px 14px;
  border-radius: 10px;
  border: 1px solid var(--border);
  background: #222735;
  color: var(--text);
  text-decoration: none;
  font-weight: 700;
  font-size: 14px;
}

.button:hover {
  background: #2b3142;
}

button.button {
  cursor: pointer;
  font-family: inherit;
}

button.button:disabled {
  opacity: .65;
  cursor: wait;
}

.small-button {
  padding: 7px 10px;
  font-size: 13px;
}

.danger-button {
  border-color: rgba(255, 92, 92, .35);
}

.card-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
}

.card-head h2 {
  margin: 0;
}

footer {
  margin-top: 28px;
  color: var(--muted);
  font-size: 13px;
}

@media (max-width: 720px) {
  .scan-row {
    grid-template-columns: 1fr;
  }
  .scan-actions {
    text-align: left;
  }
}

.button.is-loading {
  position: relative;
}

.button.is-loading::before {
  content: "";
  display: inline-block;
  width: 13px;
  height: 13px;
  margin-right: 8px;
  border: 2px solid currentColor;
  border-right-color: transparent;
  border-radius: 50%;
  vertical-align: -2px;
  animation: button-spin .7s linear infinite;
}

@keyframes button-spin {
  to {
    transform: rotate(360deg);
  }
}

.toast-region {
  position: fixed;
  top: 18px;
  right: 18px;
  z-index: 1000;
  display: grid;
  gap: 10px;
  width: min(420px, calc(100vw - 36px));
  pointer-events: none;
}

.toast {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 14px;
  align-items: start;
  padding: 14px 16px;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: #1d222c;
  box-shadow: 0 12px 36px rgba(0, 0, 0, .35);
  opacity: 1;
  transform: translateY(0);
  transition:
    opacity .18s ease,
    transform .18s ease;
  pointer-events: auto;
}

.toast-success {
  border-color: rgba(37, 194, 110, .5);
}

.toast-error {
  border-color: rgba(255, 92, 92, .6);
}

.toast-text {
  line-height: 1.45;
  word-break: break-word;
}

.toast-close {
  padding: 0;
  border: 0;
  background: transparent;
  color: var(--muted);
  font: inherit;
  font-size: 22px;
  line-height: 1;
  cursor: pointer;
}

.toast-close:hover {
  color: var(--text);
}

.toast.is-hiding {
  opacity: 0;
  transform: translateY(-8px);
}

@media (max-width: 720px) {
  .toast-region {
    top: 10px;
    right: 10px;
    width: calc(100vw - 20px);
  }
}

.auto-refresh-status {
  margin: 0 0 18px;
  padding: 10px 12px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: rgba(154, 164, 178, .06);
  color: var(--muted);
  font-size: 13px;
  line-height: 1.45;
}

.auto-refresh-status.is-running {
  border-color: rgba(240, 160, 32, .45);
  color: var(--warn);
}

.auto-refresh-status.is-paused {
  border-color: rgba(154, 164, 178, .3);
  color: var(--muted);
}

.auto-refresh-status.is-success {
  border-color: rgba(37, 194, 110, .35);
  color: var(--ok);
}
"###;

pub async fn dashboard_css() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        DASHBOARD_CSS,
    )
}
