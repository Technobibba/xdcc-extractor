# XDCC Extractor

XDCC Extractor ist ein kleiner Rust-basierter Worker für Homeserver- und Docker-Setups.

Er überwacht einen Download-Ordner, erkennt fertige XDCC-Downloads, prüft Archive, entpackt sie automatisch und kann Archivdateien nach erfolgreicher Verarbeitung optional löschen.

Das Projekt ist besonders für Workflows gedacht, bei denen Downloads nicht direkt über Sonarr/Radarr laufen, sondern z. B. über Botarr oder andere XDCC-Downloader.

---

## Aktueller Status

Der Worker ist noch in Entwicklung, aber bereits lauffähig.

Aktuell unterstützt:

- Watcher für neue Dateien
- Release-Erkennung
- Flat-Downloads direkt im Watch-Root
- Downloads in Unterordnern
- Queue-Verarbeitung
- Verify → Extract → Validate Pipeline
- Retry mit Backoff
- History für erfolgreiche und fehlgeschlagene Releases
- Docker-Betrieb
- Dry-Run-Modus für sicheren Cleanup

---

## Unterstützte Archivformate

| Format | Tool |
|---|---|
| `.rar` | `unrar` |
| `.zip` | `7z` |
| `.7z` | `7z` |
| `.001` | `7z` |
| `.tar` | `tar` |
| `.tar.gz` | `tar` |
| `.tgz` | `tar` |
| `.tar.xz` | `tar` |
| `.txz` | `tar` |
| `.tar.bz2` | `tar` |
| `.tbz2` | `tar` |

---

### Output-Ordner

```toml
[output]
directory="/downloads/_extracted"

---

## Verarbeitungspipeline

```text
WATCH
↓
READY
↓
QUEUE
↓
VERIFY
↓
EXTRACT
↓
VALIDATE
↓
CLEANUP / DRY-RUN
↓
HISTORY
