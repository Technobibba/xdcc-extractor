# XDCC Extractor

XDCC Extractor ist ein kleiner Rust-basierter Worker für automatisch heruntergeladene XDCC-Archive.

Der Worker überwacht einen Download-Ordner, erkennt fertige Archive, prüft sie, entpackt sie in einen Zielordner, merkt sich verarbeitete Releases und kann optional Gotify-Benachrichtigungen senden.

Das Projekt ist für den Betrieb per Docker gedacht.

---

## Hauptfunktionen

- überwacht einen Download-Ordner
- wartet, bis Dateien stabil sind
- verarbeitet Jobs über eine interne Queue
- erkennt Archive in Unterordnern und direkt im Root-Download-Ordner
- unterstützt mehrteilige Archive
- prüft Archive vor dem Entpacken
- entpackt Archive in einen konfigurierbaren Zielordner
- validiert, ob nach dem Entpacken Dateien vorhanden sind
- speichert verarbeitete Releases in einer History
- speichert fehlgeschlagene Releases mit Fehlertext und Versuchszähler
- unterstützt Retry mit Backoff
- unterstützt Dry-Run-Cleanup
- unterstützt Gotify-Benachrichtigungen
- unterstützt Passwortlisten für verschlüsselte Archive
- enthält Unit Tests für zentrale Logik
- enthält Docker Healthcheck
- enthält lokales Status-Script

---

## Unterstützte Archivformate

Aktuell unterstützt:

```text
.rar
.part01.rar / .part02.rar / ...
.r00 / .r01 / ...
.zip
.7z
.001 / .002 / ...
.tar
.tar.gz
.tgz
.tar.xz
.txz
.tar.bz2
.tbz2
```

RAR-Archive werden über `unrar` verarbeitet.

ZIP, 7z und Split-Archive werden über `7z` verarbeitet.

TAR-Archive werden über `tar` verarbeitet.

---

## Projektstruktur

```text
/opt/xdcc-extractor
├── src/
│   ├── main.rs
│   ├── config.rs
│   ├── extractor.rs
│   ├── history.rs
│   ├── notifications.rs
│   ├── passwords.rs
│   └── queue.rs
├── docs/
│   └── ROADMAP.md
├── scripts/
│   └── status.sh
├── state/
│   └── history/
├── config/
│   └── passwords.txt
├── Cargo.toml
├── Dockerfile
├── compose.yaml
├── healthcheck.sh
├── config.example.toml
├── config.docker.example.toml
├── passwords.example.txt
└── README.md
```

Lokale Dateien mit Secrets werden nicht committed:

```text
config.toml
config.docker.toml
config/passwords.txt
state/history/
```

---

## Docker-Betrieb

Der Worker wird per Docker Compose gestartet.

Beispiel `compose.yaml`:

```yaml
services:
  xdcc-extractor:
    build: .
    container_name: xdcc-extractor
    restart: unless-stopped

    environment:
      TZ: Europe/Berlin

    volumes:
      - /media/HDD3/XDCC:/downloads
      - ./config.docker.toml:/app/config.toml:ro
      - ./state:/state
      - ./config:/config:ro
```

Starten:

```bash
docker compose up -d
```

Logs ansehen:

```bash
docker compose logs -f
```

Neu bauen:

```bash
docker compose down
docker compose build
docker compose up -d
```

---

## Konfiguration

Die Docker-Konfiguration liegt lokal in:

```text
/opt/xdcc-extractor/config.docker.toml
```

Beispiel:

```toml
[watch]
directory="/downloads"
stable_after=30
allow_root_archives=true

[extract]
delete_archives=true
dry_run=true
keep_failed=true
password_file="/config/passwords.txt"

[output]
directory="/downloads/_extracted"

[history]
directory="/state/history"

[retry]
base_delay=60
max_delay=1800

[startup]
scan_existing=false

[notifications.gotify]
enabled=false
url="https://gotify.example.com"
token=""
priority_success=3
priority_error=8

notify_on_success=true
notify_on_error=true
notify_on_every_error=false
notify_after_attempts=3
```

---

## Wichtige Sicherheitsoptionen

### Dry Run

```toml
[extract]
dry_run=true
```

Wenn `dry_run=true` gesetzt ist, werden Archivdateien nach erfolgreicher Entpackung **nicht gelöscht**.

Der Worker zeigt nur an, welche Dateien gelöscht würden.

Empfohlen für Tests und Live-Start.

---

### Archive löschen

```toml
[extract]
delete_archives=true
```

Wenn `delete_archives=true` und `dry_run=false` gesetzt ist, löscht der Worker nach erfolgreicher Verarbeitung die erkannten Archivdateien.

Solange `dry_run=true` aktiv ist, wird trotzdem nichts gelöscht.

---

### Fehlerhafte Archive behalten

```toml
[extract]
keep_failed=true
```

Wenn ein Release fehlschlägt, bleibt es erhalten.

Der Fehler wird in der History gespeichert.

---

## Output-Verzeichnis

Das Zielverzeichnis für entpackte Releases wird hier konfiguriert:

```toml
[output]
directory="/downloads/_extracted"
```

Bei einem Archiv:

```text
/downloads/Movie.Release.1.zip
```

entsteht z. B.:

```text
/downloads/_extracted/Movie.Release.1/
```

---

## Root-Archive

Botarr/XDCC speichert Downloads oft direkt im Root-Ordner, z. B.:

```text
/downloads/Movie.Release.part01.rar
/downloads/Movie.Release.part02.rar
/downloads/Movie.Release.part03.rar
```

Dafür muss aktiviert sein:

```toml
[watch]
allow_root_archives=true
```

Dann erkennt der Worker automatisch das Startarchiv und verarbeitet die zusammengehörigen Parts.

---

## History

Die History wird hier gespeichert:

```toml
[history]
directory="/state/history"
```

Erfolgreiche Releases erhalten eine `.done`-Datei.

Fehlgeschlagene Releases erhalten eine `.failed`-Datei mit Fehlertext und Versuchszähler.

Beispiel:

```text
/state/history/Movie.Release.done
/state/history/Broken.Release.failed
```

---

## Retry / Backoff

Retry wird hier konfiguriert:

```toml
[retry]
base_delay=60
max_delay=1800
```

Beispiel:

```text
1. Fehler: Retry nach 60 Sekunden
2. Fehler: Retry nach 120 Sekunden
3. Fehler: Retry nach 240 Sekunden
...
maximal 1800 Sekunden
```

---

## Gotify-Benachrichtigungen

XDCC Extractor kann Benachrichtigungen über Gotify senden.

Unterstützt wird:

- Meldung bei erfolgreicher Verarbeitung
- Meldung bei Fehlern
- konfigurierbare Prioritäten
- Schutz vor zu vielen Fehlermeldungen

Beispiel:

```toml
[notifications.gotify]
enabled=true
url="https://gotify.example.com"
token="YOUR_GOTIFY_APP_TOKEN"

priority_success=3
priority_error=8

notify_on_success=true
notify_on_error=true

# Wenn true, wird jeder Fehler gemeldet.
notify_on_every_error=false

# Wenn notify_on_every_error=false ist,
# wird erst ab diesem Fehlversuch eine Meldung gesendet.
notify_after_attempts=3
```

Empfohlene Einstellung:

```toml
notify_on_success=true
notify_on_error=true
notify_on_every_error=false
notify_after_attempts=3
```

Damit wird ein fehlerhaftes Release nicht bei jedem Retry gemeldet, sondern erst nach mehreren Fehlversuchen.

Der Gotify-Token gehört nur in lokale Config-Dateien wie:

```text
config.docker.toml
```

und darf nicht committed werden.

---

## Gotify testen

```bash
curl -sS -X POST "https://gotify.example.com/message?token=YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"title":"XDCC Test","message":"Gotify funktioniert.","priority":5}'
```

---

## Passwortgeschützte Archive

XDCC Extractor kann passwortgeschützte Archive mit einer Passwortliste verarbeiten.

Ablauf:

```text
Archiv prüfen
↓
Passwortfehler erkannt
↓
Passwortliste laden
↓
Passwörter nacheinander testen
↓
mit passendem Passwort entpacken
```

Unterstützt wird aktuell:

- passwortgeschützte `.zip` Archive über `7z`
- passwortgeschützte `.7z` Archive über `7z`
- passwortgeschützte `.rar` Archive über `unrar`

---

## Passwortdatei konfigurieren

In der Config:

```toml
[extract]
password_file="/config/passwords.txt"
```

Wenn keine Passwortliste genutzt werden soll:

```toml
[extract]
password_file=""
```

---

## Passwortdatei im Docker-Setup

Der lokale Config-Ordner wird in den Container eingebunden:

```yaml
volumes:
  - ./config:/config:ro
```

Host-Pfad:

```text
/opt/xdcc-extractor/config/passwords.txt
```

Container-Pfad:

```text
/config/passwords.txt
```

---

## Beispiel `passwords.txt`

```text
# Kommentare beginnen mit #
password1
password2
mein-geheimes-passwort
```

Regeln:

- ein Passwort pro Zeile
- leere Zeilen werden ignoriert
- Zeilen mit `#` am Anfang werden ignoriert

Die Datei darf nicht committed werden.

Prüfen:

```bash
git check-ignore -v config/passwords.txt
```

---

## Verhalten bei falschem Passwort

Wenn kein Passwort passt, wird das Release als fehlgeschlagen markiert.

Der Fehler erscheint in:

```text
/state/history/*.failed
```

und kann per Gotify gemeldet werden.

---

## Docker Healthcheck

Der Container enthält einen Healthcheck.

Geprüft wird:

- Binary vorhanden
- Config vorhanden
- Download-Ordner vorhanden
- State-Ordner vorhanden
- Config-Ordner vorhanden, wenn Passwortdatei konfiguriert ist

Status prüfen:

```bash
docker ps --format "table {{.Names}}\t{{.Status}}"
```

Erwartung:

```text
xdcc-extractor   Up ... (healthy)
```

---

## Lokaler Status-Check

Für eine schnelle Übersicht auf dem Docker-Host:

```bash
./scripts/status.sh
```

Der Check zeigt:

- Git-Status
- Docker-Status
- Container-Health
- Docker-Mounts
- Config-Status
- Passwortdatei-Status
- wichtige Verzeichnisse
- letzte Container-Logs

---

## Entwicklung

Rust-Version prüfen:

```bash
rustc --version
cargo --version
```

Formatieren:

```bash
cargo fmt
```

Tests ausführen:

```bash
cargo test
```

Build:

```bash
cargo build
```

Release-Build:

```bash
cargo build --release
```

---

## Tests

Aktuell gibt es Tests für:

- Archiv-Erkennung
- Archiv-Start-Erkennung
- TAR-Erkennung
- RAR-Part-Erkennung
- Cleanup-Gruppen
- Queue-Verhalten
- History `.done` / `.failed`
- Fehlerklassifizierung
- Passwortdatei-Laden

Tests ausführen:

```bash
cargo test
```

---

## Testarchive erzeugen

### Normales ZIP

```bash
rm -rf /tmp/xdcc-test
mkdir -p /tmp/xdcc-test

echo "XDCC Test" > /tmp/xdcc-test/test.txt

rm -f /media/HDD3/XDCC/Test.Release.1.zip
rm -rf /media/HDD3/XDCC/_extracted/Test.Release.1

7z a /media/HDD3/XDCC/Test.Release.1.zip /tmp/xdcc-test/test.txt

rm -rf /tmp/xdcc-test
```

---

### Passwortgeschütztes ZIP

```bash
echo "secret123" > config/passwords.txt

docker compose restart

rm -rf /tmp/password-test
mkdir -p /tmp/password-test

echo "Passwort Test" > /tmp/password-test/test.txt

rm -f /media/HDD3/XDCC/Password.Test.1.zip
rm -rf /media/HDD3/XDCC/_extracted/Password.Test.1

7z a -psecret123 -mem=AES256 /media/HDD3/XDCC/Password.Test.1.zip /tmp/password-test/test.txt

rm -rf /tmp/password-test
```

Logs ansehen:

```bash
docker compose logs -f
```

Erwartung:

```text
Archiv benötigt vermutlich ein Passwort
Archivprüfung mit Passwort #1 erfolgreich
Entpackung abgeschlossen
Entpackung validiert
```

---

### Fehlerhaftes Archiv

```bash
rm -f /media/HDD3/XDCC/Error.Test.1.zip
echo "kein echtes archiv" > /media/HDD3/XDCC/Error.Test.1.zip
```

Erwartung im Log:

```text
Grund: Datei ist kein gültiges Archiv
```

---

## Git-Sicherheit

Vor jedem Commit prüfen:

```bash
git status --short
```

Secrets dürfen nicht committed werden:

```bash
git check-ignore -v config.docker.toml
git check-ignore -v config/passwords.txt
```

Sicher committen:

```bash
git add .
git restore --staged config.docker.toml 2>/dev/null || true
git restore --staged config/passwords.txt 2>/dev/null || true
git commit -m "Commit message"
```

---

## Aktueller empfohlener Betrieb

Für Live-Betrieb aktuell empfohlen:

```toml
[extract]
delete_archives=true
dry_run=true
keep_failed=true
password_file="/config/passwords.txt"
```

Damit werden Archive verarbeitet und Entpackungen geprüft, aber Archivdateien noch nicht gelöscht.

Erst wenn genügend echte Downloads sauber verarbeitet wurden, sollte `dry_run=false` getestet werden.

---

## Roadmap

Kurzfristig:

- separate Fehlerklasse `password_required`
- bessere Gotify-Meldung für Passwortfehler
- Status-Ausgabe direkt im Binary
- Tests für Cleanup mit echten temporären Dateien
- bessere Config-Fehlermeldungen

Später:

- WebUI
- Passwortliste über WebUI verwalten
- manuelles Retry über WebUI
- Release-Übersicht
- Integration mit Medienbibliothek
- optionaler Move nach erfolgreicher Verarbeitung

---

## Fehlerklassen

XDCC Extractor klassifiziert typische Archivfehler maschinenlesbar.

Aktuell bekannte Fehlerklassen:

    password_required
    corrupt_archive
    missing_part
    unsupported_method
    invalid_archive
    unknown

Beispiel in Logs oder History:

    Fehlerklasse: password_required
    Grund: Passwort erforderlich oder falsches Passwort

Diese Fehlerklasse wird auch für Gotify genutzt.

Bei `password_required` sendet Gotify eine eigene Meldung:

    XDCC Extractor: Passwort benötigt

Dadurch lassen sich Passwortfehler besser von kaputten oder unvollständigen Archiven unterscheiden.

---

## Version anzeigen

Lokal nach einem Build:

```bash
./target/debug/xdcc-extractor --Version

Im Docker-Container:

docker exec -it xdcc-extractor /usr/local/bin/xdcc-extractor --version

Der Status zeigt die Version ebenfalls an:

docker exec -it xdcc-extractor /usr/local/bin/xdcc-extractor --status


---

## Config-Pfad explizit angeben

Der Worker nutzt standardmäßig:

```text
config.toml

Alternativ kann eine Config explizit angegeben werden:

xdcc-extractor --config /pfad/zur/config.toml

Beispiel lokal nach einem Build:

./target/debug/xdcc-extractor --config config.docker.toml

Auch der Status-Befehl unterstützt --config:

./target/debug/xdcc-extractor --status --config config.docker.toml

