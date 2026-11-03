#!/usr/bin/env bash
# Alternating measurement runs for #3206 on Android: installs 1.x, runs, installs 2.x, runs.
#
#   run_scenario.sh gp "" 5 60
#   run_scenario.sh gp "&pulse=false&livekit=false" 3 60   (params appended to the deeplink)
#
# Each run writes docs/bench/data/2x-vs-1x-3206/<device>/<scenario>/<label>/ with the Perfetto
# trace, the per-frame CSV, memory/thermal/CPU samples and meta.json (bench_android.py), plus a
# launch.json holding the cold-launch and draw-ready numbers.
set -euo pipefail

SCENARIO=${1:?scenario name}
EXTRA=${2:-}
RUNS=${3:-5}
SECONDS_WINDOW=${4:-60}
SETTLE=${SETTLE:-25}
SERIAL=${ANDROID_SERIAL:-R5CW60624NE}
PKG=org.decentraland.godotexplorer
DEVICE_TAG=${DEVICE_TAG:-android-a54}
ROOT=${ROOT:-docs/bench/data/2x-vs-1x-3206/$DEVICE_TAG/$SCENARIO}
DL="$(cd "$(dirname "$0")/../../../.bench-dl" && pwd)"
HERE="$(cd "$(dirname "$0")" && pwd)"

apk_for() {
  case "$1" in
    1x) echo "$DL/apk-1x-release-latest.apk" ;;
    2x) echo "$DL/apk-2x-e3b7aa102.apk" ;;
  esac
}

installed_version() {
  adb -s "$SERIAL" shell dumpsys package "$PKG" 2>/dev/null | sed -n 's/.*versionName=\([^ ]*\).*/\1/p' | head -1
}

ensure_build() {
  local want=$1 have
  have=$(installed_version)
  case "$want:$have" in
    1x:1.16*) return 0 ;;
    2x:2.*) return 0 ;;
  esac
  echo "[install] $want (have ${have:-none})"
  if ! adb -s "$SERIAL" install -r -d "$(apk_for "$want")" >/dev/null 2>&1; then
    # The Play-protection layer refuses a version-code downgrade, so going 2.x -> 1.x wipes data.
    adb -s "$SERIAL" uninstall "$PKG" >/dev/null 2>&1 || true
    adb -s "$SERIAL" install "$(apk_for "$want")" >/dev/null
  fi
}

prepare_build() {
  # A fresh install has no session, so the deep link waits. Take the entry flow once; every later
  # launch (same data) enters the world straight from the link.
  local build=$1
  adb -s "$SERIAL" shell am force-stop "$PKG"
  adb -s "$SERIAL" shell logcat -c
  adb -s "$SERIAL" shell am start -a android.intent.action.VIEW -d "'decentraland://open?position=0,0'" "$PKG" >/dev/null 2>&1
  for _ in $(seq 1 8); do
    sleep 4
    adb -s "$SERIAL" shell logcat -d 2>/dev/null | grep -q "\[RenderReady\]" && return 0
    case "$build" in
      1x)
        # Godot draws its own UI, so these are portrait coordinates, not accessibility nodes.
        adb -s "$SERIAL" shell input tap 540 1592 >/dev/null 2>&1 || true   # PLAY AS GUEST
        adb -s "$SERIAL" shell input tap 541 1985 >/dev/null 2>&1 || true   # Don't allow (notifications)
        adb -s "$SERIAL" shell input tap 541 1897 >/dev/null 2>&1 || true   # LET'S GO
        ;;
      2x)
        for text in "Play as guest" "Continue" "SKIP" "Not now"; do
          "$HERE/ui.sh" texts 2>/dev/null | grep -q "^$text |" && "$HERE/ui.sh" tap "$text" >/dev/null 2>&1 || true
        done
        ;;
    esac
  done
}

launch_into() {
  local uri="decentraland://open?position=0,0${EXTRA}"
  adb -s "$SERIAL" shell am force-stop "$PKG"
  adb -s "$SERIAL" shell logcat -c
  local t0 t1 total
  t0=$(python3 -c 'import time;print(int(time.time()*1000))')
  total=$(adb -s "$SERIAL" shell am start -W -a android.intent.action.VIEW -d "'$uri'" "$PKG" 2>&1 | sed -n 's/.*TotalTime: \([0-9]*\).*/\1/p' | head -1)
  # A fresh install lands on the entry flow: take it through guest/continue/skip.
  for _ in $(seq 1 6); do
    sleep 3
    for text in "Play as guest" "Continue" "START" "SKIP"; do
      if "$HERE/ui.sh" texts 2>/dev/null | grep -q "^$text |"; then
        "$HERE/ui.sh" tap "$text" >/dev/null 2>&1 || true
      fi
    done
    if adb -s "$SERIAL" shell logcat -d 2>/dev/null | grep -q "\[RenderReady\]"; then break; fi
  done
  for _ in $(seq 1 90); do
    if adb -s "$SERIAL" shell logcat -d 2>/dev/null | grep -q "\[RenderReady\]"; then break; fi
    sleep 2
  done
  t1=$(python3 -c 'import time;print(int(time.time()*1000))')
  echo "$t0" "$t1" "${total:-0}"
}

ORDER=${ORDER:-"1x 1x 2x 2x 1x 1x 2x 2x 1x 2x"}
mkdir -p "$ROOT"
r=0
for build in $ORDER; do
  r=$((r + 1))
  label="${build}-${SCENARIO}-r${r}"
  ensure_build "$build"
  if [ "$build" != "${PREPARED:-}" ]; then
    prepare_build "$build"
    PREPARED=$build
  fi
  read -r t0 t1 total < <(launch_into "$build")
  adb -s "$SERIAL" shell logcat -d -v time 2>/dev/null | grep -E "\[RenderReady\]|\[Startup\]|\[MemMonitor\]" > "/tmp/launch-${label}.log" || true
  python3 - "$build" "$label" "$t0" "$t1" "$total" "$SETTLE" <<'PY' > "$ROOT/${label}-launch.json"
import json, sys
build, label, t0, t1, total, settle = sys.argv[1:7]
print(json.dumps({
    "build": build,
    "label": label,
    "am_total_time_ms": int(total),
    "cold_launch_to_render_ready_ms": int(t1) - int(t0),
    "settle_seconds": int(settle),
}, indent=2))
PY
  cp "/tmp/launch-${label}.log" "$ROOT/" 2>/dev/null || true
  if [ "${NET_OFF_AFTER_READY:-0}" = "1" ]; then
    # Removing the transport is the only way to sample an empty plaza on a build that does not
    # let the benchmark hold comms (#3206 §8): no network, no remote avatars.
    adb -s "$SERIAL" shell cmd wifi set-wifi-enabled disabled >/dev/null 2>&1 || true
  fi
  sleep "$SETTLE"
  python3 "$HERE/bench_android.py" --serial "$SERIAL" window \
    --label "$label" --build "$build" --scenario "$SCENARIO" --seconds "$SECONDS_WINDOW" \
    --out "$ROOT/$label" > "/tmp/window-${label}.json"
  if [ "${NET_OFF_AFTER_READY:-0}" = "1" ]; then
    adb -s "$SERIAL" shell cmd wifi set-wifi-enabled enabled >/dev/null 2>&1 || true
  fi
  echo "[done] $label"
done
