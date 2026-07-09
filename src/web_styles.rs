use axum::{http::header, response::IntoResponse};

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
}
code {
  color: #cfd7e6;
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
code {
  color: #cfd7e6;
}
.small {
  color: var(--muted);
  font-size: 13px;
}
@media (max-width: 720px) {
  .grid {
    grid-template-columns: 1fr;
  }
}"###;

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
code {
  color: #cfd7e6;
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
}"###;

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
code {
  color: #cfd7e6;
}
@media (max-width: 720px) {
  .scan-row {
    grid-template-columns: 1fr;
  }
  .scan-actions {
    text-align: left;
  }
}"###;

pub async fn dashboard_css() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        DASHBOARD_CSS,
    )
}
