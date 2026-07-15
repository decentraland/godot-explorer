#!/usr/bin/env bash
# Drive the UNIFIED scene-inspector channel (eval + tree queries) through the
# debug-hub consumer port. One request → its matching ACK.
#
# This speaks the scene-inspector CMD protocol (the source-of-truth contract an
# external inspector app already uses) — the single transport for the live
# client, reachable on any platform (incl. iOS device) because the device dials
# OUT to the hub.
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
#
# Keep stdin open (the trailing sleep) until the ACK returns: `grep -m1` exits on
# the first match and SIGPIPEs the rest of the pipeline, so a fast reply tears the
# whole thing down immediately and the sleep is only a latency ceiling. Without
# it, websocat closes the socket on stdin EOF and can miss a slower on-device
# reply (loopback desktop is sub-ms, but USB/adb + LAN round-trips are not).
reply="$(
  { printf '%s\n' "$frame"; sleep "${REPLY_TIMEOUT:-10}"; } \
    | websocat -B 16777216 "ws://$HOST:$PORT" 2>/dev/null \
    | grep -m1 "\"id\":\"$id\"" || true
)"
if [ -n "$reply" ]; then
  printf '%s\n' "$reply"
else
  echo "unified.sh: no reply for id=$id within ${REPLY_TIMEOUT:-10}s — is a device connected to the hub (device port 9231)?" >&2
  exit 1
fi
