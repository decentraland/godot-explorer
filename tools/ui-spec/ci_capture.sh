#!/usr/bin/env bash
# Screenshot gallery stories on a booted Android emulator/device: Metro with Storybook, the debug APK, Maestro.
#   tools/ui-spec/ci_capture.sh <sources|--set|--ids <out-dir> a,b> <out-dir>
# The gallery is __DEV__-only, so the debug APK (-PdclNoGodot) is served by Metro, as on a developer's phone.
# DCL_APK overrides the APK path; METRO_PORT defaults to 8081 (the app's built-in default).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SELECT="$1"
OUT="$2"
APK="${DCL_APK:-$ROOT/rn/android/app/build/outputs/apk/debug/app-debug.apk}"
export METRO_PORT="${METRO_PORT:-8081}"
export EXPO_PUBLIC_STORYBOOK_ENABLED=true
export MAESTRO_CLI_NO_ANALYTICS=1
export MAESTRO_CLI_ANALYSIS_NOTIFICATION_DISABLED=true
WORK="$(mktemp -d)"

(cd "$ROOT/rn" && exec npx react-native start --port "$METRO_PORT" >"$ROOT/metro.log" 2>&1) &
METRO_PID=$!
trap 'kill "$METRO_PID" 2>/dev/null || true; rm -rf "$WORK"' EXIT

for _ in $(seq 1 120); do
  curl -sf "http://localhost:$METRO_PORT/status" 2>/dev/null | grep -q running && break
  sleep 1
done
# The first bundle request transforms the whole app; do it before the first timed capture.
curl -sf --max-time 900 -o /dev/null \
  "http://localhost:$METRO_PORT/index.bundle?platform=android&dev=true&minify=false" \
  || { echo "Metro could not build the bundle:" >&2; tail -n 40 "$ROOT/metro.log" >&2; exit 1; }

adb install -r -t "$APK"
adb reverse "tcp:$METRO_PORT" "tcp:$METRO_PORT"
. "$ROOT/tools/ui-spec/device_normalize.sh"
normalize_device

if [ "$SELECT" = "--ids" ]; then
  python3 "$ROOT/tools/ui-spec/maestro_flows.py" "$WORK/flows" --ids "$3"
elif [ "$SELECT" = "--set" ]; then
  python3 "$ROOT/tools/ui-spec/maestro_flows.py" "$WORK/flows" --set
else
  python3 "$ROOT/tools/ui-spec/maestro_flows.py" "$WORK/flows" --source "$SELECT"
fi

status=0
maestro ${ANDROID_SERIAL:+--udid "$ANDROID_SERIAL"} test --test-output-dir "$WORK/out" "$WORK/flows" || status=$?

# Maestro writes <run>/<flow>/takeScreenshot/<id>.png; the baselines are laid out by id.
mkdir -p "$OUT"
find "$WORK/out" -path '*/takeScreenshot/*' -name '*.png' | while read -r shot; do
  rel="${shot#*/takeScreenshot/}"
  mkdir -p "$OUT/$(dirname "$rel")"
  cp "$shot" "$OUT/$rel"
done
python3 "$ROOT/tools/ui-spec/visual_report.py" optimize "$OUT"
exit "$status"
