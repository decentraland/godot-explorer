#!/usr/bin/env bash
# Pin, verify or release the display refresh rate. The A54 switches between 120 and 60 Hz on its own,
# which changes the vsync period and therefore every cadence number in the report, so a run is only
# admissible with the pin in place and verified after the window.
#
#   display_mode.sh pin       # set min and peak to 120 Hz
#   display_mode.sh verify    # exit 1 unless both settings read 120.0 and the panel agrees
#   display_mode.sh unpin     # delete both settings
set -euo pipefail

SERIAL=${ANDROID_SERIAL:-R5CW60624NE}
TARGET=120.0
sh_() { adb -s "$SERIAL" shell "$@"; }

read_setting() { sh_ settings get system "$1" | tr -d '\r'; }

case "${1:-}" in
  pin)
    sh_ settings put system min_refresh_rate "$TARGET"
    sh_ settings put system peak_refresh_rate "$TARGET"
    echo "[display_mode] pinned to ${TARGET} Hz"
    ;;
  unpin)
    sh_ settings delete system min_refresh_rate >/dev/null
    sh_ settings delete system peak_refresh_rate >/dev/null
    echo "[display_mode] pin released (settings deleted)"
    ;;
  verify)
    min=$(read_setting min_refresh_rate)
    peak=$(read_setting peak_refresh_rate)
    mode=$(sh_ dumpsys display 2>/dev/null | grep -m1 'mActiveModeId' | tr -d '\r')
    echo "[display_mode] min=${min} peak=${peak} ${mode}"
    case "$min:$peak" in
      "${TARGET}:${TARGET}") ;;
      *) echo "[display_mode] FAIL: refresh rate is not pinned to ${TARGET}" >&2; exit 1 ;;
    esac
    ;;
  *)
    echo "usage: $0 pin|verify|unpin" >&2
    exit 64
    ;;
esac
