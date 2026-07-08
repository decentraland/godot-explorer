#!/usr/bin/env bash
# profile_ios.sh — capture an iOS Time Profiler trace via xctrace during the
# GP benchmark sampling window.
#
# Watches /tmp/dcl-bench-ios.log for "PROFILE_WINDOW_BEGIN duration_s=<N>"
# (emitted by gp_benchmark_runner.gd at the start of `sampling` phase),
# then runs `xcrun xctrace record --template "Time Profiler"` against the
# iPhone for that duration. The .trace bundle is opened with Instruments.app
# (or the xctrace export CLI) for analysis.
#
# Prereqs:
#   - macOS with Xcode installed (Instruments.app + xctrace).
#   - iPhone paired and trusted; UDID in scripts/bench/.env (IOS_UDID) or
#     auto-detected from the first paired device.
#   - The app already running on the device (launch_devices.sh handles that;
#     this script only attaches the profiler).
#
# Usage:
#   scripts/bench/profile_ios.sh <bench-tag>
#
# Designed to be spawned by launch_devices.sh --profile.

set -euo pipefail

ENV_FILE="$(dirname "$0")/.env"
[[ -f "$ENV_FILE" ]] && set -a && source "$ENV_FILE" && set +a

TAG="${1:-baseline}"
PKG="org.decentraland.godotexplorer"
LOG=/tmp/dcl-bench-ios.log
OUT_DIR="bench-results/profiles/ios-${TAG}-$(date -u +%Y%m%dT%H%M%SZ)"
mkdir -p "$OUT_DIR"

if ! command -v xcrun >/dev/null 2>&1; then
  echo "[profile-ios] xcrun missing — install Xcode" >&2
  exit 2
fi

UDID="${IOS_UDID:-}"
if [[ -z "$UDID" ]]; then
  UDID="$(xcrun devicectl list devices 2>/dev/null \
    | awk '/available \(paired\)/ {print $4; exit}')"
fi
if [[ -z "$UDID" ]]; then
  echo "[profile-ios] no paired iPhone found (set IOS_UDID in .env)" >&2
  exit 2
fi

echo "[profile-ios] waiting for PROFILE_WINDOW_BEGIN in $LOG (5 min cap)"
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
  echo "[profile-ios] timed out waiting for marker" >&2
  exit 1
fi
echo "[profile-ios] window opened, recording ${DURATION}s on $UDID"

# xctrace attaches to a running process by name. The app should already be
# launched by launch_devices.sh. `--time-limit` accepts seconds.
TRACE_OUT="$OUT_DIR/profile.trace"
xcrun xctrace record \
  --template "Time Profiler" \
  --device "$UDID" \
  --attach "$PKG" \
  --time-limit "${DURATION}s" \
  --output "$TRACE_OUT" 2>&1 | tail -10 || true

if [[ ! -d "$TRACE_OUT" ]]; then
  echo "[profile-ios] WARN: trace bundle not produced at $TRACE_OUT" >&2
  exit 1
fi

# Try to export a flat function-time CSV alongside the .trace bundle so the
# bench analysis can grep without booting Instruments.app.
echo "[profile-ios] exporting function-time CSV"
xcrun xctrace export \
  --input "$TRACE_OUT" \
  --xpath '/trace-toc/run[@number="1"]/data/table[@schema="time-profile"]' \
  > "$OUT_DIR/time-profile.xml" 2>&1 || true

echo "[profile-ios] done → $OUT_DIR"
ls -la "$OUT_DIR"
