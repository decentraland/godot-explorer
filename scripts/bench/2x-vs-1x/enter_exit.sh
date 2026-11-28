#!/usr/bin/env bash
# Enter and leave the same scene N times on one build (#3206 stress): the cost of each cycle is the
# cold load, and the leak signal is PSS drifting run to run. Each cycle force-stops the process so
# the next one starts from zero, which is what "entering the scene again" costs a player.
#
#   enter_exit.sh 2x 20
set -euo pipefail

BUILD=${1:?build (1x|2x)}
CYCLES=${2:-20}
SERIAL=${ANDROID_SERIAL:-R5CW60624NE}
PKG=org.decentraland.godotexplorer
ROOT=${ROOT:-docs/bench/data/2x-vs-1x-3206/android-a54/enter-exit}
OUT="$ROOT/$BUILD"
mkdir -p "$OUT"
CSV="$OUT/cycles.csv"
[ -f "$CSV" ] || echo "cycle,version,load_ms,pss_kb,footprint_mb,level" > "$CSV"

version=$(adb -s "$SERIAL" shell dumpsys package "$PKG" | sed -n 's/.*versionName=//p' | head -1)

for i in $(seq 1 "$CYCLES"); do
  adb -s "$SERIAL" shell am force-stop "$PKG"
  adb -s "$SERIAL" shell logcat -c
  t0=$(python3 -c 'import time;print(int(time.time()*1000))')
  adb -s "$SERIAL" shell am start -W -a android.intent.action.VIEW \
    -d "'decentraland://open?position=0,0'" "$PKG" >/dev/null 2>&1 || true
  ok=0
  for _ in $(seq 1 120); do
    if adb -s "$SERIAL" shell logcat -d 2>/dev/null | grep -q "\[RenderReady\]"; then ok=1; break; fi
    sleep 1
  done
  t1=$(python3 -c 'import time;print(int(time.time()*1000))')
  [ "$ok" = "1" ] || { echo "cycle $i: no RenderReady, aborting" >&2; break; }
  # stand still long enough for the scene to finish settling before sampling
  adb -s "$SERIAL" shell input tap 1200 2000 >/dev/null 2>&1 || true
  for _ in $(seq 1 15); do sleep 1; done
  pss=$(adb -s "$SERIAL" shell dumpsys meminfo "$PKG" | sed -n 's/.*TOTAL PSS:[[:space:]]*\([0-9]*\).*/\1/p' | head -1)
  fp=$(adb -s "$SERIAL" shell logcat -d 2>/dev/null | grep "\[MemMonitor\] available=" | tail -1 \
        | sed -n 's/.*footprint=\([0-9]*\)MB.*/\1/p')
  lvl=$(adb -s "$SERIAL" shell logcat -d 2>/dev/null | grep "\[MemMonitor\] available=" | tail -1 \
        | sed -n 's/.*level=\([0-9]*\).*/\1/p')
  echo "$i,$version,$((t1-t0)),${pss:-},${fp:-},${lvl:-}" >> "$CSV"
  echo "cycle $i: load=$((t1-t0))ms pss=${pss:-?}KB footprint=${fp:-?}MB level=${lvl:-?}"
  adb -s "$SERIAL" exec-out screencap -p > "$OUT/cycle-$(printf %02d "$i").png" || true
done
