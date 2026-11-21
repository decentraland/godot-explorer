#!/usr/bin/env bash
# N windows of iOS frame data, recorded with Instruments instead of sampled from the HUD.
#
# Each window launches (or reuses) the app in the world, waits for it to settle, records a Game
# Performance trace and reduces `displayed-surfaces-interval` to the same frames.csv the Android arm
# writes, so both platforms land in one table. Thermal state is not gated here: iOS gives no
# equivalent of Android's Thermal Status, and pretending otherwise would be worse than saying it.
#
#   N=12 WINDOW=30 SETTLE=20 sh scripts/bench/2x-vs-1x/run_ios_windows.sh 2x-ios
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
DEVICE=${IOS_DEVICE:-00008140-000529060C82801C}
BUNDLE=${IOS_BUNDLE:-org.decentraland.godotexplorer}
PROCESS=${IOS_PROCESS:-DclRn}
OUT=${OUT:-$ROOT/docs/bench/data/2x-vs-1x-3206/ios-16e/sky-noon-pinned}
N=${N:-12}
WINDOW=${WINDOW:-30}
SETTLE=${SETTLE:-20}
PREFIX=${PREFIX:-2x-ios-win}
POSITION=${POSITION:-0,0}

mkdir -p "$OUT"

for i in $(seq 0 $((N - 1))); do
  label=$(printf '%s-r%d' "$PREFIX" "$i")
  echo "=== $label"
  xcrun devicectl device process launch --device "$DEVICE" --terminate-existing \
    --payload-url "decentraland://open?position=$POSITION" "$BUNDLE" >/dev/null 2>&1 || true
  sleep "$SETTLE"
  trace="/tmp/$label.trace"
  rm -rf "$trace"
  xcrun xctrace record --template 'Game Performance' --device "$DEVICE" \
    --time-limit "${WINDOW}s" --output "$trace" --attach "$PROCESS" >/dev/null 2>&1
  python3 "$ROOT/scripts/bench/2x-vs-1x/ios_frames.py" --trace "$trace" \
    --out "$OUT/$label" --label "$label" || echo "[warn] $label skipped"
  rm -rf "$trace"
done
echo "=== ios arm done"
