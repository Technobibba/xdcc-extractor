#!/usr/bin/env bash

queue_worker() {

    while true
    do

        for FILE in "$STATE_DIR"/releases/*.state
        do

            [ -f "$FILE" ] || continue

            RELEASE=$(basename "$FILE" .state)

            STATUS=$(grep STATUS "$FILE" | cut -d= -f2)

            if [ "$STATUS" = "ACTIVE" ]
            then

                RELEASE_PATH="${WATCH_DIR}/${RELEASE}"

                if release_ready "$RELEASE_PATH"
                then

                    log_success "$RELEASE ready"

                    state_set "$RELEASE" STATUS READY

                fi

            fi

        done

        sleep 5

    done

}
