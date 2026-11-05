#!/usr/bin/env bash
# RSS across the app's four states (#3206): menus, in world, after leaving the world, and after a
# second enter/exit cycle. Samples `dumpsys meminfo` for 15 s in each state, one file per state.
#
#   rss_states.sh 2x
#   rss_states.sh 1x
set -euo pipefail

BUILD=${1:?build (1x|2x)}
SERIAL=${ANDROID_SERIAL:-R5CW60624NE}
PKG=org.decentraland.godotexplorer
ROOT=${ROOT:-docs/bench/data/2x-vs-1x-3206/android-a54/rss-states}
OUT="$ROOT/$BUILD"
mkdir -p "$OUT"

sample_state() {
  local state=$1 pid i
  for i in $(seq 1 8); do
    {
      adb -s "$SERIAL" shell dumpsys meminfo "$PKG"
      adb -s "$SERIAL" shell logcat -d 2>/dev/null | grep "\[MemMonitor\]" | tail -1
    } > "$OUT/$state-$i.log" 2>&1 || true
    sleep 2
  done
}

# 1. menus: a plain launch stops at the shell (no deep link).
adb -s "$SERIAL" shell am force-stop "$PKG"
adb -s "$SERIAL" shell logcat -c
adb -s "$SERIAL" shell am start -W -n "$PKG/com.godot.game.GodotAppLauncher" >/dev/null
sleep 25
adb -s "$SERIAL" exec-out screencap -p > "$OUT/state-1-menus.png"
sample_state "1-menus"

# 2. in world.
adb -s "$SERIAL" shell am start -a android.intent.action.VIEW -d "'decentraland://open?position=0,0'" "$PKG" >/dev/null
for _ in $(seq 1 60); do
  adb -s "$SERIAL" shell logcat -d 2>/dev/null | grep -q "\[RenderReady\]" && break
  sleep 2
done
sleep 20
adb -s "$SERIAL" exec-out screencap -p > "$OUT/state-2-world.png"
sample_state "2-world"

# 3. leave the world (Android back returns the shell to the previous screen in both builds).
adb -s "$SERIAL" shell input keyevent KEYCODE_BACK
sleep 15
adb -s "$SERIAL" exec-out screencap -p > "$OUT/state-3-after-exit.png"
sample_state "3-after-exit"

# 4. second cycle.
adb -s "$SERIAL" shell logcat -c
adb -s "$SERIAL" shell am start -a android.intent.action.VIEW -d "'decentraland://open?position=0,0'" "$PKG" >/dev/null
for _ in $(seq 1 60); do
  adb -s "$SERIAL" shell logcat -d 2>/dev/null | grep -q "\[RenderReady\]" && break
  sleep 2
done
sleep 20
sample_state "4-world-2nd"
adb -s "$SERIAL" shell input keyevent KEYCODE_BACK
sleep 15
adb -s "$SERIAL" exec-out screencap -p > "$OUT/state-5-after-2nd-exit.png"
sample_state "5-after-2nd-exit"
