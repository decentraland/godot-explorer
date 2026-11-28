#!/usr/bin/env bash
# One measured block of runs at one condition (#3206).
#
# A block is the unit that must survive review: every run in it is gated, measured, and given a manifest
# that a reviewer can reject later without touching the phone. The block stops at the first run that
# fails its manifest, because a block with a hole in the middle is not a block.
#
#   BUILD=2x SCENARIO=sky-worldclock POSITION=100,100 N=6 LABEL_PREFIX=2x-wc \
#   LEDGER=$PWD/docs/bench/data/2x-vs-1x-3206/android-a54/sky-worldclock/gates.jsonl \
#   scripts/bench/2x-vs-1x/campaign.sh
#
# The gate lives here and nowhere else: run_paired.sh is called with TARGET_TEMP=60 so its own cooldown
# passes through, and this script owns the real condition.
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$HERE/../../.." && pwd)"
cd "$REPO"

BUILD=${BUILD:?build 1x|2x}
SCENARIO=${SCENARIO:?scenario name}
N=${N:-6}
WINDOW=${WINDOW:-30}
SETTLE=${SETTLE:-20}
POSITION=${POSITION:-0,0}
LABEL_PREFIX=${LABEL_PREFIX:-$BUILD-$SCENARIO}
LEDGER=${LEDGER:-$REPO/docs/bench/data/2x-vs-1x-3206/android-a54/$SCENARIO/gates.jsonl}
CAMPAIGN=${CAMPAIGN:-}
FIRST_FRESH=${FIRST_FRESH:-1}   # 1: the first run of the block installs and applies settings
RAW_OUT="docs/bench/data/2x-vs-1x-3206/android-a54/$SCENARIO"

mkdir -p "$(dirname "$LEDGER")"
BLOCK_STATE=$(mktemp)

cleanup() { rm -f "$BLOCK_STATE"; }
trap cleanup EXIT

echo "[block] build=$BUILD scenario=$SCENARIO n=$N window=${WINDOW}s position=$POSITION ledger=$LEDGER"

for i in $(seq 1 "$N"); do
  label="$LABEL_PREFIX-r$i"
  out="$RAW_OUT/$label"
  since=$(cat "$BLOCK_STATE" 2>/dev/null || true)

  echo "[block] $label ($i/$N) ${since:+since=$since}"
  LEDGER="$LEDGER" "$HERE/thermal_gate.sh" ${since:+--since "$since"} || {
    echo "[block] ABORT: thermal gate did not pass for $label" >&2
    exit 2
  }

  settings_env=SKIP_SETTINGS=1
  if [ "$i" -eq 1 ] && [ "$FIRST_FRESH" = "1" ]; then
    settings_env=FORCE_SETTINGS=1
  fi

  env $settings_env SCENARIO="$SCENARIO" WINDOW="$WINDOW" SETTLE="$SETTLE" \
      POSITION="$POSITION" TARGET_TEMP=60 \
      "$HERE/run_paired.sh" "$BUILD" "$label" || {
    echo "[block] ABORT: run_paired.sh failed for $label" >&2
    exit 3
  }

  LEDGER="$LEDGER" CAMPAIGN="$CAMPAIGN" python3 "$HERE/manifest.py" build "$out" --gate "$LEDGER" ${CAMPAIGN:+--campaign "$CAMPAIGN"} || {
    echo "[block] ABORT: manifest could not be built for $label" >&2
    exit 4
  }
  if ! LEDGER="$LEDGER" python3 "$HERE/manifest.py" check "$out" --gate "$LEDGER"; then
    echo "[block] ABORT: $label failed its manifest check - see above" >&2
    exit 5
  fi

  date +%s > "$BLOCK_STATE"
  echo "[block] $label ok"
done

echo "[block] done: $N runs at $SCENARIO for $BUILD"
