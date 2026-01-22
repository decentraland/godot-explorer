#!/bin/bash
set -e

# Build script for DclSwiftLib iOS GDExtension
# Builds SwiftGodot + Reown SDK (WalletConnect) xcframework
#
# Usage:
#   ./build_ios_swift.sh          # Full build
#   ./build_ios_swift.sh --clean  # Clean and rebuild

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "${SCRIPT_DIR}"

CLEAN_BUILD=false
if [ "$1" == "--clean" ]; then
    CLEAN_BUILD=true
fi

echo "========================================"
echo "Building DclSwiftLib (SwiftGodot + WalletConnect)"
echo "========================================"

# Clean if requested
if [ "$CLEAN_BUILD" == true ]; then
    echo ""
    echo "Cleaning previous build..."
    make clean
fi

# Build xcframework
echo ""
echo "Building xcframework for iOS device + simulator..."
make xcframework

# Verify output
if [ -d "bin/DclSwiftLib.xcframework" ]; then
    echo ""
    echo "========================================"
    echo "Build completed successfully!"
    echo "========================================"
    echo ""
    echo "Output: bin/DclSwiftLib.xcframework"
    echo ""
    echo "Contents:"
    ls -la bin/DclSwiftLib.xcframework/
    echo ""

    # Show framework size
    DEVICE_SIZE=$(du -sh bin/DclSwiftLib.xcframework/ios-arm64/DclSwiftLib.framework/DclSwiftLib 2>/dev/null | cut -f1)
    echo "Device framework size: ${DEVICE_SIZE}"
    echo ""
    echo "The GDExtension is ready for iOS export."
    echo "Run: cargo run -- export --target ios"
else
    echo ""
    echo "ERROR: xcframework not found after build"
    exit 1
fi
