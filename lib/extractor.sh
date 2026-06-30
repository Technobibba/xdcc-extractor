#!/usr/bin/env bash

load_extractors() {

    for FILE in /app/lib/extractors/*.sh
    do
        [ -f "$FILE" ] || continue
        source "$FILE"
    done

}

extract_release() {

    local RELEASE="$1"

    local RAR

    RAR=$(rar_supported "$RELEASE")

    if [ -n "$RAR" ]
    then
        extract_rar "$RELEASE"
        return
    fi

    log_error "No extractor found"

}
