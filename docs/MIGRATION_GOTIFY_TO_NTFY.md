# Migration von Gotify zu ntfy

XDCC Extractor v1.1.0 verwendet ntfy anstelle von Gotify. Die alte Sektion `[notifications.gotify]` wird nicht mehr ausgewertet.

## Vor dem Update

1. Bestehende `config.docker.toml` sichern.
2. Einen ntfy-Server, ein Topic und optional einen Zugriffstoken vorbereiten.
3. Den Feature-Branch zunächst mit einer Testnachricht prüfen.

## Alte Konfiguration

```toml
[notifications.gotify]
enabled=true
url="https://gotify.example.org"
token=""
priority_success=3
priority_error=8
notify_on_success=true
notify_on_error=true
notify_on_every_error=false
notify_after_attempts=3
```

## Neue Konfiguration

```toml
[notifications]
enabled=true
provider="ntfy"

[notifications.ntfy]
server="https://ntfy.example.org"
topic="xdcc-extractor"
token=""
priority_success=3
priority_error=5
notify_on_worker_start=false
notify_on_processing_start=false
notify_on_success=true
notify_on_error=true
notify_on_every_error=false
notify_after_attempts=3
```

Ein leerer Token ist zulässig, wenn das Topic anonym beschrieben werden darf.

## Migration über die WebUI

1. `/settings/edit` öffnen.
2. Benachrichtigungen aktivieren.
3. ntfy-Server, Topic und optionalen Token eintragen.
4. Einstellungen speichern.
5. Anschließend **Testnachricht senden** wählen.
6. Erst nach erfolgreichem Test die alte Gotify-Sektion aus der Runtime-Konfiguration entfernen.

## Prioritäten

ntfy verwendet Prioritäten von 1 bis 5. Ein früherer Gotify-Wert wie `8` oder `9` muss daher auf höchstens `5` reduziert werden.

## Rückweg

Vor jeder WebUI-Änderung wird die Konfigurationsdatei automatisch gesichert. Zusätzlich sollte vor dem Update eine manuelle Kopie der produktiven Config erstellt werden.
