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

# The export embeds the base URL, so the port is part of what the build depends on.
source_hash() {
  {
    echo "port=$PORT"
    find . -type f \
      -not -path '*/node_modules/*' -not -path './static/*' -not -path '*/bin/*' -not -name '.DS_Store' -not -name main.crdt \
      -print0 | LC_ALL=C sort -z | xargs -0 shasum
  } | shasum | cut -d' ' -f1
}

HASH="$(source_hash)"
if [ -f static/.source-hash ] && [ "$(cat static/.source-hash)" = "$HASH" ]; then
  echo "Scene tests unchanged, reusing static/"
else
  if [ ! -d node_modules ] || [ package-lock.json -nt node_modules ]; then
    npm ci --no-audit --no-fund
  fi
  npm run build
  rm -rf static
  npx sdk-commands export-static --destination static/ --realmName scene-explorer-tests \
    --baseUrl "http://localhost:$PORT/"
  echo "$HASH" > static/.source-hash
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
