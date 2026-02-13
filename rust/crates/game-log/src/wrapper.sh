#!/bin/bash
# CapyDeploy Game Wrapper — captures stdout/stderr to a log file.
# Injected into Steam launch options as:
#   /path/to/capydeploy-game-wrapper.sh APPID %command%
#
# First argument is the Steam appId (passed by the injector so non-Steam
# shortcuts get the correct ID instead of relying on $SteamAppId which is 0).
#
# The log file path is predictable: game_{APPID}_{TIMESTAMP}.log

APPID="${1:-unknown}"
shift

LOG_DIR="${HOME}/.local/share/capydeploy/logs"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
LOG_FILE="${LOG_DIR}/game_${APPID}_${TIMESTAMP}.log"

mkdir -p "$LOG_DIR"

# Run the game, redirecting stdout+stderr to log file.
# exec replaces this shell so Steam tracks the game PID correctly.
exec "$@" > "$LOG_FILE" 2>&1
