#!/usr/bin/env bash

set -euo pipefail

VERSION="0.1.0-alpha"

source /app/lib/bootstrap.sh
source /app/lib/monitor.sh
source /app/lib/release.sh

startup

watcher_start
