#!/usr/bin/env bash
# profile_android_gpu.sh — capture a GPU perfetto trace from the Android
# device during the GP benchmark sampling window. Mirrors profile_android.sh
# but uses perfetto + AGI's Mali producer instead of simpleperf.
#
# Watches /tmp/dcl-bench-android.log for "PROFILE_WINDOW_BEGIN duration_s=<N>",
# captures a perfetto trace for <N> seconds with gpu.renderstages +
# gpu.counters + ftrace, pulls the trace, runs `gapit perfetto -mode metrics`
# to extract per-renderpass GPU time as JSON, and emits a top-N renderpass
# summary.
#
# Prereqs:
#   - AGI installed at $AGI_HOME or /Applications/AGI.app/Contents/MacOS/
#   - App must have `<profileable android:shell="true"/>` in manifest
#   - adb device connected, app launched with `bench-tag=<tag>` deeplink
#
# Why we activate the producer at start (gapit validate_gpu_profiling):
#   the Mali libgpudataproducer.so isn't running by default. Without it
#   `gpu.renderstages` / `gpu.counters` data sources emit nothing.
#   `validate_gpu_profiling` calls into the producer's start() and leaves
#   it primed for the duration of this run.
#
# Why we clean up gpu_debug_* settings before starting:
#   if any prior `gapit trace -api vulkan` ran on the device it leaves the
#   GraphicsSpy + KhronosValidation Vulkan layers globally injected via
#   `settings put global gpu_debug_layers ...`. That makes the app crash
#   on launch. Always wipe before profiling.
#
# Usage:
#   scripts/bench/profile_android_gpu.sh <tag>

set -uo pipefail
# NOTE: deliberately NOT using `set -e` because gapit subcommands often
# exit non-zero on shutdown ("context cancelled" during gapis kill) even
# when the operation succeeded. We check exit codes explicitly where
# correctness matters.

TAG="${1:-baseline}"
PKG="org.decentraland.godotexplorer"
LOG=/tmp/dcl-bench-android.log
OUT_DIR="bench-results/profiles/android-gpu-${TAG}-$(date -u +%Y%m%dT%H%M%SZ)"
mkdir -p "$OUT_DIR"

# Locate AGI's gapit binary.
AGI_HOME="${AGI_HOME:-/Applications/AGI.app/Contents/MacOS}"
GAPIT="${AGI_HOME}/gapit"
if [[ ! -x "$GAPIT" ]]; then
  echo "ERROR: gapit not found at $GAPIT — install AGI or set AGI_HOME" >&2
  exit 2
fi

# Locate perfetto config relative to this script.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CFG_LOCAL="$SCRIPT_DIR/perfetto_gpu.cfg"
if [[ ! -f "$CFG_LOCAL" ]]; then
  echo "ERROR: perfetto_gpu.cfg not found at $CFG_LOCAL" >&2
  exit 2
fi

# Sticky-settings wipe — see header for why.
echo "[gpu-profile] cleaning gpu_debug_* settings (sticky from prior AGI runs)"
adb shell settings delete global enable_gpu_debug_layers >/dev/null 2>&1 || true
adb shell settings delete global gpu_debug_layers       >/dev/null 2>&1 || true
adb shell settings delete global gpu_debug_app          >/dev/null 2>&1 || true
adb shell settings delete global gpu_debug_layer_app    >/dev/null 2>&1 || true

# Activate the Mali GPU producer once. The validation also installs the
# gapid helper APK on first run; subsequent runs are fast (~5 s).
SERIAL="$(adb get-serialno 2>/dev/null || echo "")"
if [[ -z "$SERIAL" || "$SERIAL" == "unknown" ]]; then
  echo "ERROR: no adb serial found — connect a device first" >&2
  exit 2
fi
echo "[gpu-profile] activating Mali producer via gapit validate_gpu_profiling (device=$SERIAL)"
"$GAPIT" validate_gpu_profiling -gapis-port=0 2>&1 \
  | tail -8 \
  | sed 's/^/[gpu-profile] /'

# Push the perfetto config into a path readable by the on-device perfetto
# binary (running as system uid via shell).
adb push "$CFG_LOCAL" /data/local/tmp/perfetto_gpu.cfg >/dev/null

# Watch logcat for the bench's PROFILE_WINDOW_BEGIN marker.
echo "[gpu-profile] waiting for PROFILE_WINDOW_BEGIN in $LOG (5 min cap)"
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
  echo "[gpu-profile] timed out waiting for marker" >&2
  exit 1
fi
echo "[gpu-profile] window opened, capturing ${DURATION}s of perfetto"

# Append duration_ms to the config so the on-device trace stops by itself.
TRACE_DEVICE_PATH="/data/misc/perfetto-traces/dcl-gpu-${TAG}.perfetto"
adb shell "rm -f $TRACE_DEVICE_PATH"
adb shell "(cat /data/local/tmp/perfetto_gpu.cfg; echo 'duration_ms: $((DURATION*1000))') \
  | perfetto -c - --txt -o $TRACE_DEVICE_PATH" 2>&1 | tail -3 \
  | sed 's/^/[gpu-profile] /'

# Pull the trace.
TRACE_LOCAL="$OUT_DIR/trace.perfetto"
adb pull "$TRACE_DEVICE_PATH" "$TRACE_LOCAL" 2>&1 | tail -1
TRACE_BYTES=$(stat -f%z "$TRACE_LOCAL" 2>/dev/null || stat -c%s "$TRACE_LOCAL" 2>/dev/null || echo 0)
echo "[gpu-profile] trace size: ${TRACE_BYTES} bytes"

# Run gapit perfetto -mode metrics to compute the standard metric set.
# `-categories CPU,Memory,Graphics.Memory.GPU` is the Graphics-relevant
# subset. -format json so we can post-process.
echo "[gpu-profile] running gapit perfetto -mode metrics"
"$GAPIT" perfetto \
  -in "$TRACE_LOCAL" \
  -mode metrics \
  -categories "CPU,Memory,Graphics.Memory.GPU" \
  -format json \
  -out "$OUT_DIR/metrics.json" 2>&1 | tail -5 \
  | sed 's/^/[gpu-profile] /' || true

# Top-N renderpasses summary via interactive SQL. We pipe a single SELECT
# that reads gpu_slice (the table the Mali producer fills) and dumps top
# 30 by total GPU time across the trace.
SQL=$(cat <<'SQL'
SELECT name,
       COUNT(*)                       AS n_passes,
       SUM(dur)/1e6                   AS total_ms,
       AVG(dur)/1e6                   AS avg_ms,
       MAX(dur)/1e6                   AS max_ms
FROM gpu_slice
GROUP BY name
ORDER BY total_ms DESC
LIMIT 30;
SQL
)
echo "[gpu-profile] top renderpasses:"
echo "$SQL" \
  | "$GAPIT" perfetto -in "$TRACE_LOCAL" -mode interactive 2>/dev/null \
  | tee "$OUT_DIR/top_renderpasses.txt" \
  | sed 's/^/[gpu-profile] /' \
  || echo "[gpu-profile] (interactive mode failed, see metrics.json)"

echo "[gpu-profile] done → $OUT_DIR"
ls -la "$OUT_DIR"
