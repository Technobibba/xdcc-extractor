# Release-Checkliste

## 1. Arbeitsbaum prüfen

~~~bash
git status --short
~~~

Der Arbeitsbaum muss vor der Versionsänderung sauber sein.

## 2. Version vorbereiten

Die neue Version muss übereinstimmend eingetragen werden:

~~~text
Cargo.toml
Cargo.lock
CHANGELOG.md
~~~

Der bisherige Bereich `Unreleased` wird in einen
Versionsabschnitt umgewandelt:

~~~text
## vX.Y.Z - YYYY-MM-DD
~~~

## 3. Lokale Prüfungen

~~~bash
cargo fmt --check
cargo test
cargo build
./scripts/webui-smoke-test.sh
./scripts/publication-check.sh
git diff --check
~~~

## 4. Docker lokal prüfen

~~~bash
docker compose down
docker compose build
docker compose up -d

sleep 10

docker inspect   --format='{{.State.Health.Status}}'   xdcc-extractor

docker compose logs   --tail=100
~~~

Erwartet:

~~~text
healthy
~~~

## 5. WebUI prüfen

Mindestens diese Seiten öffnen:

~~~text
/
 /settings
/settings/edit
/logs
/diagnostics
/health
~~~

Zusätzlich prüfen:

- Watch-Ordner werden korrekt angezeigt
- Scan und manuelle Verarbeitung funktionieren
- Logs werden geladen
- Speicherplatzwerte werden angezeigt
- keine Secrets werden dargestellt
- Versionsnummer stimmt

## 6. Release-Commit

~~~bash
git add   Cargo.toml   Cargo.lock   CHANGELOG.md   README.md   ROADMAP.md

git commit   -m "Release vX.Y.Z"

git push
~~~

## 7. Versions-Tag erstellen

~~~bash
git tag   -a vX.Y.Z   -m "XDCC Extractor vX.Y.Z"

git push   origin   vX.Y.Z
~~~

Der Tag startet automatisch:

~~~text
Rust-Prüfungen
Publication-Check
Docker-Build
GHCR-Veröffentlichung
GitHub-Release
~~~

## 8. GitHub Actions prüfen

Im Release-Workflow müssen alle Schritte erfolgreich sein:

~~~text
Validate tag and Cargo version
Check formatting
Run tests
Build release binary
Run publication check
Build and publish Docker image
Create GitHub release
~~~

## 9. GitHub-Release prüfen

Prüfen:

- Titel entspricht `XDCC Extractor vX.Y.Z`
- Changelog ist vollständig
- Release ist als neuestes Release markiert
- Versions-Tag verweist auf den Release-Commit

## 10. GHCR-Paket prüfen

Erwartete Image-Tags:

~~~text
vX.Y.Z
X.Y.Z
X.Y
X
latest
~~~

Nach der ersten Veröffentlichung zusätzlich prüfen:

- Paket ist mit dem Repository verknüpft
- Paketsichtbarkeit ist für die gewünschte Nutzung korrekt
- öffentliches Pull ohne Anmeldung funktioniert

Beispiel:

~~~bash
docker pull   ghcr.io/technobibba/xdcc-extractor:X.Y.Z
~~~

## 11. Installationstest mit GHCR-Image

~~~bash
cp   compose.ghcr.example.yaml   compose.ghcr.yaml

docker compose   -f compose.ghcr.yaml   pull

docker compose   -f compose.ghcr.yaml   up -d
~~~

Danach Healthcheck und WebUI prüfen.

## 12. Abschluss

~~~bash
git status --short

git log   -3   --oneline   --decorate

git tag   --sort=-version:refname   | head
~~~
