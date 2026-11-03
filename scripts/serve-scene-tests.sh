#!/usr/bin/env bash
#
# Builds the scene tests in tests/scene-tests/ as a static realm and serves it on localhost.
# The realm is http://localhost:<port>/scene-explorer-tests (default port 7666).
#
#   scripts/serve-scene-tests.sh              build if the sources changed, then serve
#   scripts/serve-scene-tests.sh --build-only build if the sources changed, then exit
#   scripts/serve-scene-tests.sh --port 8000
set -euo pipefail

PORT=7666
SERVE=1
while [ $# -gt 0 ]; do
  case "$1" in
    --port) PORT="$2"; shift 2 ;;
    --build-only) SERVE=0; shift ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

SUITE="$(cd "$(dirname "$0")/../tests/scene-tests" && pwd)"
cd "$SUITE"

# nvm installs are not on PATH in non-interactive shells.
if ! command -v npm >/dev/null 2>&1 && [ -s "${NVM_DIR:-$HOME/.nvm}/nvm.sh" ]; then
  . "${NVM_DIR:-$HOME/.nvm}/nvm.sh" >/dev/null
fi

hash_of() {
  find "$@" -type f \
    -not -path '*/node_modules/*' -not -path '*/bin/*' -not -name '.DS_Store' -not -name main.crdt \
    -print0 2>/dev/null | LC_ALL=C sort -z | xargs -0 shasum | shasum | cut -d' ' -f1
}

# Each scene is built and exported on its own, and only when its folder changed; a change to what
# they all share (or to the port, which the export embeds) redoes every one. The realm in static/
# is then assembled from the per-scene exports. The hashes and exports live outside static/.
HASHES=.build-hashes
EXPORTS=.exports
mkdir -p "$HASHES" "$EXPORTS"
SHARED="$(hash_of testing-library package.json package-lock.json)-port-$PORT"
if [ "$(cat "$HASHES/shared" 2>/dev/null)" != "$SHARED" ]; then
  rm -rf "${HASHES:?}"/scene-* "${EXPORTS:?}"/*
fi

if [ ! -d node_modules ] || [ package-lock.json -nt node_modules ]; then
  npm ci --no-audit --no-fund
fi

build_scene() {
  local scene="$1" hash="$2"
  (cd "$scene" && npx sdk-commands build --skip-install >/dev/null) || { echo "Building $scene failed" >&2; return 1; }
  rm -rf "${EXPORTS:?}/$scene"
  npx sdk-commands export-static --dir "$scene" --destination "$EXPORTS/$scene" \
    --realmName scene-explorer-tests --baseUrl "http://localhost:$PORT/" >/dev/null || return 1
  echo "$hash" > "$HASHES/scene-$scene"
  echo "Built $scene"
}

PENDING=()
for scene in */; do
  scene="${scene%/}"
  # scene folders are named x,y-name
  [[ "$scene" =~ ^-?[0-9]+,-?[0-9]+- ]] && [ -f "$scene/scene.json" ] || continue
  HASH="$(hash_of "$scene")"
  if [ "$(cat "$HASHES/scene-$scene" 2>/dev/null)" != "$HASH" ] || [ ! -d "$EXPORTS/$scene" ]; then
    PENDING+=("$scene" "$HASH")
  fi
done
CHANGED=0
JOBS="$(sysctl -n hw.ncpu 2>/dev/null || nproc 2>/dev/null || echo 4)"
# In batches of one per core (bash 3.2 on macOS has no `wait -n`).
FAILED=0
PIDS=()
for ((i = 0; i < ${#PENDING[@]}; i += 2)); do
  build_scene "${PENDING[i]}" "${PENDING[i + 1]}" &
  PIDS+=($!)
  CHANGED=1
  if [ "${#PIDS[@]}" -ge "$JOBS" ] || [ $((i + 2)) -ge "${#PENDING[@]}" ]; then
    for pid in "${PIDS[@]}"; do wait "$pid" || FAILED=1; done
    PIDS=()
  fi
done
[ "$FAILED" = 0 ] || { echo "Some scene tests did not build" >&2; exit 1; }
echo "$SHARED" > "$HASHES/shared"

# Scenes removed from the suite leave the realm too.
for exported in "$EXPORTS"/*/; do
  [ -d "$exported" ] || continue
  exported="$(basename "$exported")"
  if [ ! -d "$exported" ]; then
    rm -rf "${EXPORTS:?}/$exported" "$HASHES/scene-$exported"
    CHANGED=1
  fi
done

if [ "$CHANGED" = 1 ] || [ ! -f static/scene-explorer-tests/about ]; then
  python3 -I - "$EXPORTS" static <<'EOF'
import json, os, shutil, sys
exports, static = sys.argv[1], sys.argv[2]
shutil.rmtree(static, ignore_errors=True)
about, urns = None, []
for scene in sorted(os.listdir(exports)):
    root = os.path.join(exports, scene)
    for folder, _, files in os.walk(root):
        relative = os.path.relpath(folder, root)
        os.makedirs(os.path.join(static, relative), exist_ok=True)
        for name in files:
            if relative == "scene-explorer-tests" and name == "about":
                scene_about = json.load(open(os.path.join(folder, name)))
                about = about or scene_about
                urns += scene_about["configurations"]["scenesUrn"]
                continue
            target = os.path.join(static, relative, name)
            if not os.path.exists(target):
                os.link(os.path.join(folder, name), target)
about["configurations"]["scenesUrn"] = urns
json.dump(about, open(os.path.join(static, "scene-explorer-tests", "about"), "w"))
print(f"Assembled the realm in {static}/ ({len(urns)} scenes)")
EOF
else
  echo "Scene tests unchanged, reusing static/"
fi

if [ "$SERVE" = 1 ]; then
  echo "Serving the realm http://localhost:$PORT/scene-explorer-tests"
  # The client asks for every scene at once; `python3 -m http.server` keeps a backlog of 5 and
  # resets the rest, and a scene whose entity request fails never loads.
  exec python3 -I - "$PORT" static <<'EOF'
import functools, http.server, sys
port, root = int(sys.argv[1]), sys.argv[2]
class Server(http.server.ThreadingHTTPServer):
    request_queue_size = 128
handler = functools.partial(http.server.SimpleHTTPRequestHandler, directory=root)
Server(("127.0.0.1", port), handler).serve_forever()
EOF
fi
