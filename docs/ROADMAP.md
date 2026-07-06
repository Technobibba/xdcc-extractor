# XDCC Extractor Roadmap

## Erledigt

### Projektfundament

- Rust-Projekt erstellt
- Git-Repository initialisiert
- Cargo Build funktioniert
- Dockerfile erstellt
- Docker Compose eingerichtet
- Config-Dateien vorbereitet
- Lokale `config.toml` aus Git ausgeschlossen

---

### Watcher & Release-Erkennung

- Watch-Ordner überwachen
- Dateiänderungen erkennen
- Release-Kandidaten sammeln
- Warten bis ein Release stabil ist
- `stable_after` konfigurierbar
- Unterordner-Releases erkennen
- Flat-Downloads im Root erkennen
- Root-Archive optional erlauben
- Startup-Scan optional aktivieren/deaktivieren

---

### Queue & Verarbeitung

- JobQueue erstellt
- Releases werden nacheinander verarbeitet
- Keine parallelen Entpackungen
- Fehlgeschlagene Jobs werden erneut versucht
- Retry-Backoff eingebaut

---

### Archiv-Erkennung

Unterstützt:

- `.rar`
- `.zip`
- `.7z`
- `.001`
- `.tar`
- `.tar.gz`
- `.tgz`
- `.tar.xz`
- `.txz`
- `.tar.bz2`
- `.tbz2`

---

### Verify / Extract / Validate

- Archive prüfen
- Archive entpacken
- Zielordner erstellen
- Vorhandenen Zielordner ersetzen
- Entpackung validieren
- Leere Entpackung erkennen
- Fehler sauber loggen

---

### Cleanup & Sicherheit

- Cleanup-Kandidaten erkennen
- Nur Archivdateien als Cleanup-Kandidaten verwenden
- Sicherheitsprüfung für Cleanup-Pfade
- Dry-Run-Modus eingebaut
- Echte Löschung technisch möglich, aber per `dry_run=true` abgesichert

---

### History

- Erfolgreiche Releases als `.done` speichern
- Fehlgeschlagene Releases als `.failed` speichern
- Anzahl Fehlversuche speichern
- Fehlertext speichern
- Erfolgreiche Verarbeitung entfernt alten Fehlerstatus
- Bereits verarbeitete Releases werden übersprungen

---

### Docker

- Multi-Stage Docker Build
- Runtime mit `7z`, `unrar`, `tar`
- Config als Volume
- State persistent als Volume
- Downloadordner als Volume

---

## Als Nächstes

### Dokumentation

- README pflegen
- Konfigurationsoptionen dokumentieren
- Troubleshooting ergänzen
- Beispiel-Setups ergänzen

---

### Output verbessern

Aktuell:

```text
/downloads/_extracted/Release.Name/

---

## Passwortarchive

### Erledigt

- Passwortdatei in der Config
- Passwortliste wird geladen
- Kommentare und leere Zeilen werden ignoriert
- Passwortgeschützte Archive werden erkannt
- Passwörter werden nacheinander getestet
- Erfolgreiches Passwort wird für die Entpackung verwendet
- Fehler werden sauber klassifiziert

### Geplant

- Fehlerstatus `password_required`
- Optional getrennte Meldung für Passwortfehler
- Passwortliste über WebUI verwalten

---

## Dry-Run / Cleanup

### Erledigt

- Cleanup nur nach erfolgreicher Entpackung
- Cleanup nur nach erfolgreicher Validierung
- Dry-Run-Modus
- Cleanup-Tests mit temporären Dateien
- Dry-Run-Report
- Dry-Run-Safety-Check
- kontrollierter `dry_run=false` Test erfolgreich

### Offen

- Produktiventscheidung, wann `dry_run=false` dauerhaft aktiviert wird
