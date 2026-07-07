# Release Checkliste

## Vor jedem Release

~~~bash
cargo fmt
cargo test
cargo build
~~~

## Secret- und Public-Check

~~~bash
./scripts/publication-check.sh
~~~

## Docker prüfen

~~~bash
docker compose down
docker compose build
docker compose up -d

sleep 10

docker inspect --format='{{.State.Health.Status}}' xdcc-extractor
docker compose logs --tail=100
~~~

## WebUI prüfen

- `/`
- `/settings`
- `/settings/edit`
- `/logs`
- `/health`

## Vor GitHub-Veröffentlichung prüfen

Diese Dateien dürfen nicht getrackt sein:

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

Prüfen:

~~~bash
git ls-files | grep -Ei '(^|/)(\.env|config\.toml|config\.docker\.toml|config\.env|passwords\.txt)$|(^|/)(state|logs|target)/'
~~~

Wenn keine Ausgabe kommt, ist es gut.

## Version setzen

~~~bash
cargo test
git status --short
~~~

Dann Version in `Cargo.toml` und `CHANGELOG.md` anpassen.

## Commit und Tag

~~~bash
git add Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "Bump version to X.Y.Z"

git tag -a vX.Y.Z -m "XDCC Extractor vX.Y.Z"
~~~

## Optional Push

~~~bash
git push origin master
git push origin vX.Y.Z
~~~

Falls dein Branch `main` heißt:

~~~bash
git push origin main
git push origin vX.Y.Z
~~~
