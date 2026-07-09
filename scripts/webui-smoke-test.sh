#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${1:-http://192.168.2.184:8099}"

if [[ -f ".env" ]]; then
    set -a
    # shellcheck disable=SC1091
    source .env
    set +a
fi

AUTH_USER="${XDCC_WEB_AUTH_USER:-admin}"
AUTH_PASSWORD="${XDCC_WEB_AUTH_PASSWORD:-}"

if [[ -z "$AUTH_PASSWORD" ]]; then
    echo "FEHLER: XDCC_WEB_AUTH_PASSWORD ist nicht gesetzt."
    exit 1
fi

check_public() {
    local path="$1"

    printf "Prüfe %-24s " "$path"

    curl \
        --fail \
        --silent \
        --show-error \
        "${BASE_URL}${path}" \
        >/dev/null

    echo "OK"
}

check_page() {
    local path="$1"
    local expected="$2"

    printf "Prüfe %-24s " "$path"

    local response

    response="$(
        curl \
            --fail \
            --silent \
            --show-error \
            --user "${AUTH_USER}:${AUTH_PASSWORD}" \
            "${BASE_URL}${path}"
    )"

    if ! grep -Fq "$expected" <<<"$response"; then
        echo "FEHLER"
        echo "Erwarteter Inhalt wurde nicht gefunden: $expected"
        exit 1
    fi

    echo "OK"
}

check_json() {
    local path="$1"

    printf "Prüfe %-24s " "$path"

    curl \
        --fail \
        --silent \
        --show-error \
        --user "${AUTH_USER}:${AUTH_PASSWORD}" \
        "${BASE_URL}${path}" \
        | python3 -m json.tool \
        >/dev/null

    echo "OK"
}

echo "XDCC Extractor WebUI Smoke-Test"
echo "Ziel: $BASE_URL"
echo

check_public "/health"

check_page "/" "<h1>XDCC Extractor</h1>"
check_page "/settings" "<h1>Einstellungen</h1>"
check_page "/settings/edit" "<h1>Einstellungen bearbeiten</h1>"
check_page "/logs" "<h1>Logs</h1>"

check_json "/api/status"
check_json "/api/config"
check_json "/api/scan"
check_json "/api/failures"
check_json "/api/logs"

echo
echo "Alle WebUI-Tests erfolgreich."
