#!/usr/bin/env bash
# Send a single JSON frame to the debug WS server and print the reply.
# Bakes in -B 16777216 so websocat doesn't split large frames.
#
# Usage:
#   debug-ws.sh '{"id":1,"cmd":"ping"}'
#   echo '{"id":1,"cmd":"scenes"}' | debug-ws.sh

set -euo pipefail

if [ $# -ge 1 ]; then
  payload="$1"
else
  payload="$(cat)"
fi

printf '%s\n' "$payload" | websocat -n1 -B 16777216 --text ws://127.0.0.1:9230
