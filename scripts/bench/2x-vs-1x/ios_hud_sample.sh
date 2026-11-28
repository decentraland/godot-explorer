#!/usr/bin/env bash
# iOS has no frame timeline (Perfetto is Android-only), but the engine HUD is in the accessibility
# tree, so fps can be sampled from it. 1 Hz resolution: this gives an average, not a tail.
#
#   SETTLE=25 SAMPLES=20 scripts/bench/2x-vs-1x/ios_hud_sample.sh 2x-ios-r1
set -euo pipefail
LABEL=${1:?run label}
SETTLE=${SETTLE:-25}
SAMPLES=${SAMPLES:-20}
IPHONE=${IPHONE:-/Users/lordmanuel/bin/iphone}
OUT="docs/bench/data/2x-vs-1x-3206/ios-16e/gp-hi-ios/$LABEL"
mkdir -p "$OUT"
sleep "$SETTLE"
for _ in $(seq 1 "$SAMPLES"); do
  "$IPHONE" ui 2>/dev/null | grep -o '[0-9]* FPS' | head -1 || true
  sleep 1.5
done | tee "$OUT/samples.txt"
"$IPHONE" shot "$OUT/screen.png" >/dev/null 2>&1 || true
python3 - "$OUT" <<'PY'
import statistics as st, sys
out = sys.argv[1]
vals = [int(l.split()[0]) for l in open(f"{out}/samples.txt") if l.strip() and l.split()[0].isdigit()]
print(f"fps median {st.median(vals)} min {min(vals)} max {max(vals)} (n={len(vals)}, HUD-sampled)")
PY
