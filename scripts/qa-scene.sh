#!/usr/bin/env bash
#
# Opens one scene test in QA mode, as the QA World shows it: a production build (no tests, the QA
# panel) served locally, with the client standing on the scene.
#
#   scripts/qa-scene.sh ui-dropdown                   opens it; Ctrl-C closes client and server
#   scripts/qa-scene.sh ui-dropdown --screenshot f.png  saves a screenshot once it loaded, then closes
#
# The screenshot goes through the debug hub (`cargo run -- debug-hub`), started here when none runs.
# It fails within a minute when the scene does not load, printing the client's state.
set -euo pipefail

NAME="${1:-}"
[ -n "$NAME" ] && [ "${NAME#-}" = "$NAME" ] || { echo "usage: $0 <scene name> [--screenshot <file.png>]" >&2; exit 2; }
shift
SHOT=""
while [ $# -gt 0 ]; do
  case "$1" in
    --screenshot) SHOT="$2"; shift 2 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
HUB="$ROOT/.claude/skills/mobile-dev-debug-tool/scripts/unified.sh"
GODOT="${GODOT4_BIN:-$ROOT/.bin/godot/godot4_bin}"
PORT=7667
REALM="http://localhost:$PORT/scene-explorer-tests"

PARCEL="$(python3 -I - "$ROOT/tests/scene-tests/scenes.json" "$NAME" <<'EOF'
import json, sys
scenes = {s["name"]: s["parcel"] for s in json.load(open(sys.argv[1]))["scenes"]}
if sys.argv[2] not in scenes:
    sys.exit(f"unknown scene {sys.argv[2]}; scenes: {', '.join(scenes)}")
print("%d,%d" % tuple(scenes[sys.argv[2]]))
EOF
)"

PIDS=()
cleanup() { for pid in ${PIDS[@]+"${PIDS[@]}"}; do kill "$pid" 2>/dev/null || true; done; }
trap cleanup EXIT INT TERM

# Not in the background: a build error stops here instead of a minute later.
"$ROOT/scripts/serve-scene-tests.sh" --production --build-only --port "$PORT"
"$ROOT/scripts/serve-scene-tests.sh" --production --port "$PORT" >/dev/null 2>&1 &
PIDS+=($!)
for _ in $(seq 1 30); do curl -fs "$REALM/about" >/dev/null && break; sleep 1; done
curl -fs "$REALM/about" >/dev/null || { echo "the QA realm did not come up on :$PORT" >&2; exit 1; }

ARGS=(--path "$ROOT/godot" --realm "$REALM" --location "$PARCEL")
if [ -n "$SHOT" ]; then
  if ! nc -z 127.0.0.1 9231 2>/dev/null; then
    (cd "$ROOT" && exec cargo run -q -- debug-hub) >/dev/null 2>&1 &
    PIDS+=($!)
    for _ in $(seq 1 60); do nc -z 127.0.0.1 9231 2>/dev/null && break; sleep 1; done
  # The hub answers through the first client that dialled in: a second one would be ignored.
  elif REPLY_TIMEOUT=2 "$HUB" ping >/dev/null 2>&1; then
    echo "a client is already connected to the debug hub; close it first" >&2
    exit 1
  fi
  ARGS+=(--scene-inspector=ws://127.0.0.1:9231)
fi

echo "Opening $NAME at $PARCEL in QA mode"
"$GODOT" "${ARGS[@]}" >"$ROOT/tests/scene-tests/qa-scene.log" 2>&1 &
CLIENT=$!
PIDS+=("$CLIENT")

if [ -z "$SHOT" ]; then
  wait "$CLIENT"
  exit 0
fi

state=""
for _ in $(seq 1 60); do
  kill -0 "$CLIENT" 2>/dev/null || { echo "the client exited; see tests/scene-tests/qa-scene.log" >&2; exit 1; }
  state="$(REPLY_TIMEOUT=2 "$HUB" state 2>/dev/null || true)"
  echo "$state" | grep -q "\"parcel\":\[${PARCEL}\]" && echo "$state" | grep -q '"scenes_loaded":[1-9]' && break
  sleep 1
done
echo "$state" | grep -q '"scenes_loaded":[1-9]' || { echo "the scene did not load; last state: $state" >&2; exit 1; }
# The scene's UI renders a few frames after it loads.
sleep 3
"$HUB" screenshot '{"inline":true}' | python3 -I -c '
import base64, json, sys
reply = json.loads(sys.stdin.read())
if not reply.get("ok"):
    sys.exit("screenshot failed: %s" % reply.get("error"))
open(sys.argv[1], "wb").write(base64.b64decode(reply["data"]["png_base64"]))
' "$SHOT"
echo "Saved $SHOT"
