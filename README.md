# XDCC Extractor

XDCC Extractor ist ein Rust-basierter Docker-Worker zum automatischen Erkennen, Prüfen und Entpacken von XDCC-/Botarr-Downloads.

Der Worker überwacht einen oder mehrere Download-Ordner, erkennt fertige Releases, verarbeitet Archive automatisch, verwaltet Verlauf und Fehlerstatus und bietet eine geschützte WebUI für Kontrolle, Konfiguration, Wartung und Diagnose.

## Features

- Docker-first Betrieb
- Watcher für neue Downloads in einem oder mehreren Ordnern
- Prüfung vorhandener Releases beim Start
- Warteschlange mit Duplikatschutz
- Archivprüfung vor dem Entpacken
- Unterstützung für Passwortlisten
- Wiederholungslogik bei Fehlern
- Verlauf für erfolgreiche und fehlgeschlagene Releases
- Bereinigung nach erfolgreicher Verarbeitung
- Gotify-Benachrichtigungen
- Geschützte WebUI mit Basic Auth
- Dashboard mit Release-Übersicht und manuellen Aktionen
- Read-only Einstellungen und separater Bearbeiten-Bereich
- Überwachte Ordner direkt über die WebUI hinzufügen, bearbeiten und entfernen
- History-/Verlaufs-Reset mit automatischer Sicherung
- Passwortlisten-Verwaltung mit automatischen Sicherungen
- Diagnose-Seite für Pfade, Speicherplatz, Verlauf, Gotify und Passwortliste
- Übersicht vorhandener Config-, Verlaufs- und Passwortlisten-Sicherungen
- Logs und geschützte JSON-APIs
- Docker-Healthcheck
- Automatischer WebUI-Smoke-Test für Seiten, APIs, Assets und UX-Funktionen
- Automatische Dashboard-Aktualisierung mit Pause bei inaktivem Browser-Tab
- Toast-Meldungen und sichtbare Ladezustände für WebUI-Aktionen
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
directories=[]

[output]
directory="/downloads/_extracted"

[web]
enabled=true
bind="0.0.0.0:8099"
~~~

Die produktive `config.docker.toml` kann private Pfade, Gotify URL und Token enthalten und darf nicht committed werden.

### Mehrere überwachte Ordner

Zusätzliche physische Ordner oder Datenträger
müssen zuerst in `compose.yaml` in den
Container eingebunden werden:

~~~yaml
services:
  xdcc-extractor:
    volumes:
      - /media/HDD3/XDCC:/downloads
      - /media/HDD4/XDCC:/downloads2
~~~

Danach können die Container-Pfade in der WebUI
unter `Einstellungen → Bearbeiten → Überwachung`
eingetragen werden:

~~~text
/downloads
/downloads2
~~~

Der erste Eintrag wird als `watch.directory`
gespeichert. Weitere Einträge werden unter
`watch.directories` abgelegt.

Bestehende Ein-Ordner-Konfigurationen bleiben
vollständig kompatibel.

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

Die WebUI ist per Basic Auth geschützt. Die Zugangsdaten werden über `.env` gesetzt.

Verfügbare Seiten:

~~~text
/                  Dashboard
/settings          Aktuell geladene Einstellungen
/settings/edit     Einstellungen und Wartungsfunktionen
/logs              Laufende Worker-Logs
/diagnostics       Read-only Diagnose und Sicherungsübersicht
/health            Öffentlicher Docker-Healthcheck
~~~

Funktionen:

- Worker- und Systemstatus anzeigen
- Releases neu prüfen
- Releases manuell verarbeiten
- Fehlerstatus einzelner Releases zurücksetzen
- Letzte Fehler anzeigen
- Laufende Logs anzeigen
- Sichere Einstellungen bearbeiten
- Einen oder mehrere überwachte Ordner verwalten
- Gotify-URL und Token neu setzen, ohne bestehende Werte anzuzeigen
- Verlauf mit vorheriger Sicherung zurücksetzen
- Einzelne Passwörter ergänzen
- Passwortliste vollständig ersetzen
- Config-, Verlaufs- und Passwortlisten-Sicherungen anzeigen
- Erreichbarkeit wichtiger Speicherorte prüfen
- Gesamten, belegten und freien Speicherplatz je Dateisystem anzeigen
- Worker direkt über die WebUI neu starten
- Dashboard automatisch alle 30 Sekunden aktualisieren

Die Diagnose-Seite zeigt keine Passwortinhalte, Gotify-Tokens oder anderen vertraulichen Inhalte.

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

Die Config- und Status-APIs liefern für neue
Integrationen Listenfelder mit allen Watch-Ordnern:

~~~text
watch.directories
watch_directories
~~~

Die bisherigen Einzelordner-Felder bleiben aus
Kompatibilitätsgründen erhalten.

## Wichtige lokale Pfade

Im Standard-Dockerbetrieb:

~~~text
/downloads                    Haupt-Download-Ordner im Container
/downloads2                   Optionaler zusätzlicher Watch-Ordner
/downloads/_extracted         Zielordner für entpackte Dateien
/state/history                Verlauf
/state/config-backups         Config-Sicherungen
/state/history-backups        Verlaufs-Sicherungen
/state/password-backups       Passwortlisten-Sicherungen
/config/passwords.txt         Optionale Passwortliste
/app/config.toml              Gemountete Runtime-Config
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

WebUI inklusive Seiten, APIs und Assets prüfen:

~~~bash
./scripts/webui-smoke-test.sh
~~~

## WebUI-Modulstruktur

Die WebUI ist in klar getrennte Module aufgeteilt:

~~~text
src/web.rs               Router, Auth und Formular-Handler
src/web_api.rs           JSON-API-Endpunkte
src/web_pages.rs         HTML-Seiten und sichtbare Inhalte
src/web_assets.rs        JavaScript
src/web_styles.rs        Gemeinsame und seitenspezifische Styles
src/web_settings.rs      Config-Speicherung und Config-Sicherungen
src/web_maintenance.rs   Verlaufs- und Passwortlisten-Verwaltung
src/web_history.rs       Verlauf und Fehlerdaten
src/web_backups.rs       Read-only Sicherungsübersicht
src/web_disk.rs          Speicherplatzabfrage für die Diagnose
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
./scripts/webui-smoke-test.sh
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
