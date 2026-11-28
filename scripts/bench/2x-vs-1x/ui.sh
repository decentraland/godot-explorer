#!/usr/bin/env bash
# Tap and wait by visible text, using the accessibility tree (uiautomator).
#
# Only the React Native shell exposes its views this way: the 1.x client draws
# its UI inside the Godot surface, so there the protocol uses fixed coordinates.
#
#   ui.sh texts                  # list every visible text with its centre point
#   ui.sh tap "Continue"         # tap the node whose text is exactly this
#   ui.sh wait "Genesis Plaza" 30
set -euo pipefail

SERIAL=${ANDROID_SERIAL:-R5CW60624NE}
CMD=${1:?command}

dump() {
  adb -s "$SERIAL" shell uiautomator dump /sdcard/ui.xml >/dev/null 2>&1
  adb -s "$SERIAL" shell cat /sdcard/ui.xml 2>/dev/null
}

center_of() {
  python3 -c '
import re, sys
xml, want = sys.stdin.read(), sys.argv[1]
for m in re.finditer(r"text=\"([^\"]*)\"[^>]*?bounds=\"\[(\d+),(\d+)\]\[(\d+),(\d+)\]\"", xml):
    text = m.group(1)
    if text.strip() != want:
        continue
    x1, y1, x2, y2 = map(int, m.groups()[1:])
    print((x1 + x2) // 2, (y1 + y2) // 2)
    break
' "$1"
}

case "$CMD" in
  texts)
    dump | tr '>' '>\n' | grep -oE 'text="[^"]+"[^/]*bounds="[^"]+"' |
      sed -E 's/text="([^"]*)".*bounds="\[([0-9]+),([0-9]+)\]\[([0-9]+),([0-9]+)\]"/\1 | \2 \3 \4 \5/' |
      head -60
    ;;
  tap)
    coords=$(dump | center_of "${2:?text}")
    [ -n "$coords" ] || { echo "not found: $2" >&2; exit 1; }
    adb -s "$SERIAL" shell input tap $coords
    echo "tapped '$2' at $coords"
    ;;
  wait)
    want=${2:?text}
    deadline=$(( SECONDS + ${3:-60} ))
    while [ "$SECONDS" -lt "$deadline" ]; do
      if dump | grep -qF "text=\"$want\""; then
        echo "found '$want' after $SECONDS s"
        exit 0
      fi
      sleep 1
    done
    echo "timeout waiting for '$want'" >&2
    exit 1
    ;;
  *)
    echo "unknown command $CMD" >&2
    exit 2
    ;;
esac
