#!/bin/bash
# Build DclSwiftLib for iOS WalletConnect integration
# This script builds the xcframework and copies it to godot/ios/dcl-swift-lib/

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SWIFT_LIB_DIR="$SCRIPT_DIR/dcl-swift-lib"

if [ ! -d "$SWIFT_LIB_DIR" ]; then
    echo "Error: dcl-swift-lib directory not found at $SWIFT_LIB_DIR"
    exit 1
fi

echo "Building DclSwiftLib..."
cd "$SWIFT_LIB_DIR"
make xcframework

echo ""
echo "Done! Framework installed at: godot/ios/dcl-swift-lib/DclSwiftLib.framework"
