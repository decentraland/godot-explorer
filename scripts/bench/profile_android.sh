#!/usr/bin/env bash
# profile_android.sh — capture a CPU sampling profile from the Android device
# during the GP benchmark sampling window.
#
# Watches /tmp/dcl-bench-android.log for "PROFILE_WINDOW_BEGIN duration_s=<N>",
# spawns simpleperf record on the device for <N> seconds, then pulls perf.data,
# generates a flamegraph SVG via the NDK simpleperf scripts.
#
# Prereqs:
#   - Android NDK installed at $ANDROID_NDK_HOME or $ANDROID_HOME/ndk/<v>/
#   - App must have <profileable android:shell="true"/> in manifest (it does)
#   - adb device connected, app launched with bench-tag=<tag>
#
# Usage:
#   scripts/bench/profile_android.sh <tag>

set -euo pipefail

TAG="${1:-baseline}"
PKG="org.decentraland.godotexplorer"
LOG=/tmp/dcl-bench-android.log
OUT_DIR="bench-results/profiles/android-${TAG}-$(date -u +%Y%m%dT%H%M%SZ)"
mkdir -p "$OUT_DIR"

NDK="${ANDROID_NDK_HOME:-}"
if [[ -z "$NDK" ]]; then
  NDK="$(ls -d "${ANDROID_HOME:-$HOME/Library/Android/sdk}/ndk/"*/ 2>/dev/null | tail -1)"
fi
SP_DIR="${NDK%/}/simpleperf"
if [[ ! -d "$SP_DIR" ]]; then
  echo "ERROR: NDK simpleperf scripts not found at $SP_DIR" >&2
  exit 2
fi

echo "[profile] waiting for PROFILE_WINDOW_BEGIN in $LOG (5 min cap)"
DURATION=""
for _ in {1..300}; do
  line=$(grep "PROFILE_WINDOW_BEGIN" "$LOG" 2>/dev/null | tail -1 || true)
  if [[ -n "$line" ]]; then
    DURATION=$(echo "$line" | sed -nE 's/.*duration_s=([0-9]+).*/\1/p')
    [[ -n "$DURATION" ]] && break
  fi
  sleep 1
done
if [[ -z "$DURATION" ]]; then
  echo "[profile] timed out waiting for marker" >&2
  exit 1
fi
echo "[profile] window opened, recording ${DURATION}s"

adb shell "rm -f /data/local/tmp/perf.data; \
  simpleperf record --app $PKG -f 1000 --duration $DURATION -g \
    --call-graph dwarf -o /data/local/tmp/perf.data" 2>&1 | tail -5

adb pull /data/local/tmp/perf.data "$OUT_DIR/perf.data" 2>&1 | tail -1

echo "[profile] generating report.txt"
python3 "$SP_DIR/report.py" -i "$OUT_DIR/perf.data" -g --no_browser \
    > "$OUT_DIR/report.txt" 2>&1 || true

echo "[profile] generating flamegraph.svg"
python3 "$SP_DIR/report_html.py" -i "$OUT_DIR/perf.data" \
    -o "$OUT_DIR/report.html" 2>&1 | tail -3 || true

# Inferno-style flamegraph (FlameGraph.pl format)
python3 "$SP_DIR/report_sample.py" -i "$OUT_DIR/perf.data" \
    > "$OUT_DIR/stacks.folded" 2>/dev/null || true
if [[ -s "$OUT_DIR/stacks.folded" ]] && command -v flamegraph.pl >/dev/null 2>&1; then
  flamegraph.pl "$OUT_DIR/stacks.folded" > "$OUT_DIR/flamegraph.svg"
fi

echo "[profile] done → $OUT_DIR"
ls -la "$OUT_DIR"
