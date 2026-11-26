#!/usr/bin/env bash
# Vertical centroid of every row of light text/solid landmark in a screenshot, for Figma-vs-device checks.
#   text_rows.sh <png> [--height H] [--crop WxH+X+Y] [--colors '#FCFCFC,#FFFFFF,#ECEBED,#A0ABFF']
# --height H first scales the image to H px tall (use the device height for the Figma shot, so both
# images share one y axis). Prints one line per band: y range, height and the centroid in px.
# Needs ImageMagick (`magick`). Thin text can split a line's ascenders and descenders into two bands:
# compare centroids, which stay put, rather than band edges.
set -euo pipefail
png="$1"; shift
height=""; crop=""; colors="#FCFCFC,#FFFFFF,#ECEBED,#A0ABFF"
while [ $# -gt 0 ]; do
  case "$1" in
    --height) height="$2"; shift 2 ;;
    --crop) crop="$2"; shift 2 ;;
    --colors) colors="$2"; shift 2 ;;
    *) echo "unknown arg $1" >&2; exit 2 ;;
  esac
done
args=()
[ -n "$height" ] && args+=(-resize "x$height")
[ -n "$crop" ] && args+=(-crop "$crop" +repage)
fills=()
IFS=',' read -ra cs <<<"$colors"
for c in "${cs[@]}"; do fills+=(-fill white -opaque "$c"); done
magick "$png" "${args[@]}" -fuzz 4% "${fills[@]}" -fill black +opaque white \
  -scale "1x!" -depth 16 -compress none pgm:- | python3 -I -c '
import sys
t = sys.stdin.read().split()
vals = [int(x) / 65535 * 100 for x in t[4:]]
bands, on, start = [], False, 0
for i, v in enumerate(vals):
    if v > 0.3 and not on: on, start = True, i
    if v <= 0.3 and on: on = False; bands.append((start, i - 1))
if on: bands.append((start, len(vals) - 1))
merged = []
for b in bands:
    if merged and b[0] - merged[-1][1] < 8: merged[-1] = (merged[-1][0], b[1])
    else: merged.append(b)
for a, b in merged:
    if b - a < 4: continue
    w = sum(vals[a:b + 1]); c = sum(i * vals[i] for i in range(a, b + 1)) / w
    print(f"y={a}-{b} h={b - a + 1} centroid={c:.1f}")
'
