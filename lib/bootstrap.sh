#!/usr/bin/env bash

source /app/lib/colors.sh
source /app/lib/logger.sh
source /app/lib/config.sh
source /app/lib/state.sh
source /app/lib/extractor.sh

startup() {

    mkdir -p "$LOG_DIR"
    mkdir -p "$STATE_DIR"
    mkdir -p "${STATE_DIR}/releases"
    log_success "XDCC Extractor gestartet"

    load_config
    load_extractors

}
