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
# Scene-free parcel by default only when asked: Genesis Plaza owns the sky clock, so the pinned
# skybox only applies where there is no scene (POSITION=100,100 for the forced-sky matrix).
POSITION=${POSITION:-0,0}
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
  sh_ am start -a android.intent.action.VIEW -d "'decentraland://open?position=$POSITION'" "$PKG" >/dev/null
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
# The shell only answers taps once it is up, and a fresh install sits on "Get started!" behind a
# system notification dialog, so every step is confirmed through the accessibility tree instead of
# tapping blind: a blind tap here silently measured the shipped defaults once already.
texts_() { "$HERE/ui.sh" texts 2>/dev/null; }

wait_for_text() {  # wait_for_text <regex> <seconds>
  local re=$1 limit=${2:-60} i
  for i in $(seq 1 "$limit"); do
    texts_ | grep -qiE "$re" && return 0
    sleep 1
  done
  return 1
}

settings_2x() {
  mkdir -p "$OUT"
  sh_ am force-stop "$PKG"
  sh_ am start -n "$PKG/com.godot.game.GodotAppLauncher" >/dev/null
  local i
  local dump
  # A fresh install greets with the system notification dialog, which is not in the accessibility
  # tree and makes uiautomator dump hang: answer it by coordinate before asking the tree anything.
  sh_ input tap 538 2039 >/dev/null 2>&1 || true
  sleep 2
  for i in $(seq 1 40); do
    # One accessibility dump per iteration: each one costs 1-3 s here, so asking five times per round
    # turned the welcome flow into a 20-minute heat source.
    dump=$(texts_)
    for step in "Accept and continue" "Play as guest" "Continue" "SKIP" "Not now"; do
      grep -q "^$step |" <<<"$dump" && "$HERE/ui.sh" tap "$step" >/dev/null 2>&1 || true
    done
    # After a reinstall the launcher resumes whatever the previous session left on screen, which can
    # be the world: BACK opens the RN leave dialog, whose LEAVE node returns to the shell.
    if grep -qE "^LEAVE \|" <<<"$dump"; then
      "$HERE/ui.sh" tap "LEAVE" >/dev/null 2>&1 || true
    elif ! grep -qE "^(Discover|Accept and continue|Play as guest|Continue) \|" <<<"$dump"; then
      sh_ input keyevent KEYCODE_BACK >/dev/null 2>&1 || true
    fi
    grep -qE "^Discover \|" <<<"$dump" && break
    sleep 2
  done
  texts_ | grep -qE "^Discover \|" || { echo "[FAIL] 2x shell never came up"; return 1; }
  adb -s "$SERIAL" exec-out screencap -p > "$OUT/settings-1-shell.png"
  sh_ input tap 645 2094 >/dev/null; sleep 1      # Settings tab
  wait_for_text "^Graphics \|" 20 || { echo "[FAIL] no Settings list"; return 1; }
  adb -s "$SERIAL" exec-out screencap -p > "$OUT/settings-2-list.png"
  sh_ input tap 400 292 >/dev/null; sleep 1       # Graphics
  wait_for_text "Dynamic Graphics" 20 && texts_ | grep -qE "^Graphic Profiles \||^High \||High" \
    || { echo "[FAIL] no Graphics page"; return 1; }
  sh_ input tap 901 465 >/dev/null; sleep 1       # Dynamic Graphics -> OFF
  sh_ input tap 538 860 >/dev/null; sleep 1       # Graphic Profiles dropdown
  sh_ input tap 260 1193 >/dev/null; sleep 1      # High
  adb -s "$SERIAL" exec-out screencap -p > "$OUT/settings.png"
  texts_ | grep -qi "High" || { echo "[FAIL] profile is not High"; return 1; }
  echo "[settings] 2x: dynamic OFF + profile High"
}

# 1.x: the settings panel only exists inside the world (avatar -> gear).
settings_1x() {
  mkdir -p "$OUT"
  # One tap on the avatar opens the panel (the "Menu" bubble is only a label); tapping it twice
  # closes what the first tap opened, which silently measured the defaults once.
  # The first tap after entering the world is eaten by the tutorial overlay, so a neutral tap in the
  # sky burns it and the avatar tap below actually opens the panel.
  sh_ input tap 1700 300 >/dev/null; sleep 1.5
  sh_ input tap 224 105 >/dev/null; sleep 1.5
  adb -s "$SERIAL" exec-out screencap -p > "$OUT/settings-1-menu.png"
  sh_ input tap 226 723 >/dev/null; sleep 1.5     # gear -> Settings
  adb -s "$SERIAL" exec-out screencap -p > "$OUT/settings-2-graphics.png"
  sh_ input tap 2077 310 >/dev/null; sleep 1.5    # Dynamic Graphics -> OFF
  sh_ input tap 1568 624 >/dev/null; sleep 1.5    # Graphic Profiles dropdown
  sh_ input swipe 1340 450 1340 200 400 >/dev/null; sleep 1.5   # the list opens on "Very Low"; High is below the fold
  sh_ input tap 1026 517 >/dev/null; sleep 1.5    # High
  sh_ input tap 2077 933 >/dev/null; sleep 1.5    # Dynamic Skybox -> OFF (pins the sun where 2.x pins it)
  adb -s "$SERIAL" exec-out screencap -p > "$OUT/settings.png"
  # 1.x restores the last panel on relaunch and an open panel halves the frame rate: leave via the
  # back chevron, then tap the world to close the menu (BACK here reopens the panel instead).
  sh_ input tap 199 88 >/dev/null; sleep 1.5
  sh_ input tap 1900 500 >/dev/null; sleep 1.5
  adb -s "$SERIAL" exec-out screencap -p > "$OUT/settings-3-world.png"
  echo "[settings] 1x: dynamic OFF + profile High + skybox OFF (verify settings.png and settings-3-world.png)"
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
probe_profile() {  # relaunch and report the profile the engine actually loaded
  sh_ am force-stop "$PKG"
  sh_ logcat -c
  sh_ am start -a android.intent.action.VIEW -d "'decentraland://open?position=$POSITION'" "$PKG" >/dev/null 2>&1 || true
  local i
  for i in $(seq 1 45); do
    sh_ logcat -d 2>/dev/null | grep -q '\[RenderReady\]' && break
    sleep 2
  done
  sh_ logcat -d 2>/dev/null | grep -o '"profile":[0-9]*' | tail -1
}

apply_settings() {
  case "$BUILD" in
    2x) settings_2x ;;
    1x)
      launch_world          # the gear only exists in-world
      settings_1x
      ;;
  esac
}

# Taps are not evidence: the panel can show High while the engine loads something else, and on 1.x the
# panel is drawn by the engine so nothing reads it. Every attempt is therefore checked against the
# profile the engine reports, and the taps are repeated until it matches.
if [ "${SKIP_SETTINGS:-0}" = "1" ]; then
  # The taps were applied by hand and verified through the engine log; tapping again would flip the
  # toggle back on, and on 2.x uiautomator can hang on the Discover carousel.
  echo "[settings] SKIP_SETTINGS=1: applied by hand"
elif [ "$FRESH" = "1" ] || [ "${FORCE_SETTINGS:-0}" = "1" ]; then
  for attempt in 1 2 3; do
    apply_settings || true
    probed=$(probe_profile)
    echo "[settings] attempt $attempt: engine ${probed:-no profile}"
    [ "$probed" = '"profile":3' ] && break
  done
  [ "$probed" = '"profile":3' ] || { echo "[FAIL] settings never reached profile 3"; mkdir -p "$OUT"; echo "Settings could not be applied in 3 attempts." > "$OUT/INVALID.md"; exit 1; }
else
  echo "[settings] $BUILD already installed and set in this sequence, not re-tapping"
fi
cool_down

# The buffer carries every earlier launch of this session, so it is cleared here and the evidence is
# then filtered by the pid of the process this launch created: reading it unfiltered once made a
# run's "verified settings" come from a launch that had already finished.
# A locked screen leaves Godot's window at viewVisibility=8 and no RenderReady is ever printed, which
# reads downstream as "did not start on profile 3". Five runs were lost to that before the lock screen was
# found, so the keyguard is dismissed here and its state recorded with the run.
#
# The app is also stopped first: delivered to a live instance, the intent re-enters the world without a
# fresh RenderReady, so the guard timed out on a run whose engine was already up.
sh_ input keyevent KEYCODE_WAKEUP >/dev/null 2>&1 || true
sh_ am force-stop "$PKG" >/dev/null 2>&1 || true
sleep 1
sh_ wm dismiss-keyguard >/dev/null 2>&1 || true
unlocked=$(sh_ dumpsys window 2>/dev/null | grep -cE 'mDreamingLockscreen=true|isKeyguardShowing=true' || true)
sh_ logcat -c
t0=$(python3 -c 'import time;print(int(time.time()*1000))')
total=$(sh_ am start -W -a android.intent.action.VIEW -d "'decentraland://open?position=$POSITION'" "$PKG" 2>&1 |
  sed -n 's/.*TotalTime: \([0-9]*\).*/\1/p' | head -1)
pid=$(sh_ pidof "$PKG" 2>/dev/null | tr ' ' '\n' | head -1)
for _ in $(seq 1 90); do
  sh_ logcat -d 2>/dev/null | grep -q "\[RenderReady\]" && break
  sleep 2
done
t1=$(python3 -c 'import time;print(int(time.time()*1000))')
sh_ logcat -d 2>/dev/null | grep -E "\[RenderReady\]|\[DynamicGraphics\] initialized" |
  grep " ${pid:-none} " > "$OUT/launch.log" || true
# What the engine can actually prove about the settings: the profile it loaded (the RenderReady json
# carries it) and that the dynamic-graphics controller never moved it during the window. The
# `initialized: enabled=…` line is not usable as the guard — it is emitted by the 1.x engine path and
# a run of 2.x never prints it, so grepping for it in an unfiltered buffer once "verified" a run with
# a line from a different launch.
if ! grep -q '"profile":3' "$OUT/launch.log"; then
  echo -n "[FAIL] $BUILD did not start on profile 3 (pid ${pid:-?}, log bytes $(wc -c < "$OUT/launch.log"), keyguard-blocked-lines ${unlocked:-0}): "
  if [ ! -s "$OUT/launch.log" ]; then
    echo "no engine output at all — the screen was locked or the process died at launch"
  else
    echo; cat "$OUT/launch.log"
  fi
  exit 1
fi
profile_changes() { sh_ logcat -d 2>/dev/null | grep " ${pid:-none} " | grep -cE "Profile (downgrade|upgrade)" || true; }
python3 - "$BUILD" "$LABEL" "$t0" "$t1" "${total:-0}" <<'PY' > "$OUT/launch.json"
import json, sys
build, label, t0, t1, total = sys.argv[1:6]
print(json.dumps({"build": build, "label": label, "am_total_time_ms": int(total),
                  "cold_launch_to_render_ready_ms": int(t1) - int(t0)}, indent=2))
PY
# The first-run tutorial is a modal over the world: it dims everything and it darkened a run to luma
# 48, so it is dismissed and the dismissal is verified before the window instead of assumed.
dismiss_tutorial() {
  local i
  for i in 1 2 3; do
    if [ "$BUILD" = "2x" ]; then
      # The welcome modal is drawn by the engine, so it is not in the accessibility tree: it is
      # dismissed by the coordinate of its SKIP button, once, before the settle.
      sh_ input tap 331 661 >/dev/null 2>&1 || true
    else
      # 1.x draws its own overlay in the engine, so it is burned with a tap in the sky.
      [ "$i" = "1" ] && sh_ input tap 1700 300 >/dev/null 2>&1 || true
      return 0
    fi
    sleep 1.5
  done
  return 0
}
dismiss_tutorial

sleep "$SETTLE"
WINDOW_START_UTC=$(python3 -c 'import time;print("%.3f" % time.time())')
python3 "$HERE/bench_android.py" --serial "$SERIAL" window \
  --label "$LABEL" --build "$BUILD" --scenario "$SCENARIO" --seconds "$WINDOW" \
  --out "$OUT" > "$OUT/window.json"
# Fingerprint of the state the run actually measured: the sky time is the biggest confounder in this
# scene (the same build gives ~20 fps at night and ~31 fps at noon), so it is recorded next to the
# temperature instead of being assumed.
python3 - "$OUT" "$(ap_temp)" "$WINDOW_START_UTC" <<'PY2' > "$OUT/state.json"
import json, sys
from PIL import Image, ImageStat
out, temp, start = sys.argv[1], sys.argv[2], sys.argv[3]
luma = ImageStat.Stat(Image.open(f"{out}/screen.png").convert("L")).mean[0]
thermal = open(f"{out}/thermal.csv").read().splitlines()[1].split(",")
# The in-game hour at the moment the window opened, not the hour it was scheduled for: the thermal gate
# can delay a window by a couple of minutes, which is 24 in-game minutes, and that is the difference the
# scene cost follows.
epoch = float(start)
print(json.dumps({"luma_mean": round(luma, 1), "thermal_start_c": float(thermal[1]),
                  "thermal_skin_start_c": float(thermal[2]), "thermal_end_ap_c": float(temp or 0),
                  "window_start_utc": start,
                  "window_start_phase_s": round((epoch * 12) % 86400, 1)},
                 indent=2))
PY2
luma=$(sed -n 's/.*"luma_mean": \([0-9.]*\).*/\1/p' "$OUT/state.json")
# A modal over the world or a wrong pose reads as a different scene: below this the window is not the
# world (the tutorial run measured luma 48 against 75-140 for the real thing). LUMA_MIN exists because the
# world-clock arms measure at night on purpose, where the same scene legitimately reads ~47.
LUMA_MIN=${LUMA_MIN:-55}
if python3 -c "import sys; sys.exit(0 if float('$luma') < float('$LUMA_MIN') - 0.001 else 1)"; then
  echo "[FAIL] window is not the plain world (luma $luma): an overlay or a wrong pose"
  printf 'Window luma %.1f is far below the floor %s: an overlay (the first-run tutorial dims the world)\nor a different pose.\n' "$luma" "$LUMA_MIN" > "$OUT/INVALID.md"
  exit 1
fi
if python3 -c "import sys; sys.exit(0 if float('$luma') >= 100 else 1)"; then :; else
  echo "[warn] window looks like dusk/night (luma $luma): the sky time is not pinned the same way on both builds"
fi

changes=$(profile_changes)
if [ "${changes:-0}" -gt 0 ]; then
  echo "[INVALID] the dynamic-graphics controller changed the profile $changes time(s) inside the window" |
    tee "$OUT/INVALID.md"
fi
echo "[done] $LABEL  AP $(ap_temp) °C"
