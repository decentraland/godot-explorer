#!/usr/bin/env bash
# VIBE-CODED SPIKE: copy the Godot engine (built with embedded_host=yes), the exported project
# pack and the GDExtension frameworks into rn/ios/Godot. macOS only; not run locally.
#
# Needs, from the repo root:
#   GODOT_HOST_LIB=<path to libgodot.ios.template_*.arm64.a built with embedded_host=yes>
#   cargo run -- build --target ios && cargo run -- export --target ios
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
DEST="$ROOT/rn/ios/Godot"
: "${GODOT_HOST_LIB:?set GODOT_HOST_LIB to the embedded_host libgodot .a}"

rm -rf "$DEST"
mkdir -p "$DEST/Frameworks"

cp "$GODOT_HOST_LIB" "$DEST/libgodot.a"
# The engine looks for <host executable>.pck in the bundle root, and the executable is DclRn.
cp "$ROOT/exports/decentraland-godot-client.pck" "$DEST/DclRn.pck"
find "$ROOT/exports" -maxdepth 4 -type d -name '*.framework' -not -name 'libgodot*' \
  -exec cp -R {} "$DEST/Frameworks/" \;
# The exporter ships GDExtension dylibs (libdclgodot.dylib) in a dylibs/ folder next to the Xcode project.
find "$ROOT/exports" -maxdepth 3 -type d -name dylibs -exec cp -R {} "$DEST/dylibs" \; -quit

echo "synced: $(ls "$DEST/Frameworks" | wc -l | tr -d ' ') frameworks, dylibs: $(ls "$DEST/dylibs" 2>/dev/null | wc -l | tr -d ' ')"
