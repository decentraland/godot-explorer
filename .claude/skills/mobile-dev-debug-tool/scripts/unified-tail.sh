#!/usr/bin/env bash
# Subscribe to opt-in streams and tail the UNIFIED channel through the debug-hub.
# Nothing is captured on the device until you subscribe (prod stays ~zero-cost),
# so this both subscribes and streams.
#
# Usage:
#   unified-tail.sh                 # logs only (default)
#   unified-tail.sh log,network     # logs + HTTP
#   unified-tail.sh log,lifecycle   # logs + per-tick scene lifecycle
#   PORT=9230 unified-tail.sh log | grep -i error
#
# Streams: log, network, lifecycle  (crdt/perf flow by default once connected).
# Ctrl-C to stop (this also unsubscribes on disconnect — capture stops device-side).
set -euo pipefail

PORT="${PORT:-9230}"
HOST="${HOST:-127.0.0.1}"
streams="${1:-log}"

arr="$(printf '%s' "$streams" | python3 -c 'import json,sys; print(json.dumps([s for s in sys.stdin.read().strip().split(",") if s]))')"
sub="$(printf '{"type":"SCENE_INSPECTOR_CMD","cmd":"subscribe","args":{"streams":%s},"id":"sub"}' "$arr")"

# Send the subscribe, then keep stdin open (cat) so the socket stays connected
# and we keep receiving SCENE_INSPECTOR frames.
{ printf '%s\n' "$sub"; cat; } | websocat -B 16777216 "ws://$HOST:$PORT"
