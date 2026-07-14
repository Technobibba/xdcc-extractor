# XDCC Extractor v1.1.0 – Notification Design

## Ziel

Gotify wird durch eine provider-unabhängige Benachrichtigungsarchitektur ersetzt. v1.1.0 liefert zunächst den Provider `ntfy`; weitere Provider können später ergänzt werden, ohne Download-, Queue- oder Extractor-Code umzubauen.

## Grundsätze

- Benachrichtigungen sind optional und standardmäßig deaktiviert.
- Fehler beim Senden dürfen den Worker niemals abbrechen.
- Tokens werden weder in Logs noch in Diagnose/API/WebUI ausgegeben.
- Konfigurationsänderungen über die WebUI erzeugen weiterhin ein Backup.
- Interne Dienste dürfen HTTP verwenden; öffentliche ntfy-URLs sollten HTTPS verwenden.
- Die WebUI bietet Speichern und eine separate Testnachricht.

## Konfiguration

```toml
[notifications]
enabled=false
provider="ntfy"

[notifications.ntfy]
server="https://ntfy.example.org"
topic="xdcc-extractor"
token=""
priority_success=3
priority_error=5
notify_on_success=true
notify_on_error=true
notify_on_every_error=false
notify_after_attempts=3
```

### Validierung

Wenn `notifications.enabled=true` gilt:

- `provider` muss `ntfy` sein.
- `notifications.ntfy.server` darf nicht leer sein.
- `notifications.ntfy.topic` darf nicht leer sein.
- `server` muss mit `http://` oder `https://` beginnen.
- `topic` darf keine Leerzeichen, `?` oder `#` enthalten.
- Prioritäten müssen im ntfy-Bereich 1 bis 5 liegen.
- `notify_after_attempts` muss mindestens 1 sein.
- Ein leerer Token ist zulässig, damit öffentliche Topics unterstützt werden.

## Rust-Architektur

### Öffentliche Schnittstelle

```rust
pub enum NotificationEvent {
    ProcessingSucceeded,
    ProcessingFailed,
    Test,
}

pub struct NotificationMessage {
    pub event: NotificationEvent,
    pub title: String,
    pub body: String,
    pub priority: u8,
    pub tags: Vec<String>,
}

pub struct Notifications {
    config: NotificationConfig,
}
```

`Notifications` bleibt die einzige Schnittstelle für den übrigen Worker. Provider-spezifische Logik bleibt innerhalb von `src/notifications.rs` oder wird später in `src/notifications/` aufgeteilt.

### ntfy-Request

- Methode: `POST`
- URL: `{server}/{topic}`
- Header:
  - `Authorization: Bearer <token>` nur bei nicht leerem Token
  - `Title`
  - `Priority`
  - `Tags`
- Body: Nachricht als UTF-8-Text
- Timeouts müssen gesetzt werden.
- Fehler werden mit redigierten Informationen geloggt.

## Ereignisse in v1.1.0

Verbindlich:

- Verarbeitung erfolgreich
- Verarbeitung fehlgeschlagen
- Testnachricht aus der WebUI

Für spätere Minor-Versionen vorbereitet:

- Verarbeitung gestartet
- Entpacken gestartet
- Queue-Timeout
- Watch-Verzeichnis nicht erreichbar
- Worker-Neustart

Die aktuelle Anwendung erkennt keine eigenständigen IRC-Verbindungsereignisse. Ein Ereignis `IRCDisconnected` gehört daher nicht in v1.1.0.

## WebUI

Der vorhandene Bearbeiten-Bereich erhält eine Karte **Benachrichtigungen** mit:

- Aktiviert
- Provider (in v1.1.0 read-only: ntfy)
- Server-URL
- Topic
- neuer Access Token
- Priorität Erfolg
- Priorität Fehler
- Erfolgsmeldungen
- Fehlermeldungen
- jeden Fehler melden
- erst nach Anzahl Versuchen melden
- Einstellungen speichern
- Testnachricht senden

### Secret-Verhalten

- Der bestehende Token wird niemals als Formularwert gerendert.
- Leeres Token-Feld beim Speichern bedeutet: bestehenden Token behalten.
- Eine separate Checkbox „Gespeicherten Token löschen“ ermöglicht das Entfernen.
- Diagnose und JSON-API zeigen nur `token_configured: true|false`.

### Test-Endpunkt

`POST /api/notifications/test`

Ergebnis als JSON:

```json
{"ok":true,"message":"Testnachricht wurde gesendet"}
```

Fehler werden nutzerfreundlich klassifiziert:

- Server nicht erreichbar
- TLS-/DNS-Fehler
- HTTP 401: Token ungültig
- HTTP 403: keine Schreibberechtigung
- sonstiger HTTP-Fehler

Der Endpunkt nutzt die aktuell gespeicherte Konfiguration. Optional kann später ein „Test vor Speichern“ ergänzt werden.

## Migration Gotify → ntfy

v1.1.0 führt einen bewussten Konfigurationsschnitt durch. `[notifications.gotify]` wird nicht mehr aktiv verwendet.

Beim Laden einer alten Konfiguration:

- Anwendung startet mit deaktivierten ntfy-Benachrichtigungen.
- Es wird eine klare Warnung geloggt, wenn `[notifications.gotify]` erkannt wird.
- Die WebUI erklärt, dass ntfy neu eingerichtet werden muss.
- Die alte Sektion wird erst beim Speichern der Einstellungen aus der Datei entfernt.

Damit werden bestehende Installationen nicht durch einen Parse-Fehler unstartbar, aber Secrets werden auch nicht automatisch migriert.

## Sicherheit

- `Debug`-Implementierungen redigieren Tokens.
- Fehlertexte dürfen keine Request-Header oder komplette Ziel-URL mit Secrets enthalten.
- Publication-Check sucht zusätzlich nach `tk_`-Tokens und Bearer-Headern.
- Beispielconfigs enthalten ausschließlich Platzhalter.
- Test-API bleibt hinter bestehender Basic Auth.

## Tests

### Unit-Tests

- Default-Konfiguration
- ntfy-Konfiguration parsen
- Validierungsfehler für URL, Topic, Priorität und Versuche
- Token wird in `Debug` redigiert
- leeres Token ist gültig
- URL-Zusammensetzung ohne doppelte Slashes
- Fehlertext wird weiterhin gekürzt

### WebUI/Smoke-Test

- neue Felder vorhanden
- Token-Feld ist leer und `type=password`
- Test-Button und API-Route vorhanden
- API-/Diagnoseausgabe enthält kein Token
- alte Gotify-Begriffe sind aus sichtbarer v1.1.0-Oberfläche entfernt

## Nicht Bestandteil von v1.1.0

- Discord, Telegram oder E-Mail
- mehrere parallele Provider
- frei definierbare Templates
- Benachrichtigungsverlauf
- Retry-Queue für ausgefallene Notification-Server
