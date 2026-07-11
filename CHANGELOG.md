# Changelog

## Unreleased

### Added

- Mehrere gleichzeitig überwachte Ordner
- Abwärtskompatible Konfiguration über `watch.directory` und `watch.directories`
- Verwaltung der Watch-Ordner direkt über die WebUI
- Mehrordner-Unterstützung für Worker, Startup-Scan und manuellen Scan
- Mehrordner-Ausgabe in Dashboard, Einstellungen, Diagnose und JSON-APIs
- Speicherplatzanzeige mit Gesamt-, Belegt-, Frei- und Auslastungswerten
- Eigene Diagnosekarte für jeden überwachten Ordner
- Automatische Dashboard-Aktualisierung alle 30 Sekunden
- Pause der automatischen Aktualisierung bei inaktivem Browser-Tab
- Toast-Meldungen und sichtbare Ladezustände für WebUI-Aktionen
- Erweiterte Smoke-Tests für UX-, Auto-Refresh-, Speicherplatz- und Mehrordner-Funktionen

### Changed

- Diagnosebereich für Speicherorte übersichtlicher gestaltet
- Scan-Logik zentralisiert
- Watch-Ordner werden normalisiert und doppelte Einträge entfernt
- Alte Ein-Ordner-Konfigurationen bleiben ohne Migration nutzbar
- Neue API-Listenfelder ergänzt; bestehende Einzelordner-Felder bleiben erhalten

### Removed

- Früherer Laufzeit-Sicherheitsmodus
- Zugehörige veraltete Prüf- und Berichtsbefehle
- Redundante Dashboard-Angaben und erklärende Footer-Hinweise

### Security

- Watch-Ordner können nur als absolute Container-Pfade gespeichert werden
- Config-Backups werden weiterhin vor Änderungen angelegt
- Gotify-Secrets und Passwortlisten-Inhalte bleiben verborgen





## v0.9.0 - 2026-07-10

### Added
- Read-only Diagnose-Seite
- Übersicht für Config-, Verlaufs- und Passwortlisten-Sicherungen
- WebUI-Smoke-Test für Seiten, APIs, JavaScript und Stylesheets
- Eigenes Modul für die Sicherungsübersicht

### Changed
- WebUI vollständig in kleinere Module aufgeteilt
- HTML, JavaScript und CSS aus `web.rs` ausgelagert
- Gemeinsame CSS-Regeln zusammengeführt
- Dashboard und Einstellungen sprachlich vereinheitlicht
- Sichtbare Release-Zustände auf Deutsch dargestellt
- Eingabefelder kompakter und responsiver gestaltet
- Navigation und Abstände überarbeitet
- README und Roadmap an den aktuellen Funktionsumfang angepasst

### Security
- Diagnose zeigt keine Passwortinhalte oder Gotify-Secrets
- Sicherungsübersicht zeigt keine Backup-Inhalte
- Geschützte WebUI- und API-Routen bleiben hinter Basic Auth
- `/health` bleibt ausschließlich für den Docker-Healthcheck öffentlich

## v0.8.0 - 2026-07-08

### Added
- WebUI History-Reset mit Backup
- WebUI Passwortlisten-Verwaltung
- Backups für Passwortlistenänderungen
- Einheitliche WebUI-Navigation

### Changed
- Dashboard vereinfacht
- API-Karte durch Systemstatus ersetzt
- Navigation-Abstände verbessert
- Bearbeiten-Seite um Wartungsfunktionen erweitert

### Security
- Passwortlisten-Inhalt wird weiterhin nicht angezeigt
- History- und Passwortänderungen erstellen Backups
- Lokale Runtime-Dateien bleiben aus Git ausgeschlossen

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
