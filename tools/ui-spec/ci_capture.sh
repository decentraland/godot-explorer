#!/usr/bin/env bash
# Screenshots gallery screens from the react-native-web build of the app (no emulator, no Godot).
#   tools/ui-spec/ci_capture.sh <out-dir> --set | --ids a,b | --source s1,s2
# The gallery is __DEV__-only, so the web bundle is exported in dev mode.
# CHANNEL=chrome uses the installed Chrome; otherwise `npx playwright-core install chromium` first.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT="$1"
shift
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

(cd "$ROOT/rn" && npx expo export --platform web --dev --output-dir "$WORK/web")
status=0
node "$ROOT/tools/ui-spec/web_capture.mjs" --dir "$WORK/web" --out "$OUT" "$@" || status=$?
python3 "$ROOT/tools/ui-spec/visual_report.py" optimize "$OUT"
exit "$status"
