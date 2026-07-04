#!/bin/sh
set -eu

PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"

echo "== XDCC Extractor Status =="
echo

cd "$PROJECT_DIR"

echo "Projekt:"
echo "  Pfad: $PROJECT_DIR"
echo

echo "Git:"
git status --short || true
echo

echo "Docker:"
docker compose ps || true
echo

echo "Container Health:"
docker ps --filter "name=xdcc-extractor" --format "table {{.Names}}\t{{.Status}}" || true
echo

echo "Mounts:"
docker inspect xdcc-extractor --format '{{range .Mounts}}{{.Source}} -> {{.Destination}}{{println}}{{end}}' 2>/dev/null || true
echo

echo "Config:"
if [ -f config.docker.toml ]; then
    echo "  config.docker.toml: OK"
else
    echo "  config.docker.toml: FEHLT"
fi

if grep -q 'password_file="/config/passwords.txt"' config.docker.toml 2>/dev/null; then
    if [ -f config/passwords.txt ]; then
        COUNT="$(grep -v '^[[:space:]]*$' config/passwords.txt | grep -v '^[[:space:]]*#' | wc -l)"
        echo "  Passwortdatei: OK ($COUNT Einträge)"
    else
        echo "  Passwortdatei: FEHLT"
    fi
fi
echo

echo "Verzeichnisse:"
for dir in /media/HDD3/XDCC /media/HDD3/XDCC/_extracted state; do
    if [ -e "$dir" ]; then
        echo "  $dir: OK"
    else
        echo "  $dir: FEHLT"
    fi
done
echo

echo "Letzte Logs:"
docker compose logs --tail=40 xdcc-extractor 2>/dev/null || true
