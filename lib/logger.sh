#!/usr/bin/env bash

source /app/lib/colors.sh

LOGFILE="${LOG_DIR}/extractor.log"

timestamp() {
    date "+%Y-%m-%d %H:%M:%S"
}

write_log() {
    echo "$(timestamp) [$1] $2" >> "$LOGFILE"
}

log_info() {
    echo -e "${BLUE}[$(timestamp)] [INFO ]${RESET} $1"
    write_log INFO "$1"
}

log_warn() {
    echo -e "${YELLOW}[$(timestamp)] [WARN ]${RESET} $1"
    write_log WARN "$1"
}

log_error() {
    echo -e "${RED}[$(timestamp)] [ERROR]${RESET} $1"
    write_log ERROR "$1"
}

log_success() {
    echo -e "${GREEN}[$(timestamp)] [ OK  ]${RESET} $1"
    write_log OK "$1"
}
