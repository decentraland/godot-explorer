#!/usr/bin/env bash
# Capture gallery previews on a booted Android emulator/device: Metro + the debug APK + rn_capture.sh.
#   tools/ui-spec/ci_capture.sh <sources|--ids a,b> <out-dir>
# The gallery is __DEV__-only, so the debug APK (-PdclNoGodot) is served by Metro, as on a developer's phone.
# DCL_APK overrides the APK path; METRO_PORT defaults to 8081 (the app's built-in default).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SELECT="$1"
OUT="$2"
APK="${DCL_APK:-$ROOT/rn/android/app/build/outputs/apk/debug/app-debug.apk}"
export METRO_PORT="${METRO_PORT:-8081}"

(cd "$ROOT/rn" && exec npx react-native start --port "$METRO_PORT" >"$ROOT/metro.log" 2>&1) &
METRO_PID=$!
trap 'kill "$METRO_PID" 2>/dev/null || true' EXIT

for _ in $(seq 1 120); do
  curl -sf "http://localhost:$METRO_PORT/status" 2>/dev/null | grep -q running && break
  sleep 1
done
# The first bundle request transforms the whole app; do it before the first timed capture.
curl -sf --max-time 900 -o /dev/null \
  "http://localhost:$METRO_PORT/index.bundle?platform=android&dev=true&minify=false" \
  || { echo "Metro could not build the bundle:" >&2; tail -n 40 "$ROOT/metro.log" >&2; exit 1; }

adb install -r -t "$APK"

if [ "$SELECT" = "--ids" ]; then
  SELECTION=(--ids "$3")
else
  SELECTION=(--source "$SELECT")
fi

status=0
"$ROOT/tools/ui-spec/rn_capture.sh" "${SELECTION[@]}" --out "$OUT" --normalize --timeout 30 || status=$?
python3 "$ROOT/tools/ui-spec/visual_diff.py" optimize "$OUT"
exit "$status"
