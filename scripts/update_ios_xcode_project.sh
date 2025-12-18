#!/bin/bash
# Script to update iOS Xcode project with latest builds
#
# This directly copies built libraries to the exported Xcode project,
# avoiding the slow ios.zip template update process.
#
# Usage:
#   ./scripts/update_ios_xcode_project.sh [--godot] [--plugin] [--pck] [--all]
#
# Options:
#   --godot   Update Godot engine library
#   --plugin  Update dcl-godot-ios plugin
#   --pck     Re-export and update the PCK file (GDScript/assets)
#   --all     Update all (default if no options given)
#
# Prerequisites:
#   - Export Xcode project first: cargo run -- export --target ios
#   - Build Godot engine: cd /path/to/godotengine && scons platform=ios target=template_debug arch=arm64
#   - Build iOS plugin: cd plugins/dcl-godot-ios && ./scripts/build.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# Configuration
GODOT_ENGINE_DIR="${GODOT_ENGINE_DIR:-/Users/leandro/github/godotengine}"
XCODE_PROJECT="$PROJECT_DIR/exports"
GODOT_PROJECT="$PROJECT_DIR/godot"

# Source files
GODOT_LIB_SOURCE="$GODOT_ENGINE_DIR/bin/libgodot.ios.template_debug.arm64.a"
PLUGIN_SOURCE="$PROJECT_DIR/plugins/dcl-godot-ios/bin/dcl_godot_ios-device.release_debug.a"
PCK_FILE="$XCODE_PROJECT/Decentraland.pck"

# Target files in Xcode project
GODOT_LIB_TARGET="$XCODE_PROJECT/Decentraland.xcframework/ios-arm64/libgodot.a"
PLUGIN_TARGET="$XCODE_PROJECT/Decentraland/dylibs/ios/plugins/dcl_godot_ios/dcl_godot_ios.xcframework/ios-arm64/dcl_godot_ios-device.release_debug.a"

# Detect Godot executable
detect_godot() {
    # Check for Godot in common locations
    if command -v godot &> /dev/null; then
        echo "godot"
    elif [[ -f "$PROJECT_DIR/.bin/godot/Godot.app/Contents/MacOS/Godot" ]]; then
        echo "$PROJECT_DIR/.bin/godot/Godot.app/Contents/MacOS/Godot"
    elif [[ -f "/Applications/Godot.app/Contents/MacOS/Godot" ]]; then
        echo "/Applications/Godot.app/Contents/MacOS/Godot"
    else
        echo ""
    fi
}

# Parse arguments
UPDATE_GODOT=false
UPDATE_PLUGIN=false
UPDATE_PCK=false

if [[ $# -eq 0 ]] || [[ "$1" == "--all" ]]; then
    UPDATE_GODOT=true
    UPDATE_PLUGIN=true
    UPDATE_PCK=true
else
    while [[ $# -gt 0 ]]; do
        case $1 in
            --godot)
                UPDATE_GODOT=true
                shift
                ;;
            --plugin)
                UPDATE_PLUGIN=true
                shift
                ;;
            --pck)
                UPDATE_PCK=true
                shift
                ;;
            --all)
                UPDATE_GODOT=true
                UPDATE_PLUGIN=true
                UPDATE_PCK=true
                shift
                ;;
            *)
                echo "Unknown option: $1"
                echo "Usage: $0 [--godot] [--plugin] [--pck] [--all]"
                exit 1
                ;;
        esac
    done
fi

# Check Xcode project exists
if [[ ! -d "$XCODE_PROJECT/Decentraland.xcodeproj" ]]; then
    echo "❌ Xcode project not found at: $XCODE_PROJECT"
    echo "   Export it first with: cargo run -- export --target ios"
    exit 1
fi

echo "🔄 Updating iOS Xcode project..."

# Update Godot library
if [[ "$UPDATE_GODOT" == true ]]; then
    if [[ ! -f "$GODOT_LIB_SOURCE" ]]; then
        echo "❌ Godot library not found: $GODOT_LIB_SOURCE"
        echo "   Build it first with:"
        echo "     cd $GODOT_ENGINE_DIR"
        echo "     scons platform=ios target=template_debug arch=arm64"
    else
        echo "📋 Updating Godot engine library..."
        cp "$GODOT_LIB_SOURCE" "$GODOT_LIB_TARGET"

        SOURCE_SIZE=$(stat -f%z "$GODOT_LIB_SOURCE" 2>/dev/null || stat -c%s "$GODOT_LIB_SOURCE")
        echo "   ✅ libgodot.a ($(echo "$SOURCE_SIZE" | awk '{printf "%.1f MB", $1/1024/1024}'))"
    fi
fi

# Update plugin
if [[ "$UPDATE_PLUGIN" == true ]]; then
    if [[ ! -f "$PLUGIN_SOURCE" ]]; then
        echo "❌ Plugin library not found: $PLUGIN_SOURCE"
        echo "   Build it first with:"
        echo "     cd $PROJECT_DIR/plugins/dcl-godot-ios"
        echo "     ./scripts/build.sh"
    else
        echo "📋 Updating dcl-godot-ios plugin..."
        cp "$PLUGIN_SOURCE" "$PLUGIN_TARGET"

        SOURCE_SIZE=$(stat -f%z "$PLUGIN_SOURCE" 2>/dev/null || stat -c%s "$PLUGIN_SOURCE")
        echo "   ✅ dcl_godot_ios plugin ($(echo "$SOURCE_SIZE" | awk '{printf "%.1f KB", $1/1024}'))"
    fi
fi

# Update PCK
if [[ "$UPDATE_PCK" == true ]]; then
    GODOT_BIN=$(detect_godot)
    if [[ -z "$GODOT_BIN" ]]; then
        echo "❌ Godot executable not found"
        echo "   Install Godot with: cargo run -- install"
    else
        echo "📋 Re-exporting PCK file..."

        # Remove old PCK
        rm -f "$PCK_FILE"

        # Export just the PCK using --export-pack
        cd "$GODOT_PROJECT"
        "$GODOT_BIN" --headless --export-pack "ios" "$PCK_FILE" 2>/dev/null || true
        cd "$PROJECT_DIR"

        if [[ -f "$PCK_FILE" ]]; then
            PCK_SIZE=$(stat -f%z "$PCK_FILE" 2>/dev/null || stat -c%s "$PCK_FILE")
            echo "   ✅ Decentraland.pck ($(echo "$PCK_SIZE" | awk '{printf "%.1f MB", $1/1024/1024}'))"
        else
            echo "   ⚠️  PCK export may have failed, trying full export..."
            # Fallback: use cargo run which handles imports properly
            cargo run -- export --target ios 2>/dev/null || true
            if [[ -f "$PCK_FILE" ]]; then
                PCK_SIZE=$(stat -f%z "$PCK_FILE" 2>/dev/null || stat -c%s "$PCK_FILE")
                echo "   ✅ Decentraland.pck ($(echo "$PCK_SIZE" | awk '{printf "%.1f MB", $1/1024/1024}'))"
            else
                echo "   ❌ Failed to generate PCK"
            fi
        fi
    fi
fi

echo ""
echo "✅ Xcode project updated!"
echo ""
echo "Next steps:"
echo "  1. Open Xcode: open $XCODE_PROJECT/Decentraland.xcodeproj"
echo "  2. Build and run on device (Cmd+R)"
