#!/usr/bin/env bash
# Thermal-matched A/B runs for #3206: dynamic graphics OFF + graphic profile HIGH in both builds.
#
#   run_paired.sh 1x 1x-hi-p1        # one 30 s run of the 1.x build, settings applied first
#   WINDOW=30 SETTLE=25 TARGET_TEMP=40 run_paired.sh 2x 2x-hi-p2
#
# Why this exists: with dynamic graphics OFF and the HIGH profile the A54 heats up and throttles, so
# a block of runs drifts (16.9 -> 13.6 fps inside the same build). Each run here therefore starts
# from the same AP temperature (TARGET_TEMP, polled while the app is stopped), so the number belongs
# to the build and not to how warm the phone already was.
#
# Settings are not scriptable from a file (they live in the app's internal storage) and each install
# wipes them, so they are applied with taps, once per installation, before the cooldown.
set -euo pipefail

BUILD=${1:?build (1x|2x)}
LABEL=${2:?run label}
WINDOW=${WINDOW:-30}
SETTLE=${SETTLE:-25}
TARGET_TEMP=${TARGET_TEMP:-40}
SCENARIO=${SCENARIO:-gp-hi-pair}
SERIAL=${ANDROID_SERIAL:-R5CW60624NE}
PKG=org.decentraland.godotexplorer
DEVICE_TAG=${DEVICE_TAG:-android-a54}
HERE="$(cd "$(dirname "$0")" && pwd)"
DL="$(cd "$HERE/../../../.bench-dl" && pwd)"
OUT="docs/bench/data/2x-vs-1x-3206/$DEVICE_TAG/$SCENARIO/$LABEL"

apk_for() {
  case "$1" in
    1x) echo "$DL/apk-1x-release-latest.apk" ;;
    2x) echo "$DL/apk-2x-e3b7aa102.apk" ;;
  esac
}

sh_() { adb -s "$SERIAL" shell "$@"; }

installed_version() {
  sh_ dumpsys package "$PKG" 2>/dev/null | sed -n 's/.*versionName=\([^ ]*\).*/\1/p' | head -1
}

FRESH=0
ensure_build() {
  local have
  have=$(installed_version)
  case "$BUILD:$have" in
    1x:1.16*) return 0 ;;
    2x:2.*) return 0 ;;
  esac
  FRESH=1
  echo "[install] $BUILD (have ${have:-none})"
  # A version-code downgrade is refused, so 2.x -> 1.x needs an uninstall; that also wipes the
  # settings, which is why they are re-applied below.
  if ! adb -s "$SERIAL" install -r -d "$(apk_for "$BUILD")" >/dev/null 2>&1; then
    adb -s "$SERIAL" uninstall "$PKG" >/dev/null 2>&1 || true
    adb -s "$SERIAL" install "$(apk_for "$BUILD")" >/dev/null
  fi
}

launch_world() {
  sh_ am force-stop "$PKG"
  sh_ logcat -c
  sh_ am start -a android.intent.action.VIEW -d "'decentraland://open?position=0,0'" "$PKG" >/dev/null
  for _ in $(seq 1 40); do
    # A fresh install stops at the entry flow: take it the way the runner does.
    if [ "$BUILD" = "2x" ]; then
      for text in "Play as guest" "Continue" "SKIP" "Not now" "Allow"; do
        "$HERE/ui.sh" texts 2>/dev/null | grep -q "^$text |" && "$HERE/ui.sh" tap "$text" >/dev/null 2>&1 || true
      done
    else
      sh_ input tap 540 1592 >/dev/null 2>&1 || true   # PLAY AS GUEST
      sh_ input tap 541 1985 >/dev/null 2>&1 || true   # Don't allow (notifications)
      sh_ input tap 541 1897 >/dev/null 2>&1 || true   # LET'S GO
    fi
    sh_ logcat -d 2>/dev/null | grep -q "\[RenderReady\]" && return 0
    sleep 2
  done
  echo "[warn] no [RenderReady] within 80 s"
}

# 2.x: settings live in the React Native shell, so this runs without the world.
settings_2x() {
  sh_ am force-stop "$PKG"
  sh_ am start -n "$PKG/com.godot.game.GodotAppLauncher" >/dev/null
  for _ in $(seq 1 20); do
    "$HERE/ui.sh" texts 2>/dev/null | grep -qiE "Play as guest" && "$HERE/ui.sh" tap "Play as guest" >/dev/null 2>&1 || true
    "$HERE/ui.sh" texts 2>/dev/null | grep -qi "Discover" && break
    sleep 1
  done
  sh_ input tap 645 2094 >/dev/null          # Settings tab
  sleep 1
  sh_ input tap 400 292 >/dev/null           # Graphics
  sleep 1
  sh_ input tap 901 465 >/dev/null           # Dynamic Graphics -> OFF
  sleep 1
  sh_ input tap 538 860 >/dev/null           # Graphic Profiles dropdown
  sleep 1
  sh_ input tap 260 1193 >/dev/null          # High
  sleep 1
  mkdir -p "$OUT"
  adb -s "$SERIAL" exec-out screencap -p > "$OUT/settings.png"
}

# 1.x: the settings panel only exists inside the world (avatar -> gear).
settings_1x() {
  sh_ input tap 224 105 >/dev/null; sleep 1      # avatar -> menu (first tap also kills the tutorial)
  sh_ input tap 224 105 >/dev/null; sleep 1
  sh_ input tap 226 723 >/dev/null; sleep 1      # gear -> Settings
  sh_ input tap 2075 328 >/dev/null; sleep 1     # Dynamic Graphics -> OFF
  sh_ input tap 1568 645 >/dev/null; sleep 1     # Graphic Profiles dropdown
  sh_ input swipe 1568 500 1568 260 400 >/dev/null; sleep 1
  sh_ input tap 1035 511 >/dev/null; sleep 1     # High
  mkdir -p "$OUT"
  adb -s "$SERIAL" exec-out screencap -p > "$OUT/settings.png"
}

ap_temp() {
  sh_ dumpsys thermalservice 2>/dev/null | grep 'mName=AP' | head -1 | sed -n 's/.*mValue=\([0-9.]*\).*/\1/p'
}

cool_down() {
  sh_ am force-stop "$PKG"
  local t
  t=$(ap_temp)
  echo "[cool] AP ${t:-?} °C, waiting for <= $TARGET_TEMP °C"
  until [ "$(ap_temp | cut -d. -f1)" -le "$TARGET_TEMP" ] 2>/dev/null; do sleep 20; done
  echo "[cool] AP $(ap_temp) °C"
}

rm -rf "$OUT"
mkdir -p "$OUT"
ensure_build
# Only a fresh install needs the taps: each install wipes the settings, and running the toggle again
# on a build that is already OFF/HIGH would flip Dynamic Graphics back on.
if [ "$FRESH" = "1" ]; then
  case "$BUILD" in
    2x) settings_2x ;;
    1x)
      launch_world          # the gear only exists in-world
      settings_1x
      ;;
  esac
else
  echo "[settings] $BUILD already installed and set in this sequence, not re-tapping"
fi
cool_down

t0=$(python3 -c 'import time;print(int(time.time()*1000))')
total=$(sh_ am start -W -a android.intent.action.VIEW -d "'decentraland://open?position=0,0'" "$PKG" 2>&1 |
  sed -n 's/.*TotalTime: \([0-9]*\).*/\1/p' | head -1)
for _ in $(seq 1 90); do
  sh_ logcat -d 2>/dev/null | grep -q "\[RenderReady\]" && break
  sleep 2
done
t1=$(python3 -c 'import time;print(int(time.time()*1000))')
sh_ logcat -d 2>/dev/null | grep -E "\[RenderReady\]|\[DynamicGraphics\] initialized" > "$OUT/launch.log" || true
python3 - "$BUILD" "$LABEL" "$t0" "$t1" "${total:-0}" <<'PY' > "$OUT/launch.json"
import json, sys
build, label, t0, t1, total = sys.argv[1:6]
print(json.dumps({"build": build, "label": label, "am_total_time_ms": int(total),
                  "cold_launch_to_render_ready_ms": int(t1) - int(t0)}, indent=2))
PY
sleep "$SETTLE"
python3 "$HERE/bench_android.py" --serial "$SERIAL" window \
  --label "$LABEL" --build "$BUILD" --scenario "$SCENARIO" --seconds "$WINDOW" \
  --out "$OUT" > "$OUT/window.json"
echo "[done] $LABEL  AP $(ap_temp) °C"
