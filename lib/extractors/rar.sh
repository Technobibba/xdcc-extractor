#!/usr/bin/env bash

rar_supported() {

    find "$1" -maxdepth 1 \
        -iname "*.rar" \
        | head -1

}

extract_rar() {

    local RELEASE="$1"

    local FIRST

    FIRST=$(rar_supported "$RELEASE")

    [ -z "$FIRST" ] && return 1

    local TMP

    TMP="$RELEASE/.extracting"

    rm -rf "$TMP"

    mkdir -p "$TMP"

    log_info "Extracting"

    log_info "$FIRST"

}
