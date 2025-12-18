#!/bin/bash
# Script to update the Godot iOS library in the template zip
#
# This copies the newly built libgodot.ios.template_*.arm64.a from the Godot engine
# build directory into the ios.zip export template.
#
# The script keeps an extracted copy of the template in .bin/ios_template/ for
# faster subsequent updates (avoids re-extracting 2GB+ zip each time).
#
# Usage:
#   ./scripts/update_godot_ios_lib.sh [debug|release]
#
# Prerequisites:
#   - Build Godot engine for iOS first:
#     cd /path/to/godotengine
#     scons platform=ios target=template_debug arch=arm64
#     scons platform=ios target=template_release arch=arm64

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# Configuration
GODOT_ENGINE_DIR="${GODOT_ENGINE_DIR:-/Users/leandro/github/godotengine}"
GODOT_VERSION="4.5.1"
TEMPLATES_DIR="$HOME/Library/Application Support/Godot/export_templates/${GODOT_VERSION}.stable"
CACHE_DIR="$PROJECT_DIR/.bin/ios_template"

# Parse arguments
BUILD_TYPE="${1:-debug}"
if [[ "$BUILD_TYPE" != "debug" && "$BUILD_TYPE" != "release" ]]; then
    echo "Usage: $0 [debug|release]"
    exit 1
fi

echo "🔄 Updating Godot iOS library ($BUILD_TYPE)..."
echo "   Godot Engine: $GODOT_ENGINE_DIR"
echo "   Templates: $TEMPLATES_DIR"

# Determine source file and target paths based on build type
SOURCE_LIB="$GODOT_ENGINE_DIR/bin/libgodot.ios.template_${BUILD_TYPE}.arm64.a"
TARGET_ZIP="$TEMPLATES_DIR/ios.zip"
BACKUP_ZIP="${TARGET_ZIP}.backup"

if [[ "$BUILD_TYPE" == "debug" ]]; then
    XCFRAMEWORK_PATH="libgodot.ios.debug.xcframework/ios-arm64/libgodot.a"
else
    XCFRAMEWORK_PATH="libgodot.ios.release.xcframework/ios-arm64/libgodot.a"
fi

# Check source file exists
if [[ ! -f "$SOURCE_LIB" ]]; then
    echo "❌ Source library not found: $SOURCE_LIB"
    echo ""
    echo "Build it first with:"
    echo "  cd $GODOT_ENGINE_DIR"
    echo "  scons platform=ios target=template_$BUILD_TYPE arch=arm64"
    exit 1
fi

# Check target zip exists (either original or backup)
if [[ ! -f "$TARGET_ZIP" && ! -f "$BACKUP_ZIP" ]]; then
    echo "❌ Target ios.zip not found: $TARGET_ZIP"
    echo "   Run 'cargo run -- install --targets ios' to download templates."
    exit 1
fi

# Determine if we need to extract the template
NEED_EXTRACT=false
if [[ ! -d "$CACHE_DIR" ]]; then
    NEED_EXTRACT=true
    echo "📁 Cache directory not found, will extract template..."
elif [[ -f "$BACKUP_ZIP" ]]; then
    # Check if backup is newer than cache (template was re-downloaded)
    BACKUP_TIME=$(stat -f%m "$BACKUP_ZIP" 2>/dev/null || stat -c%Y "$BACKUP_ZIP")
    CACHE_TIME=$(stat -f%m "$CACHE_DIR" 2>/dev/null || stat -c%Y "$CACHE_DIR")
    if [[ "$BACKUP_TIME" -gt "$CACHE_TIME" ]]; then
        NEED_EXTRACT=true
        echo "📁 Backup is newer than cache, will re-extract template..."
    fi
fi

# Extract template if needed
if [[ "$NEED_EXTRACT" == true ]]; then
    # Determine which zip to extract from
    EXTRACT_FROM="$TARGET_ZIP"
    if [[ -f "$BACKUP_ZIP" ]]; then
        EXTRACT_FROM="$BACKUP_ZIP"
    fi

    echo "📦 Extracting ios.zip to cache (this may take a while)..."
    rm -rf "$CACHE_DIR"
    mkdir -p "$CACHE_DIR"
    unzip -q "$EXTRACT_FROM" -d "$CACHE_DIR"

    # Create backup if it doesn't exist
    if [[ ! -f "$BACKUP_ZIP" && -f "$TARGET_ZIP" ]]; then
        echo "💾 Creating backup: $BACKUP_ZIP"
        cp "$TARGET_ZIP" "$BACKUP_ZIP"
    fi
else
    echo "📁 Using cached template from: $CACHE_DIR"
fi

# Replace the library
echo "📋 Copying new library..."
TARGET_LIB="$CACHE_DIR/$XCFRAMEWORK_PATH"
cp "$SOURCE_LIB" "$TARGET_LIB"

# Show size info
NEW_LIB_SIZE=$(stat -f%z "$SOURCE_LIB" 2>/dev/null || stat -c%s "$SOURCE_LIB")
echo "   New .a file: $(echo "$NEW_LIB_SIZE" | awk '{printf "%.1f MB", $1/1024/1024}')"

# Recreate zip
echo "📦 Recreating ios.zip..."
cd "$CACHE_DIR"
rm -f "$TARGET_ZIP"
zip -q -r "$TARGET_ZIP" .

# Verify
NEW_ZIP_SIZE=$(stat -f%z "$TARGET_ZIP" 2>/dev/null || stat -c%s "$TARGET_ZIP")
echo "   New ios.zip: $(echo "$NEW_ZIP_SIZE" | awk '{printf "%.1f GB", $1/1024/1024/1024}')"

echo ""
echo "✅ Successfully updated Godot iOS library!"
echo ""
echo "Next steps:"
echo "  1. Build your project: cargo run -- build --target ios"
echo "  2. Export IPA: cargo run -- export --target ios"
