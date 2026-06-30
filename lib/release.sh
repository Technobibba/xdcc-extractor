#!/usr/bin/env bash

release_detect() {

    local FILE="$1"

    local RELEASE

    RELEASE=$(dirname "$FILE")

    if ! state_exists "$RELEASE"
    then

        log_success "New release detected"

        log_info "$RELEASE"

        state_create "$RELEASE"

    fi

    state_set "$RELEASE" LAST_CHANGE "$(date +%s)"

}
