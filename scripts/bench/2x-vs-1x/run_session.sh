#!/usr/bin/env bash
# Windows over one already-running session: the app is launched and its settings applied once,
# outside this script, and every window below belongs to the same process. Needed because the 1.x
# skybox selection does not survive a relaunch (it reverts to the stored value), so the pinned sky
# can only be kept by not relaunching.
set -u
cd "$(dirname "$0")/../../.." || exit 1

BUILD=${1:?build 1x|2x}
SCENARIO=${SCENARIO:?scenario name}
PREFIX=${PREFIX:?label prefix}
N=${N:-3}
GAP=${GAP:-90}
WIN=${WIN:-30}
SERIAL=${ANDROID_SERIAL:-R5CW60624NE}
export ANDROID_SERIAL="$SERIAL"
OUT_ROOT="docs/bench/data/2x-vs-1x-3206/android-a54/$SCENARIO"

ap_temp() {
  adb -s "$SERIAL" shell dumpsys thermalservice 2>/dev/null |
    grep -o 'mValue=[0-9.]*, mType=0, mName=AP' | head -1 | grep -o '[0-9.]*' | head -1
}

for i in $(seq 1 "$N"); do
  label="$PREFIX-r$i"
  out="$OUT_ROOT/$label"
  mkdir -p "$out"
  luma=$(adb -s "$SERIAL" exec-out screencap -p > "$out/screen.png" 2>/dev/null; python3 - "$out/screen.png" <<'PY'
import sys
from PIL import Image, ImageStat
print(round(ImageStat.Stat(Image.open(sys.argv[1]).convert('L')).mean[0], 1))
PY
)
  t0=$(ap_temp)
  echo "=== $label luma $luma AP ${t0} °C"
  python3 scripts/bench/2x-vs-1x/bench_android.py window \
    --label "$label" --build "$BUILD" --scenario "$SCENARIO" \
    --seconds "$WIN" --out "$out" --note "one session, pinned sky set live, no relaunch" || {
      echo "[FAIL] window $label"; continue; }
  python3 - "$out" "$luma" "$t0" "$(ap_temp)" <<'PY'
import json, sys
out, luma, t0, t1 = sys.argv[1:5]
json.dump({"luma_mean": float(luma), "thermal_start_c": float(t0) if t0 else None,
           "thermal_end_ap_c": float(t1) if t1 else None,
           "note": "pinned sky applied live in this session; window without relaunch"},
          open(f"{out}/state.json", "w"), indent=2)
PY
  [ "$i" -lt "$N" ] && sleep "$GAP"
done
echo "=== session done"
