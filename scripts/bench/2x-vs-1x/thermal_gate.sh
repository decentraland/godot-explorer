#!/usr/bin/env bash
# Wait for a running condition before a measurement window.
#
# The gate is conjunctive because each condition alone has failed us: Thermal Status covers the system's
# own policy, AP is what actually throttles the SoC, SKIN is what the policy follows, and the time floor
# stops two windows from landing back to back after a fast cooldown.
#
# It writes its verdict OUTSIDE the run directory, because run_paired.sh wipes that directory before the
# window; a ledger inside it would be deleted and every run would then look ungated.
#
#   LEDGER=/path/gates.jsonl thermal_gate.sh --since <end_epoch_of_previous_window>
set -euo pipefail

SERIAL=${ANDROID_SERIAL:-R5CW60624NE}
GATE_TIMEOUT=${GATE_TIMEOUT:-1800}
FLOOR_S=${FLOOR_S:-480}
AP_MAX=${AP_MAX:-40.0}
SKIN_MARGIN=${SKIN_MARGIN:-1.0}
LEDGER=${LEDGER:-${TMPDIR:-/tmp}/gates-$(date +%Y%m%d).jsonl}
BASELINE=${BASELINE:-}   # skin baseline in °C; unset means "use the first reading of this invocation"

sh_() { adb -s "$SERIAL" shell "$@" 2>/dev/null | tr -d '\r'; }

since=""
while [ $# -gt 0 ]; do
  case "$1" in
    --since) since=${2:?}; shift 2 ;;
    -h|--help) echo "usage: $0 [--since EPOCH]"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 64 ;;
  esac
done

status=$(sh_ dumpsys thermalservice | sed -n 's/.*Thermal Status: *\([0-9]*\).*/\1/p' | head -1)
read_temp() {  # $1 = mName
  sh_ dumpsys thermalservice |
    grep -o "mValue=[0-9.]*, mType=[0-9]*, mName=$1" | head -1 | sed -n 's/mValue=\([0-9.]*\).*/\1/p'
}
ap=$(read_temp AP)
skin=$(read_temp SKIN)
skin_source=thermalservice
if [ -z "$skin" ]; then
  raw=$(sh_ dumpsys battery | sed -n 's/.*temperature: *\([0-9]*\).*/\1/p' | head -1)
  [ -n "$raw" ] && skin=$(awk -v t="$raw" 'BEGIN{printf "%.1f", t/10}')
  skin_source=battery
fi
[ -n "$BASELINE" ] || BASELINE=${skin:-0}

start=$(date +%s)
now=$start
reason=""
while :; do
  now=$(date +%s)
  waited=$((now - start))
  since_prev=""
  [ -n "$since" ] && since_prev=$((now - since))

  reason=""
  [ -n "$status" ] || reason="thermal status unreadable"
  if [ -z "$reason" ] && [ "$status" -gt 1 ]; then reason="thermal status $status > 1"; fi
  if [ -z "$reason" ] && [ -n "$ap" ] && awk -v a="$ap" -v m="$AP_MAX" 'BEGIN{exit !(a>m)}'; then
    reason="AP ${ap}C > ${AP_MAX}C"
  fi
  if [ -z "$reason" ] && [ -n "$skin" ] && [ -n "$BASELINE" ] &&
     awk -v s="$skin" -v b="$BASELINE" -v m="$SKIN_MARGIN" 'BEGIN{exit !(s>b+m)}'; then
    reason="skin ${skin}C > baseline ${BASELINE}C + ${SKIN_MARGIN}C"
  fi
  if [ -z "$reason" ] && [ -n "$since_prev" ] && [ "$since_prev" -lt "$FLOOR_S" ]; then
    reason="only ${since_prev}s since the previous window (floor ${FLOOR_S}s)"
  fi

  if [ -z "$reason" ]; then
    printf '%s\n' "{\"ts\":$now,\"status\":${status:-null},\"ap_c\":${ap:-null},\"skin_c\":${skin:-null},\"skin_source\":\"$skin_source\",\"baseline_c\":${BASELINE:-null},\"wait_s\":$waited,\"since_prev_end_s\":${since_prev:-null},\"floor_s\":$FLOOR_S,\"passed\":true}" >>"$LEDGER"
    echo "[gate] pass after ${waited}s: status=${status} ap=${ap} skin=${skin} since_prev=${since_prev:-n/a}"
    exit 0
  fi

  if [ "$waited" -ge "$GATE_TIMEOUT" ]; then
    printf '%s\n' "{\"ts\":$now,\"status\":${status:-null},\"ap_c\":${ap:-null},\"skin_c\":${skin:-null},\"skin_source\":\"$skin_source\",\"baseline_c\":${BASELINE:-null},\"wait_s\":$waited,\"since_prev_end_s\":${since_prev:-null},\"floor_s\":$FLOOR_S,\"passed\":false,\"reason\":\"timeout: $reason\"}" >>"$LEDGER"
    echo "[gate] TIMEOUT after ${waited}s: $reason" >&2
    exit 2
  fi

  sleep 10
  status=$(sh_ dumpsys thermalservice | sed -n 's/.*Thermal Status: *\([0-9]*\).*/\1/p' | head -1)
  ap=$(read_temp AP)
  skin=$(read_temp SKIN)
  if [ -z "$skin" ]; then
    raw=$(sh_ dumpsys battery | sed -n 's/.*temperature: *\([0-9]*\).*/\1/p' | head -1)
    [ -n "$raw" ] && skin=$(awk -v t="$raw" 'BEGIN{printf "%.1f", t/10}')
    skin_source=battery
  fi
done
