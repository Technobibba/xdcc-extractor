#!/usr/bin/env bash

watcher_start() {

    log_info "Starting filesystem watcher..."

    inotifywait \
        -m \
        -r \
        -e create \
        -e moved_to \
        -e close_write \
        "$WATCH_DIR" |
    while read -r DIR EVENT FILE
    do

        release_detect "${DIR}${FILE}"

    done

}
