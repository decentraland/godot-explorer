#!/usr/bin/env bash
# iOS measurements reachable without UI automation (#3206): cold launch to the first interactive
# screen (the entry shell) and the memory footprint the engine logs while it stays there.
#
# The iPhone has no `adb` and no input injection on this machine (no idb/XCUITest), so the entry
# flow cannot be driven: nothing here reaches the world. See the report's limitations.
#
#   ios_launch.sh 1x 5      # 1.x-mode build (Godot app)
#   ios_launch.sh 2x 5      # React Native build
set -euo pipefail

BUILD=${1:?1x|2x}
RUNS=${2:-5}
DEVICE=${IOS_UDID:-1DD7B8A8-BECD-512A-93F5-11A7ABBD2CBC}
ROOT=${ROOT:-docs/bench/data/2x-vs-1x-3206/iphone16e/launch}
mkdir -p "$ROOT/$BUILD"

for r in $(seq 1 "$RUNS"); do
  log="$ROOT/$BUILD/console-$r.log"
  t0=$(python3 -c 'import time;print(time.time())')
  ( xcrun devicectl device process launch --device "$DEVICE" --terminate-existing \
      --payload-url "decentraland://open?position=0,0" --console org.decentraland.godotexplorer \
      > "$log" 2>&1 & echo $! > /tmp/ios-launch.pid )
  # wait for the first engine milestone, then let it settle so the footprint is steady
  for _ in $(seq 1 90); do
    grep -q "MemMonitor\] available=" "$log" 2>/dev/null && break
    sleep 0.5
  done
  t1=$(python3 -c 'import time;print(time.time())')
  sleep 25
  kill "$(cat /tmp/ios-launch.pid)" 2>/dev/null || true
  xcrun devicectl device process terminate --device "$DEVICE" org.decentraland.godotexplorer >/dev/null 2>&1 || true
  python3 - "$BUILD" "$r" "$t0" "$t1" "$log" <<'PY' > "$ROOT/$BUILD/launch-$r.json"
import json, re, sys
build, run, t0, t1, log = sys.argv[1:6]
text = open(log, errors="replace").read()
foot = [int(m) for m in re.findall(r"\[MemMonitor\] available=\d+MB footprint=(\d+)MB", text)]
print(json.dumps({
    "build": build,
    "run": int(run),
    "cold_launch_to_engine_ready_ms": round((float(t1) - float(t0)) * 1000),
    "memmonitor_footprint_samples_mb": foot,
    "memmonitor_footprint_steady_mb": foot[len(foot) // 2] if foot else None,
    "startup_lines": [l for l in text.splitlines() if "[Startup]" in l][:6],
    "render_ready": [l for l in text.splitlines() if "[RenderReady]" in l][:2],
}, indent=2))
PY
  cat "$ROOT/$BUILD/launch-$r.json"
done
