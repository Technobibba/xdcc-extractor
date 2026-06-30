#!/usr/bin/env bash

check_directory() {

    if [ ! -d "$1" ]; then

        echo

        echo "Directory not found"

        echo "$1"

        exit 1

    fi

}
