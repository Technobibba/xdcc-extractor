#!/usr/bin/env bash

VERIFY_DELAY=30

release_ready() {

    local RELEASE="$1"

    local LAST

    LAST=$(find "$RELEASE" \
        -type f \
        -printf "%T@\n" \
        | sort -n \
        | tail -1 \
        | cut -d. -f1)

    NOW=$(date +%s)

    AGE=$((NOW-LAST))

    if [ "$AGE" -ge "$VERIFY_DELAY" ]
    then
        return 0
    fi

    return 1

}
