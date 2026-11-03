#!/usr/bin/env bash
#
# Runs the scene tests in tests/scene-tests/ against an already built client library, the same
# way locally and in CI. Exits 0 when every scene of the tier reported and passed.
#
#   scripts/ci/scene-tests.sh                  ecs tier (no screenshots), headless
#   scripts/ci/scene-tests.sh --tier all       every enabled scene
#   scripts/ci/scene-tests.sh --windowed       watch it run
#   scripts/ci/scene-tests.sh -- --rust-log=debug   extra Godot arguments after --
#
# Godot comes from $GODOT4_BIN or `cargo run -- install` (.bin/godot/godot4_bin). The per-scene
# summary also goes to $GITHUB_STEP_SUMMARY when it is set.
set -euo pipefail

TIER=ecs
HEADLESS=1
PORT=7666
EXTRA=()
while [ $# -gt 0 ]; do
  case "$1" in
    --tier) TIER="$2"; shift 2 ;;
    --windowed) HEADLESS=0; shift ;;
    --) shift; EXTRA=("$@"); break ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SUITE="$ROOT/tests/scene-tests"
GODOT="${GODOT4_BIN:-$ROOT/.bin/godot/godot4_bin}"
LOG="$SUITE/scene-tests.log"

# Same rule as src/scene_tests.rs: disabled scenes never run; `all` takes every other one.
PARCELS="$(python3 -I - "$SUITE/scenes.json" "$TIER" <<'EOF'
import json, sys
scenes = json.load(open(sys.argv[1]))["scenes"]
tier = sys.argv[2]
picked = [s["parcel"] for s in scenes if s["tier"] != "disabled" and tier in ("all", s["tier"])]
if not picked:
    sys.exit(f"no scenes in tier '{tier}' (expected ecs, render or all)")
print(json.dumps(picked, separators=(",", ":")))
EOF
)"

"$ROOT/scripts/serve-scene-tests.sh" --build-only --port "$PORT"
"$ROOT/scripts/serve-scene-tests.sh" --port "$PORT" >/dev/null 2>&1 &
SERVER=$!
trap 'kill "$SERVER" 2>/dev/null || true' EXIT

REALM="http://localhost:$PORT/scene-explorer-tests"
for _ in $(seq 1 30); do
  curl -fs "$REALM/about" >/dev/null && break
  sleep 1
done
curl -fs "$REALM/about" >/dev/null || { echo "the scene-test realm did not come up on :$PORT" >&2; exit 1; }

# A fixed frame step: the same input sequence moves the player the same way on any machine.
ARGS=(--path "$ROOT/godot" --fixed-fps 60)
[ "$HEADLESS" = 1 ] && ARGS+=(--headless)
ARGS+=(--scene-test "$PARCELS" --realm "$REALM" --snapshot-folder "$ROOT/tests/snapshots/scenes")
ARGS+=(${EXTRA[@]+"${EXTRA[@]}"})

echo "Running the '$TIER' scene tests: $PARCELS"
set +e
"$GODOT" "${ARGS[@]}" 2>&1 | tee "$LOG"
CODE=${PIPESTATUS[0]}
set -e

# The verdict is Godot's summary line, as in xtask: Godot can still crash while quitting after it.
if grep -q 'All test of all scene passed' "$LOG"; then
  RESULT=0
elif grep -q 'Some tests fail' "$LOG"; then
  RESULT=1
else
  RESULT=$(( CODE == 0 ? 1 : CODE ))
fi
# A Rust panic while the tests ran fails the run even when every test passed: the panic may have
# cut a system short without failing its scene. The known crash on quit comes after the summary.
PANICS="$(awk '/All test of all scene passed|Some tests fail/ { exit } /\[Rust:panic\]/' "$LOG")"
if [ -n "$PANICS" ]; then
  echo "Rust panics while the tests ran:" >&2
  echo "$PANICS" | sed 's/\x1b\[[0-9;]*m//g' >&2
  RESULT=1
fi
if [ "$CODE" != 0 ] && [ "$RESULT" != "$CODE" ]; then
  echo "warning: Godot exited with code $CODE after printing its summary" >&2
fi

SUMMARY="$(grep -E '🟢|🔴|All test of all scene passed|Some tests fail' "$LOG" || echo "No scene reported (exit code $CODE); see $LOG")"
if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
  {
    echo "### Scene tests ($TIER)"
    echo '```'
    echo "$SUMMARY"
    echo '```'
  } >> "$GITHUB_STEP_SUMMARY"
fi
echo
echo "$SUMMARY"
exit "$RESULT"
