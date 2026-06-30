#!/usr/bin/env bash

load_config() {

    check_directory "$WATCH_DIR"
    check_directory "$LOG_DIR"
    check_directory "$STATE_DIR"

}
