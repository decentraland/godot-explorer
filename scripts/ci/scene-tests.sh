#!/usr/bin/env bash
#
# Runs the scene tests in tests/scene-tests/ against an already built client library, the same
# way locally and in CI. Exits 0 when every scene reported and passed.
#
#   scripts/ci/scene-tests.sh                  every scene, headless: render tests are skipped
#   scripts/ci/scene-tests.sh --render         with a window, and fails if a render test is skipped
#   scripts/ci/scene-tests.sh --windowed       watch it run (render tests run too)
#   scripts/ci/scene-tests.sh --scene locomotion,touch-controls   only these scenes (names in scenes.json)
#   scripts/ci/scene-tests.sh --scene locomotion --watch   rerun on every save or library rebuild
#   scripts/ci/scene-tests.sh -- --rust-log=debug   extra Godot arguments after --
#
# Godot comes from $GODOT4_BIN or `cargo run -- install` (.bin/godot/godot4_bin). The per-scene
# summary also goes to $GITHUB_STEP_SUMMARY when it is set.
set -euo pipefail

SCENES=""
HEADLESS=1
REQUIRE_RENDER=0
PORT=7666
EXTRA=()
WATCH=0
RUN_ARGS=()
for arg in "$@"; do
  [ "$arg" = --watch ] || RUN_ARGS+=("$arg")
done
while [ $# -gt 0 ]; do
  case "$1" in
    --scene) SCENES="${SCENES:+$SCENES,}$2"; shift 2 ;;
    --windowed) HEADLESS=0; shift ;;
    --render) HEADLESS=0; REQUIRE_RENDER=1; shift ;;
    --watch) WATCH=1; shift ;;
    --) shift; EXTRA=("$@"); break ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SUITE="$ROOT/tests/scene-tests"

# Watching: run, then wait until a test scene, the test library or the built client library
# changes, and run again; a failing run does not stop the loop.
if [ "$WATCH" = 1 ]; then
  fingerprint() {
    python3 -I - "$SUITE" "$ROOT/lib/target" <<'EOF'
import hashlib, os, sys
suite, target = sys.argv[1], sys.argv[2]
skip = {"node_modules", "bin", "static", ".exports", ".build-hashes"}
total = []
for folder, dirs, files in os.walk(suite):
    dirs[:] = [d for d in dirs if d not in skip]
    for name in files:
        if name not in ("main.crdt", "scene-tests.log"):
            stat = os.stat(os.path.join(folder, name))
            total.append((folder, name, stat.st_mtime_ns, stat.st_size))
for lib in ("libdclgodot_macos/libdclgodot.dylib", "libdclgodot_linux/libdclgodot.so"):
    path = os.path.join(target, lib)
    if os.path.exists(path):
        total.append((lib, os.stat(path).st_mtime_ns))
print(hashlib.sha1(repr(sorted(total)).encode()).hexdigest())
EOF
  }
  while true; do
    "$0" ${RUN_ARGS[@]+"${RUN_ARGS[@]}"} || true
    echo
    echo "Watching tests/scene-tests and the client library; Ctrl-C to stop."
    before="$(fingerprint)"
    while [ "$(fingerprint)" = "$before" ]; do sleep 1; done
  done
fi

GODOT="${GODOT4_BIN:-$ROOT/.bin/godot/godot4_bin}"
LOG="$SUITE/scene-tests.log"

# Same list as src/scene_tests.rs: every scene, or the ones named with --scene. A scene that
# cannot run yet stays in the list with its tests as skipTest.
PARCELS="$(python3 -I - "$SUITE/scenes.json" "$SCENES" <<'EOF'
import json, sys
scenes = json.load(open(sys.argv[1]))["scenes"]
names = [name for name in sys.argv[2].split(",") if name]
by_name = {s["name"]: s for s in scenes}
unknown = [name for name in names if name not in by_name]
if unknown:
    sys.exit(f"unknown scene {', '.join(unknown)}; scenes: {', '.join(by_name)}")
picked = [by_name[name] for name in names] if names else scenes
print(json.dumps([s["parcel"] for s in picked], separators=(",", ":")))
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

echo "Running the scene tests: $PARCELS"
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
# Skipped tests never fail a run, but they are counted so none is forgotten; a --render run
# exists to draw, so a render test skipped there fails it.
UNRENDERED="$(grep -c '⏭️ Skipped (no renderer)' "$LOG" || true)"
SKIPPED="$(grep -c '⏭️ Skipped [^(]' "$LOG" || true)"
if [ "$REQUIRE_RENDER" = 1 ] && [ "$UNRENDERED" != 0 ]; then
  echo "--render: $UNRENDERED render tests were skipped (Godot did not draw)" >&2
  RESULT=1
fi
if [ "$CODE" != 0 ] && [ "$RESULT" != "$CODE" ]; then
  echo "warning: Godot exited with code $CODE after printing its summary" >&2
fi

SUMMARY="$(grep -E '🟢|🔴|All test of all scene passed|Some tests fail' "$LOG" || echo "No scene reported (exit code $CODE); see $LOG")"
SUMMARY="$SUMMARY
Skipped: $UNRENDERED render tests (no renderer), $SKIPPED switched off (skipTest)"
if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
  {
    echo "### Scene tests${SCENES:+ ($SCENES)}"
    echo '```'
    echo "$SUMMARY"
    echo '```'
  } >> "$GITHUB_STEP_SUMMARY"
fi
echo
echo "$SUMMARY"
exit "$RESULT"
