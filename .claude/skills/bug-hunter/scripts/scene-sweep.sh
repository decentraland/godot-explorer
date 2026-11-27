#!/usr/bin/env bash
# Teleports the desktop client (attached to the debug hub) through parcels and records, per parcel:
# load time, client state, loaded scenes, new log errors/warnings, a screenshot and the log slice.
#   scene-sweep.sh OUTDIR "x,y" ["x,y" ...]
#   scene-sweep.sh OUTDIR --top N        the N most active places right now (places API)
# A scene missing from `scenes`, `scenes_loaded: 0`, "Scene crashed" / "not responding" / "script
# error" lines are the leads. PORT is passed through to unified.sh (default 9230).
set -uo pipefail
REPO=$(git rev-parse --show-toplevel)
H="$REPO/.claude/skills/mobile-dev-debug-tool/scripts"
case "$(uname)" in
  Darwin) LOG="$HOME/Library/Application Support/Godot/app_userdata/Decentraland/logs/godot.log" ;;
  *) LOG="$HOME/.local/share/godot/app_userdata/Decentraland/logs/godot.log" ;;
esac
OUT="${1:?usage: $0 OUTDIR x,y ... | --top N}"; shift
mkdir -p "$OUT"
if [ "${1:-}" = "--top" ]; then
  set -- $(curl -s "https://places.decentraland.org/api/places?order_by=most_active&order=desc&limit=${2:-10}" \
    | jq -r '.data[].base_position')
fi

for p in "$@"; do
  x=${p%,*}; y=${p#*,}; tag="p${x}_${y}"
  start=$(wc -l < "$LOG")
  "$H/unified.sh" teleport "{\"x\":$x,\"y\":$y}" > /dev/null
  t0=$(date +%s)
  for _ in $(seq 1 36); do
    sleep 5
    l=$("$H/unified.sh" state 2>/dev/null | jq -r '.data.loading')
    [ "$l" = "false" ] && [ $(( $(date +%s) - t0 )) -ge 10 ] && break
  done
  sleep 8   # a scene that dies does so in its first seconds
  state=$("$H/unified.sh" state 2>/dev/null | jq -c '.data')
  scenes=$("$H/unified.sh" scenes 2>/dev/null \
    | jq -c '[.data[] | {title, ents: .entity_count, missed: .missed_pct, p95: .js_tick_p95_ms}]')
  "$H/unified.sh" screenshot '{"inline":true}' 2>/dev/null | jq -r '.data.png_base64' \
    | base64 -d > "$OUT/$tag.png" 2>/dev/null
  tail -n +$((start + 1)) "$LOG" > "$OUT/$tag.log"
  echo "== $p load=$(( $(date +%s) - t0 ))s errors=$(grep -cE '^ERROR|^USER ERROR|panicked' "$OUT/$tag.log") warnings=$(grep -cE '^WARNING|^USER WARNING' "$OUT/$tag.log")"
  echo "   state:  $state"
  echo "   scenes: $scenes"
  grep -hoE "Scene crashed[^(]*|scene not responding[^@]*|script error [a-zA-Z]+: [^(]{0,90}" "$OUT/$tag.log" \
    | sort | uniq -c | sed 's/^/   ! /' | head -5
done
exit 0
