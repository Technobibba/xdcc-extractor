#!/bin/bash

set -euo pipefail

source /config.env 2>/dev/null || true

WATCH_DIR="${WATCH_DIR:-/downloads}"
LOGFILE="${LOGFILE:-/logs/unpacker.log}"
CHECK_INTERVAL="${CHECK_INTERVAL:-10}"
STABLE_TIME="${STABLE_TIME:-30}"

mkdir -p "$(dirname "$LOGFILE")"

log() {

    echo "$(date '+%F %T') | $1" | tee -a "$LOGFILE"

}

startup() {

    log "====================================="
    log "XDCC Extractor gestartet"
    log "Watch: $WATCH_DIR"
    log "====================================="

}

scan_existing() {

    log "Suche vorhandene Releases..."

}

watch_loop() {

    log "Überwachung gestartet"

    while true
    do

        sleep "$CHECK_INTERVAL"

    done

}

startup

scan_existing

watch_loop
