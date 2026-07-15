# Migration auf XDCC Extractor v1.1.0

## Vor dem Update

1. Laufende `config.docker.toml` sichern.
2. Gotify-URL und Token nicht in neue Beispielkonfigurationen übernehmen.
3. In ntfy einen eigenen Access Token für den XDCC Extractor erstellen.
4. Dem Token Schreibrechte auf das gewünschte Topic geben.

## Neue Konfiguration

Die alte Sektion:

```toml
[notifications.gotify]
```

wird ersetzt durch:

```toml
[notifications]
enabled=true
provider="ntfy"

[notifications.ntfy]
server="https://ntfy.example.org"
topic="homelab-downloads"
token=""
priority_success=3
priority_error=5
notify_on_success=true
notify_on_error=true
notify_on_every_error=false
notify_after_attempts=3
```

Alternativ erfolgt die Einrichtung nach dem Start über **Einstellungen → Bearbeiten → Benachrichtigungen**.

## Verhalten beim ersten Start

Eine vorhandene Gotify-Sektion wird erkannt, aber nicht automatisch übernommen. ntfy bleibt deaktiviert, bis Server und Topic eingerichtet wurden.

## Rückweg

- Vor dem Update erstellte Config-Sicherung wiederherstellen.
- Container mit dem vorherigen v1.0.x-Image starten.
- Bei lokalem Build den vorherigen Git-Tag auschecken.
