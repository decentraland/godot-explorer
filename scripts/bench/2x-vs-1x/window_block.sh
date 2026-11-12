#!/usr/bin/env bash
# Measures N windows against the instance that is already in the world.
#
# The patched 2.x build replays its welcome flow on every cold start and its taps cannot be driven
# reliably, so the app is brought into the world once by hand and every window then runs against that
# same process. The thermal gate, the settle and the window length are the same as run_paired.sh, so the
# records are comparable with the arm that does relaunch.
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
SERIAL=${ANDROID_SERIAL:-R5CW60624NE}
export ANDROID_SERIAL="$SERIAL"
BUILD=${BUILD:?BUILD}
PREFIX=${PREFIX:?PREFIX}
SC=${SC:?SC}
N=${N:-6}
WINDOW=${WINDOW:-30}
SETTLE=${SETTLE:-20}
OUT="$ROOT/docs/bench/data/2x-vs-1x-3206/android-a54/$SC"
PY="$ROOT/scripts/bench/2x-vs-1x"

mkdir -p "$OUT"
[ -f "$OUT/phases.csv" ] || echo "label,build,utc_iso,battery_pct,ap_c,luma" > "$OUT/phases.csv"

for i in $(seq 0 $((N - 1))); do
  label=$(printf '%s-r%d' "$PREFIX" "$i")
  bat=$(adb -s "$SERIAL" shell dumpsys battery | sed -n 's/.*level: \([0-9]*\).*/\1/p' | head -1)
  ap=$(adb -s "$SERIAL" shell dumpsys thermalservice | grep -o "mValue=[0-9.]*, mType=0, mName=AP" | head -1 | sed 's/mValue=\([0-9.]*\).*/\1/')
  echo "=== $label bat=$bat ap=$ap"
  echo "[staying] measuring the running instance"
  LEDGER=/tmp/gates-$(date +%Y%m%d).jsonl FLOOR_S=${FLOOR_S:-150} "$PY/thermal_gate.sh" || true
  sleep 1.5
  python3 "$PY/bench_android.py" window --label "$label" --build "$BUILD" --scenario "$SC" \
    --seconds "$WINDOW" --out "$OUT/$label"
  adb -s "$SERIAL" exec-out screencap -p > "$OUT/$label/screen.png"
  luma=$(python3 "$PY/luma.py" "$OUT/$label/screen.png" --json | python3 -c 'import json,sys;print(json.load(sys.stdin)["luma_crop"])')
  python3 "$PY/fingerprint.py" --dir "$OUT/$label" --label "$label" --build "$BUILD" \
    --expect-profile "${EXPECT_PROFILE:-}" --max-thermal-status "${MAX_THERMAL_STATUS:-1}" \
    $( [ -n "${LUMA_BAND:-}" ] && echo --luma $LUMA_BAND ) || echo "[warn] $label flagged"
  echo "$label,$BUILD,$(date -u +%Y-%m-%dT%H:%M:%SZ),$bat,$ap,$luma" >> "$OUT/phases.csv"
  echo "[done] $label luma=$luma"
done
echo "=== arm done"
