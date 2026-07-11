# XDCC Extractor Roadmap

Stand: 2026-07-10

## Abgeschlossen

### Worker-Kern

- Watcher für neue Downloads
- Prüfung vorhandener Releases beim Start
- Warteschlange mit Duplikatschutz
- Archivprüfung und Entpacken
- Mehrteilige und passwortgeschützte Archive
- Passwortlisten
- Wiederholungslogik
- Verlauf für erfolgreiche und fehlgeschlagene Releases
- Sichere Bereinigung nach erfolgreicher Verarbeitung

### CLI

- `--status`
- `--scan`
- `--clear-failed <PATH>`
- `--process <PATH>`
- `--help`
- `--version`

### Docker und Betrieb

- Dockerfile und Compose
- Persistente State-Daten
- HTTP-Healthcheck
- Runtime-Config außerhalb von Git
- Worker-Neustart über die WebUI

### WebUI

- Geschützte WebUI mit Basic Auth
- Dashboard
- Release-Übersicht
- Manuelle Verarbeitung
- Fehlerstatus zurücksetzen
- Letzte Fehler
- Logs
- Read-only Einstellungen
- Bearbeiten-Bereich
- Gotify-Konfiguration
- Verlaufs-Reset mit Sicherung
- Passwortlisten-Verwaltung mit Sicherung
- Diagnose-Seite
- Backup-Übersicht
- Einheitliche Navigation
- Deutsche und verständlichere Beschriftungen
- Responsive Eingabefelder

### Qualität und Veröffentlichung

- Rust-Tests
- GitHub CI
- WebUI-Smoke-Test
- Asset-Prüfungen
- Publication-Check
- Öffentliche Release-Checkliste
- Secret- und Runtime-Dateien aus Git ausgeschlossen

### Refactor

- HTML nach `web_pages.rs`
- JavaScript nach `web_assets.rs`
- CSS nach `web_styles.rs`
- gemeinsame CSS-Regeln dedupliziert
- API nach `web_api.rs`
- Config-Speicherung nach `web_settings.rs`
- Wartungslogik nach `web_maintenance.rs`
- Verlaufslogik nach `web_history.rs`
- Sicherungsübersicht nach `web_backups.rs`

## Aktuell

### Release v0.9.0

- Dokumentation aktualisieren
- vollständigen Qualitätscheck ausführen
- Version auf `0.9.0` erhöhen
- Release committen und taggen

## Danach möglich

### Bedienung

- Erfolgsmeldungen ohne vollständigen Seiten-Reload
- einheitliche Toast-Meldungen
- Ladeindikatoren für Aktionen
- automatische Aktualisierung des Dashboards

### Diagnose

- Worker-Laufzeit
- Speicherplatzinformationen
- detailliertere Ordnerprüfungen
- optionale Gotify-Verbindungsprüfung

### Erweiterungen

- Live-Logs über Server-Sent Events
- Filter und Suche für Releases und Fehler
- sichtbare Release-Historie
- Wiederherstellung ausgewählter Sicherungen
- mehrere Watch-Ordner
- automatisierte GitHub Releases
- GitHub Container Registry
