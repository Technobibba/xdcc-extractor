# Changelog


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
