#!/usr/bin/env bash
# Drives the app on an Android phone without the debug hub (CI builds cannot attach it on 2.x).
#   android-ui.sh dump              visible texts / accessibility labels with their bounds
#   android-ui.sh tap "<regex>"     tap the centre of the first element whose text or label matches
#   android-ui.sh shot <file.png>   screenshot in physical pixels
#   android-ui.sh state             pid, rotation, keyboard, top activity
#   android-ui.sh wait-landscape    wait (max 120 s) until the world turned the screen landscape
# Uses `maestro hierarchy` (~15 s): `uiautomator dump` waits for an idle UI and hangs on screens
# with animations (Discover's carousel). Set ANDROID_SERIAL when several devices are connected.
set -uo pipefail
DEV="${ANDROID_SERIAL:-$(adb devices | awk 'NR>1 && $2=="device"{print $1; exit}')}"
H="${TMPDIR:-/tmp}/android-ui-hierarchy.json"
PKG=org.decentraland.godotexplorer

hierarchy() { maestro --device "$DEV" hierarchy 2>/dev/null > "$H"; }

case "${1:-}" in
  dump)
    hierarchy
    jq -r '.. | objects | select(.attributes?) | .attributes
      | select((.text // "") != "" or (.accessibilityText // "") != "")
      | "\(.text // "")|\(.accessibilityText // "")|\(.["resource-id"] // "") @ \(.bounds)"' "$H" ;;
  tap)
    hierarchy
    b=$(jq -r --arg re "$2" '[.. | objects | select(.attributes?) | .attributes
      | select(((.text // "") | test($re;"i")) or ((.accessibilityText // "") | test($re;"i")))
      | .bounds][0] // empty' "$H")
    [ -z "$b" ] && { echo "no element matches '$2'" >&2; exit 1; }
    read -r x1 y1 x2 y2 <<< "$(echo "$b" | sed 's/\]\[/,/; s/[][]//g' | tr ',' ' ')"
    x=$(( (x1 + x2) / 2 )); y=$(( (y1 + y2) / 2 ))
    adb -s "$DEV" shell input tap "$x" "$y"
    echo "tapped '$2' at $x,$y" ;;
  shot)
    adb -s "$DEV" exec-out screencap -p > "$2" && echo "$2" ;;
  state)
    echo "pid=$(adb -s "$DEV" shell pidof $PKG)"
    adb -s "$DEV" shell dumpsys window displays | grep -o "mRotation=ROTATION_[0-9]*" | head -1
    adb -s "$DEV" shell dumpsys input_method | grep -o "mInputShown=[a-z]*" | head -1
    adb -s "$DEV" shell dumpsys activity activities | grep -m1 topResumedActivity
    adb -s "$DEV" shell dumpsys window | grep -o "isKeyguardShowing=[a-z]*" | head -1 ;;
  wait-landscape)
    for _ in $(seq 1 24); do
      r=$(adb -s "$DEV" shell dumpsys window displays | grep -o "mRotation=ROTATION_[0-9]*" | head -1)
      case "$r" in *90|*270) echo "$r"; exit 0 ;; esac
      sleep 5
    done
    echo "still portrait after 120 s" >&2; exit 1 ;;
  *)
    sed -n '2,9p' "$0"; exit 2 ;;
esac
