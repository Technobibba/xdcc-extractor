#!/bin/sh
set -eu

test -x /usr/local/bin/xdcc-extractor
test -f /app/config.toml
test -d /downloads
test -d /state

if grep -q 'password_file="/config/passwords.txt"' /app/config.toml; then
    test -d /config
fi

exit 0
