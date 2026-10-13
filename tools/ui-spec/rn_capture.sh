#!/usr/bin/env bash
# Capture registry previews from the connected Android phone into docs/rn-ui-spec/rn-shots/<id>.png.
#   tools/ui-spec/rn_capture.sh --all | --source backpack | --ids a,b   [--timeout 20] [--compare-sheet]
# Ids come from rn/src/gallery/ids.json (regenerate: cd rn && npm run gallery:ids).
set -u
# One phone, several agents: serialise whole capture runs.
exec 9>/tmp/dcl-phone.lock
flock 9
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
ADB_BIN="${ADB:-$HOME/Android/Sdk/platform-tools/adb}"
[ -x "$ADB_BIN" ] || ADB_BIN="$(command -v adb)"
adb() { "$ADB_BIN" "$@" </dev/null 9>&-; }
PKG="${DCL_PACKAGE:-org.decentraland.godotexplorer}"
OUT="$ROOT/docs/rn-ui-spec/rn-shots"
IDS_JSON="$ROOT/rn/src/gallery/ids.json"
TIMEOUT=20
MODE="" ; ARG="" ; COMPARE=""

while [ $# -gt 0 ]; do
  case "$1" in
    --all) MODE=all ;;
    --source) MODE=source; ARG="$2"; shift ;;
    --ids) MODE=ids; ARG="$2"; shift ;;
    --timeout) TIMEOUT="$2"; shift ;;
    --compare-sheet) COMPARE=1 ;;
    *) echo "unknown arg $1" >&2; exit 2 ;;
  esac
  shift
done
[ -n "$MODE" ] || { echo "usage: $0 --all | --source X | --ids a,b [--timeout s]" >&2; exit 2; }
[ -f "$IDS_JSON" ] || { echo "missing $IDS_JSON (cd rn && npm run gallery:ids)" >&2; exit 1; }
adb get-state >/dev/null 2>&1 || { echo "no adb device" >&2; exit 1; }
# adb restarts drop reverse mappings; the app reads Metro from localhost:${METRO_PORT:-8082}.
adb reverse "tcp:${METRO_PORT:-8082}" "tcp:${METRO_PORT:-8082}" >/dev/null 2>&1

# "<id> <orientation>" lines for the selection
LIST="$(python3 - "$IDS_JSON" "$MODE" "$ARG" <<'PY'
import json, sys
rows = json.load(open(sys.argv[1])); mode, arg = sys.argv[2], sys.argv[3]
if mode == "source": rows = [r for r in rows if r["source"] == arg]
if mode == "ids":
    want = arg.split(","); rows = [r for r in rows if r["id"] in want]
for r in rows: print(r["id"], r["orientation"], r["source"])
PY
)"
[ -n "$LIST" ] || { echo "no matching ids" >&2; exit 1; }

mkdir -p "$OUT"
rotation() {  # 0 portrait, 1 landscape
  adb shell settings put system accelerometer_rotation 0
  adb shell settings put system user_rotation "$1"
}
cleanup() { adb shell settings put system user_rotation 0; adb shell settings put system accelerometer_rotation 1; }
trap cleanup EXIT

ok=0; fail=0; failed=""; sources=""
while read -r id orient source; do
  [ -n "$id" ] || continue
  want_rot=0; [ "$orient" = landscape ] && want_rot=1
  found=""
  for attempt in 1 2 3; do
    rotation "$want_rot"
    sleep 1
    adb logcat -c
    adb shell am force-stop "$PKG"
    adb shell am start -a android.intent.action.VIEW -d "dclrn://preview/$id" "$PKG" >/dev/null
    found=""
    for _ in $(seq 1 $((TIMEOUT * 2))); do
      if adb logcat -d -s ReactNativeJS 2>/dev/null | grep -q "DCL_PREVIEW_READY $id"; then found=1; break; fi
      sleep 0.5
    done
    [ -n "$found" ] || continue
    # The orientation warning means the activity did not rotate in time: retry.
    if adb logcat -d -s ReactNativeJS 2>/dev/null | grep -q "DCL_PREVIEW_WARN $id orientation"; then
      found=""; continue
    fi
    break
  done
  if [ -n "$found" ]; then
    sleep 0.5
    mkdir -p "$(dirname "$OUT/$id.png")"
    adb exec-out screencap -p > "$OUT/$id.png" && ok=$((ok + 1)) && echo "ok   $id"
    case " $sources " in *" $source "*) ;; *) sources="$sources $source" ;; esac
  else
    fail=$((fail + 1)); failed="$failed $id"; echo "FAIL $id (not ready or orientation not applied after 3 tries)"
  fi
done <<< "$LIST"

echo "captured $ok, failed $fail -> $OUT"
[ -z "$failed" ] || echo "failed:$failed"
if [ -n "$COMPARE" ]; then
  for src in $sources; do
    (cd "$ROOT" && python3 -I tools/ui-spec/stitch.py --compare rn-shots --source "$src") || echo "compare-sheet failed for $src" >&2
  done
fi
[ "$fail" -eq 0 ]
