# Implementierungsplan v1.1.0

## Meilenstein 1 – Konfigurationsmodell

- `NotificationConfig` provider-unabhängig gestalten
- `NtfyConfig` ergänzen
- Validierung und redigiertes Debug implementieren
- Unit-Tests ergänzen
- Beispielkonfigurationen aktualisieren

**Abnahmekriterium:** `cargo test` erfolgreich; alte Config verursacht keinen Parse-Abbruch.

## Meilenstein 2 – ntfy-Provider

- Gotify-Request durch ntfy-POST ersetzen
- optionale Bearer-Authentifizierung
- ntfy-Prioritäten 1–5
- Event-/Message-Abstraktion
- Erfolg und Fehler an bestehenden Aufrufstellen anbinden

**Abnahmekriterium:** Erfolgs- und Fehlertest gegen lokale ntfy-Instanz funktionieren.

## Meilenstein 3 – WebUI-Einstellungen

- Formularfelder umstellen
- Secret beibehalten/löschen korrekt behandeln
- Config-Backup weiterverwenden
- sichtbare Gotify-Texte entfernen
- Diagnose/API redigieren

**Abnahmekriterium:** Einstellungen lassen sich speichern; Token wird nie ausgegeben.

## Meilenstein 4 – Testnachricht

- geschützte API-Route ergänzen
- UI-Button und Feedback ergänzen
- HTTP-Fehler verständlich übersetzen
- Smoke-Test erweitern

**Abnahmekriterium:** Testnachricht kann ohne Worker-Neustart ausgelöst werden.

## Meilenstein 5 – Dokumentation und Release

- README, Docker-Doku, Roadmap und Changelog aktualisieren
- Publication-Check erweitern
- vollständige Release-Checkliste ausführen
- Version auf 1.1.0 setzen

## Empfohlene Commit-Reihenfolge

1. `Add ntfy notification configuration`
2. `Replace Gotify sender with ntfy provider`
3. `Add ntfy settings to WebUI`
4. `Add notification test endpoint`
5. `Update notification documentation for v1.1.0`
