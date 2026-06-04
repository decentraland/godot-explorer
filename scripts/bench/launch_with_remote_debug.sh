#!/usr/bin/env bash
# launch_with_remote_debug.sh — start Godot editor in --debug-server mode +
# adb reverse + launch APK with --remote-debug baked in. Connects the running
# A54 to the editor's Visual Profiler / Debugger panel.
#
# Prereq: APK was exported with command_line/extra_args="--remote-debug tcp://127.0.0.1:6007"
# in godot/export_presets.cfg.

set -euo pipefail
SERIAL="${ANDROID_SERIAL:-100.64.0.9:37055}"
PORT="${REMOTE_DEBUG_PORT:-6007}"
PROJECT_DIR="$(cd "$(dirname "$0")/../.." && pwd)/godot"
GODOT_BIN="$(cd "$(dirname "$0")/../.." && pwd)/.bin/godot/godot4_bin"

# 1. Editor in debug-server mode (only if not already listening)
if ! lsof -nP -iTCP:$PORT -sTCP:LISTEN 2>/dev/null | grep -q .; then
  echo "[debug] starting Godot editor with --debug-server tcp://127.0.0.1:$PORT"
  nohup "$GODOT_BIN" --editor --debug-server "tcp://127.0.0.1:$PORT" --path "$PROJECT_DIR" \
    > /tmp/godot_editor.log 2>&1 &
  for _ in {1..15}; do
    sleep 1
    if lsof -nP -iTCP:$PORT -sTCP:LISTEN 2>/dev/null | grep -q .; then
      echo "[debug] editor listening on 127.0.0.1:$PORT"
      break
    fi
  done
else
  echo "[debug] editor already listening on $PORT"
fi

# 2. adb reverse so device's localhost:PORT → host's localhost:PORT
adb connect "$SERIAL" >/dev/null 2>&1 || true
adb -s "$SERIAL" reverse "tcp:$PORT" "tcp:$PORT"
echo "[debug] adb reverse: $(adb -s "$SERIAL" reverse --list)"

echo "[debug] ready — launch the bench with your usual deeplink"
echo "[debug] e.g.:"
echo "  bash scripts/bench/launch_devices.sh --android --gp-benchmark \\"
echo "    --realm http://100.64.0.3:8000 \\"
echo "    --param force-graphic-profile=0 --param kill-sky=true --param skip-gltf=true"
