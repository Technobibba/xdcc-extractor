# XDCC Extractor

XDCC Extractor ist ein Rust-basierter Docker-Worker zum automatischen Erkennen, Prüfen und Entpacken von XDCC-/Botarr-Downloads.

Der Worker überwacht einen Download-Ordner, erkennt fertige Releases, verarbeitet Archive automatisch, verwaltet History/Fehlerstatus und bietet eine geschützte WebUI zur Kontrolle und Konfiguration.

## Features

- Docker-first Betrieb
- Watcher für neue Downloads
- Startup-Scan vorhandener Releases
- Queue-Verarbeitung
- Archivprüfung vor dem Entpacken
- Unterstützung für Passwortlisten
- Retry-Logik bei Fehlern
- History für erfolgreiche und fehlgeschlagene Releases
- Cleanup nach erfolgreicher Verarbeitung
- Gotify-Benachrichtigungen
- WebUI mit Basic Auth
- Dashboard, Scan, Logs und Settings
- Manuelle Verarbeitung einzelner Releases
- Zurücksetzen fehlgeschlagener Releases
- Editierbare sichere Einstellungen
- Config-Backups bei Änderungen
- Docker Healthcheck
- Publication-Check für öffentliche Releases

## Docker Quickstart

### 1. Repository vorbereiten

~~~bash
git clone <REPOSITORY_URL>
cd xdcc-extractor
~~~

### 2. Config erstellen

~~~bash
cp config.docker.example.toml config.docker.toml
cp .env.example .env
~~~

`.env` bearbeiten:

~~~env
XDCC_WEB_AUTH_USER=admin
XDCC_WEB_AUTH_PASSWORD=change-me
~~~

`config.docker.toml` anpassen:

~~~toml
[watch]
directory="/downloads"

[output]
directory="/downloads/_extracted"

[web]
enabled=true
bind="0.0.0.0:8099"
~~~

Die produktive `config.docker.toml` kann private Pfade, Gotify URL und Token enthalten und darf nicht committed werden.

### 3. Docker starten

~~~bash
docker compose build
docker compose up -d
docker compose logs --tail=100
~~~

WebUI öffnen:

~~~text
http://<docker-host>:8099
~~~

## WebUI

Die WebUI ist per Basic Auth geschützt. Zugangsdaten werden über `.env` gesetzt.

Verfügbare Seiten:

~~~text
/
 /settings
/settings/edit
/logs
/health
~~~

Funktionen:

- Status anzeigen
- Scan aktualisieren
- Releases manuell verarbeiten
- Failed Releases zurücksetzen
- Letzte Fehler anzeigen
- Logs anzeigen
- Einstellungen bearbeiten
- Gotify URL und Token neu setzen, ohne bestehende Werte anzuzeigen
- Worker neu starten

## APIs

~~~text
/health
/api/status
/api/config
/api/scan
/api/failures
/api/logs
/api/clear-failed
/api/process
/api/restart
~~~

`/health` ist absichtlich ohne Auth erreichbar, damit Docker den Healthcheck ausführen kann.

Alle anderen WebUI-/API-Routen sind geschützt.

## Wichtige lokale Pfade

Im Standard-Dockerbetrieb:

~~~text
/downloads              Download-Ordner im Container
/downloads/_extracted   Zielordner für entpackte Dateien
/state/history          History
/state/config-backups   Config-Backups
/config/passwords.txt   Optionale Passwortliste
/app/config.toml        Gemountete Runtime-Config
~~~

## Sicherheit

Diese Dateien enthalten lokale Daten oder Secrets und dürfen nicht committed werden:

~~~text
.env
config.toml
config.docker.toml
config.env
config/*.txt
state/
logs/
target/
~~~

Die WebUI zeigt folgende Werte nicht an:

- Gotify Token
- Gotify URL
- WebUI Passwort
- Inhalt der Passwortliste

Vor einer Veröffentlichung prüfen:

~~~bash
./scripts/publication-check.sh
~~~

## Entwicklung

~~~bash
cargo fmt
cargo test
cargo build
~~~

Docker neu bauen:

~~~bash
docker compose down
docker compose build
docker compose up -d
~~~

Logs anzeigen:

~~~bash
docker compose logs --tail=100
~~~

Healthcheck prüfen:

~~~bash
docker inspect --format='{{.State.Health.Status}}' xdcc-extractor
~~~

## CLI

~~~bash
xdcc-extractor --status
xdcc-extractor --scan
xdcc-extractor --dry-run-report
xdcc-extractor --dry-run-check
xdcc-extractor --clear-failed <PATH>
xdcc-extractor --process <PATH>
~~~

## Release

Vor jedem Release:

~~~bash
cargo fmt
cargo test
cargo build
./scripts/publication-check.sh
~~~

Version setzen, committen und taggen:

~~~bash
git tag -a vX.Y.Z -m "XDCC Extractor vX.Y.Z"
~~~

## Dokumentation

Weitere Dokumente:

~~~text
docs/DOCKER.md
docs/RELEASE_CHECKLIST.md
~~~

## Lizenz

Siehe `LICENSE`.
