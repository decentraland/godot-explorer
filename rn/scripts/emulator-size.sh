#!/usr/bin/env bash
# Pins the connected Android emulator to the Figma frame: 720x1600 at 320 dpi = 360x800dp @2x, so
# a screenshot is the same size as the frame PNG and compares 1:1.  `reset` restores the phone.
#   npm run emulator:figma | npm run emulator:reset
set -eu
ADB_BIN="${ADB:-${ANDROID_HOME:-$HOME/Library/Android/sdk}/platform-tools/adb}"
[ -x "$ADB_BIN" ] || ADB_BIN="$(command -v adb)"
case "${1:-figma}" in
  figma) "$ADB_BIN" shell wm size 720x1600 && "$ADB_BIN" shell wm density 320 ;;
  reset) "$ADB_BIN" shell wm size reset && "$ADB_BIN" shell wm density reset ;;
  *) echo "usage: $0 [figma|reset]" >&2; exit 2 ;;
esac
"$ADB_BIN" shell wm size
