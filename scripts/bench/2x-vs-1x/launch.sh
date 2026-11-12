#!/usr/bin/env bash
# Cold-launch a build into a fixed place, and report when the device presented the first frame
# and when the engine reported its startup milestones (#3206).
#
#   launch.sh 1x "decentraland://open?position=0,0"
#   launch.sh 2x "decentraland://open?position=0,0"
#
# Writes nothing; prints a JSON object with the timings and the [Startup]/[RenderReady] lines.
set -euo pipefail

BUILD=${1:?build (1x|2x)}
URI=${2:?deeplink uri}
SERIAL=${ANDROID_SERIAL:-R5CW60624NE}
PKG=org.decentraland.godotexplorer
ACTIVITY=$PKG/com.godot.game.GodotAppLauncher
WAIT_READY=${WAIT_READY:-180}

adb -s "$SERIAL" shell am force-stop "$PKG"
adb -s "$SERIAL" shell logcat -c
T0=$(python3 -c 'import time;print(int(time.time()*1000))')
START_OUT=$(adb -s "$SERIAL" shell am start -W -a android.intent.action.VIEW -d "'$URI'" "$PKG" 2>&1 || true)
T_START=$(python3 -c 'import time;print(int(time.time()*1000))')
TOTAL_TIME=$(echo "$START_OUT" | sed -n 's/.*TotalTime: \([0-9]*\).*/\1/p' | head -1)

T_READY=""
for _ in $(seq 1 "$WAIT_READY"); do
  if adb -s "$SERIAL" shell logcat -d -v time 2>/dev/null | grep -q "\[RenderReady\]"; then
    T_READY=$(python3 -c 'import time;print(int(time.time()*1000))')
    break
  fi
  sleep 1
done

LOG=$(adb -s "$SERIAL" shell logcat -d -v time 2>/dev/null | grep -E "\[Startup\]|\[RenderReady\]|\[DEEPLINK\]|\[MemMonitor\]|\[Perf\]" || true)
READY_LINE=$(echo "$LOG" | grep "\[RenderReady\]" | tail -1)
STARTUP_LINES=$(echo "$LOG" | grep "\[Startup\]" || true)

python3 -c '
import json, sys
build, uri, total, t_start, t_ready = sys.argv[1:6]
print(json.dumps({
    "build": build,
    "uri": uri,
    "am_total_time_ms": int(total) if total else None,
    "t_render_ready_offset_from_start_ms": (int(t_ready) - int(t_start)) if t_ready else None,
}, indent=2))
' "$BUILD" "$URI" "$TOTAL_TIME" "$T_START" "$T_READY"
echo "--- engine startup ---"
echo "$STARTUP_LINES"
echo "--- render ready ---"
echo "$READY_LINE"
echo "--- am start ---"
echo "$START_OUT" | grep -E "Status|TotalTime|WaitTime|Error" || true
