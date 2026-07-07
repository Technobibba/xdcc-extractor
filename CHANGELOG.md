# Changelog



## v0.7.0 - 2026-07-07

### Added
- Gotify URL und Token können über die WebUI neu gesetzt werden, ohne bestehende Werte anzuzeigen
- Automatische Config-Backups bei WebUI-Settings-Änderungen
- WebUI-Neustart-Aktion
- GitHub CI Workflow
- Dockerignore für sauberen Build Context
- Public Release Checkliste und Publication-Check Script

### Changed
- Settings-Seite nutzt freundlichere Beschriftungen
- Boolean-Werte werden als Badges angezeigt
- Backup-Meldungen in der WebUI sind verständlicher
- Öffentliche Doku und Beispielconfigs wurden für GitHub bereinigt

### Security
- Gotify URL und Token werden in der WebUI nicht angezeigt
- Runtime-Dateien, Logs, State und lokale Configs werden nicht getrackt
- Publication-Check prüft auf private URLs, Tokens und sensible Dateien

## v0.6.0 - 2026-07-07

### Added
- WebUI Logs-Ansicht mit `/logs` und `/api/logs`
- Basic Auth für WebUI und geschützte APIs
- Docker HTTP-Healthcheck über `/health`
- Editierbare WebUI Settings für sichere Config-Werte

### Changed
- Config-Mount ist beschreibbar, damit WebUI Settings speichern kann

### Security
- Secrets wie Gotify Token, WebUI Passwort und Passwortlisten-Inhalt werden nicht angezeigt
- `.env` bleibt lokal und wird nicht committed

## v0.1.0-alpha

- Initial project
- Git repository
- Docker setup
- Logger
- Watcher
