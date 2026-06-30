#!/usr/bin/env bash

STATE_RELEASE_DIR="${STATE_DIR}/releases"

state_file() {

    local RELEASE="$1"

    RELEASE=$(basename "$RELEASE")

    echo "${STATE_RELEASE_DIR}/${RELEASE}.state"

}

state_exists() {

    [ -f "$(state_file "$1")" ]

}

state_create() {

    local FILE

    FILE=$(state_file "$1")

    cat > "$FILE" <<EOF
STATUS=ACTIVE
LAST_CHANGE=$(date +%s)
CREATED=$(date +%s)
FILES=0
EOF

}

state_set() {

    local FILE

    FILE=$(state_file "$1")

    local KEY="$2"

    local VALUE="$3"

    if grep -q "^${KEY}=" "$FILE"; then
        sed -i "s|^${KEY}=.*|${KEY}=${VALUE}|" "$FILE"
    else
        echo "${KEY}=${VALUE}" >> "$FILE"
    fi

}

state_get() {

    local FILE

    FILE=$(state_file "$1")

    local KEY="$2"

    grep "^${KEY}=" "$FILE" | cut -d= -f2-
}
