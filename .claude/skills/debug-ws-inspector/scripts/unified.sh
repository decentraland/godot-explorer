#!/usr/bin/env bash
# Drive the UNIFIED scene-inspector channel (eval + tree queries) through the
# debug-hub consumer port. One request → its matching ACK.
#
# This speaks the scene-inspector CMD protocol (the source-of-truth contract an
# external inspector app already uses), NOT the loopback DebugWs `{id,cmd}` form
# that debug-ws.sh uses. Same command surface, but reachable on any platform
# (incl. iOS device) because the device dials OUT to the hub.
#
# Prereqs: `cargo run -- debug-hub` running; the client launched with
#   --scene-inspector=ws://<this-mac>:9231 (the hub's device port).
#
# Usage:
#   unified.sh ping
#   unified.sh scenes
#   unified.sh scene  '{"scene_id":0,"filters":{"component":["Transform"]}}'
#   unified.sh avatar '{"by":"local"}'
#   unified.sh eval   'Engine.get_frames_per_second()'
#   PORT=9230 HOST=127.0.0.1 unified.sh focus
set -euo pipefail

PORT="${PORT:-9230}"
HOST="${HOST:-127.0.0.1}"
cmd="${1:?usage: unified.sh <cmd> [args-json | eval-code]}"
shift || true

id="req-${RANDOM}${RANDOM}"
if [ "$cmd" = "eval" ]; then
  # JSON-escape the snippet into {"code": "..."}.
  args="$(printf '%s' "${1:-}" | python3 -c 'import json,sys; print(json.dumps({"code": sys.stdin.read()}))')"
else
  args="${1:-{}}"
fi

frame="$(printf '{"type":"SCENE_INSPECTOR_CMD","cmd":"%s","args":%s,"id":"%s"}' "$cmd" "$args" "$id")"

# The channel also streams SCENE_INSPECTOR frames (logs/crdt/…); filter for our
# ACK by id so we return exactly the reply to this request.
printf '%s\n' "$frame" \
  | websocat -B 16777216 "ws://$HOST:$PORT" \
  | grep -m1 "\"id\":\"$id\""
