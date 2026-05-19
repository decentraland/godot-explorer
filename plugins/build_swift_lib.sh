#!/bin/bash
# Build DclSwiftLib (iOS Swift GDExtension)
# Compiles the xcframework and installs it into godot/ios/dcl-swift-lib/
#
# Usage:
#   ./build_swift_lib.sh                # defaults to release
#   ./build_swift_lib.sh release        # optimized for size, stripped (~6.8MB)
#   ./build_swift_lib.sh debug          # debug symbols, no size optimizations (~21MB)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SWIFT_LIB_DIR="$SCRIPT_DIR/dcl-swift-lib"

MODE="${1:-release}"

case "$MODE" in
    release)
        CONFIG=Release
        ;;
    debug)
        CONFIG=Debug
        ;;
    *)
        echo "Error: unknown mode '$MODE' (use 'release' or 'debug')"
        exit 1
        ;;
esac

if [ ! -d "$SWIFT_LIB_DIR" ]; then
    echo "Error: dcl-swift-lib directory not found at $SWIFT_LIB_DIR"
    exit 1
fi

echo "Building DclSwiftLib (CONFIG=$CONFIG)..."
cd "$SWIFT_LIB_DIR"
make xcframework CONFIG="$CONFIG"

echo ""
echo "Done! Framework installed at: godot/ios/dcl-swift-lib/DclSwiftLib.framework"
