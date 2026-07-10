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

    printf "Prüfe %-32s " "$path"

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

    printf "Prüfe %-32s " "$path"

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
        echo "Erwarteter Inhalt wurde nicht gefunden:"
        echo "$expected"
        exit 1
    fi

    echo "OK"
}

check_json() {
    local path="$1"

    printf "Prüfe %-32s " "$path"

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

check_asset() {
    local path="$1"

    printf "Prüfe %-32s " "$path"

    local response

    response="$(
        curl \
            --fail \
            --silent \
            --show-error \
            --user "${AUTH_USER}:${AUTH_PASSWORD}" \
            "${BASE_URL}${path}"
    )"

    if [[ -z "$response" ]]; then
        echo "FEHLER"
        echo "Asset ist leer: $path"
        exit 1
    fi

    echo "OK"
}


check_contains() {
    local path="$1"
    local expected="$2"
    local label="$3"

    printf "Prüfe %-32s " "$label"

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
        echo "Erwarteter Inhalt wurde nicht gefunden:"
        echo "Pfad: $path"
        echo "Inhalt: $expected"
        exit 1
    fi

    echo "OK"
}

echo "XDCC Extractor WebUI Smoke-Test"
echo "Ziel: $BASE_URL"
echo

echo "== Öffentlicher Endpunkt =="
check_public "/health"

echo
echo "== WebUI-Seiten =="
check_page "/" "<h1>XDCC Extractor</h1>"
check_page "/settings" "<h1>Einstellungen</h1>"
check_page "/settings/edit" "<h1>Einstellungen bearbeiten</h1>"
check_page "/logs" "<h1>Logs</h1>"
check_page "/diagnostics" "<h1>Diagnose</h1>"
check_page "/diagnostics" "<h2>Sicherungen</h2>"
check_page "/diagnostics" "<h2>Speicherorte und Speicherplatz</h2>"

echo
echo "== API-Endpunkte =="
check_json "/api/status"
check_json "/api/config"
check_json "/api/scan"
check_json "/api/failures"
check_json "/api/logs"

echo
echo "== WebUI-Assets =="
check_asset "/assets/common.css"
check_asset "/assets/dashboard.css"
check_asset "/assets/settings.css"
check_asset "/assets/settings-edit.css"
check_asset "/assets/logs.css"
check_asset "/assets/app.js"

echo
echo
echo "== UX- und Auto-Refresh-Merkmale =="

check_contains \
    "/" \
    'id="toast-region"' \
    "Dashboard Toast-Bereich"

check_contains \
    "/" \
    'id="auto-refresh-status"' \
    "Dashboard Auto-Refresh"

check_contains \
    "/assets/app.js" \
    "function showToast(" \
    "JavaScript Toasts"

check_contains \
    "/assets/app.js" \
    "function setButtonBusy(" \
    "JavaScript Ladezustände"

check_contains \
    "/assets/app.js" \
    "AUTO_REFRESH_INTERVAL_MS = 30000" \
    "Auto-Refresh Intervall"

check_contains \
    "/assets/app.js" \
    "visibilitychange" \
    "Pause bei inaktivem Tab"

check_contains \
    "/assets/dashboard.css" \
    ".toast-region {" \
    "Toast-Styles"

check_contains \
    "/assets/dashboard.css" \
    ".button.is-loading {" \
    "Dashboard Ladeanimation"

check_contains \
    "/assets/dashboard.css" \
    ".auto-refresh-status {" \
    "Auto-Refresh Styles"

check_contains \
    "/settings/edit" \
    'class="restart-status"' \
    "Neustart-Status"

check_contains \
    "/assets/settings-edit.css" \
    ".notice.restart-required {" \
    "Neustart-Hinweis"

check_contains \
    "/assets/settings-edit.css" \
    "settings-button-spin" \
    "Formular Ladeanimation"

check_contains \
    "/assets/settings.css" \
    ".disk-meter {" \
    "Speicherplatz Styles"

check_page \
    "/settings" \
    "Überwachte Ordner"

check_contains \
    "/api/config" \
    '"directories":' \
    "API Watch-Ordnerliste"

check_contains \
    "/api/status" \
    '"watch_directories":' \
    "Status Watch-Ordnerliste"

check_contains \
    "/assets/common.css" \
    ".watch-directory-list {" \
    "Watch-Ordner Styles"

check_page \
    "/settings/edit" \
    'name="watch_directories"'

check_page \
    "/settings/edit" \
    "Ein absoluter Container-Pfad pro Zeile"

echo "Alle WebUI-Tests erfolgreich."
