#!/usr/bin/env bash
# Sun-drift sampling: Genesis Plaza drives its own day/night cycle, so the client setting cannot pin
# the sun. Instead of pretending it is fixed, the build is sampled repeatedly while the light moves,
# recording luma next to fps so the two builds can be compared at matched light (that is what
# sampling.csv and the fit in the report use).
#
#   BUILD=2x SAMPLES=8 GAP=180 scripts/bench/2x-vs-1x/run_drift.sh 2x-drift
set -euo pipefail
BUILD=${BUILD:?build (1x|2x)}
LABEL=${LABEL:?run label}
SAMPLES=${SAMPLES:-8}
GAP=${GAP:-90}
WINDOW=${WINDOW:-30}
SERIAL=${ANDROID_SERIAL:-R5CW60624NE}
PKG=org.decentraland.godotexplorer
HERE="$(cd "$(dirname "$0")" && pwd)"
OUT="docs/bench/data/2x-vs-1x-3206/android-a54/gp-sun-drift/$LABEL"
mkdir -p "$OUT"

sh_() { adb -s "$SERIAL" shell "$@"; }
ap_temp() { sh_ dumpsys thermalservice 2>/dev/null | grep 'mName=AP' | head -1 | sed -n 's/.*mValue=\([0-9.]*\).*/\1/p'; }
luma() { adb -s "$SERIAL" exec-out screencap -p > "$1" && python3 -c "
from PIL import Image, ImageStat
print(round(ImageStat.Stat(Image.open('$1').convert('L')).mean[0], 1))"; }

printf 'sample,luma,fps_avg,p50_ms,p99_ms,cpu_pct,ap_c,started_at\n' > "$OUT/sampling.csv"
for i in $(seq 1 "$SAMPLES"); do
  [ "$i" -gt 1 ] && sleep "$GAP"
  # The world is never relaunched inside the block: a scene load inside the window is a different
  # measurement (it showed up as +45 % CPU and +5 fps). The first sample settles it instead.
  if [ "$i" -eq 1 ]; then
    sh_ am start -a android.intent.action.VIEW -d "'decentraland://open?position=0,0'" "$PKG" >/dev/null 2>&1 || true
    sleep "${SETTLE:-25}"
  fi
  l=$(luma "$OUT/sample-$i.png")
  python3 "$HERE/bench_android.py" --serial "$SERIAL" window --label "$LABEL-s$i" --build "$BUILD" --scenario gp-sun-drift --seconds "$WINDOW" --out "$OUT/s$i" > "$OUT/s$i.log"
  python3 - "$OUT" "$i" "$l" "$(ap_temp)" <<'PY'
import json, sys
out, i, luma, ap = sys.argv[1:5]
m = json.load(open(out + "/s" + i + "/meta.json"))
f = m.get("frames") or {}
row = [i, luma, f.get("fps_avg"), f.get("interval_p50_ms"), f.get("interval_p99_ms"), m.get("cpu_mean_pct"), ap, m.get("started_at", "")]
print(",".join("" if v is None else str(v) for v in row))
PY
done | tee -a "$OUT/sampling.csv"
echo "[drift] $LABEL done: $OUT/sampling.csv"
