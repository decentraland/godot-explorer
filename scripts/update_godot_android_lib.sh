#!/bin/bash
# Script to update the Godot Android library in the template AAR
#
# This copies the newly built libgodot.android.template_*.so from the Godot engine
# build directory into the godot-lib.template_*.aar used by the Android build.
#
# Usage:
#   ./scripts/update_godot_android_lib.sh [debug|release]
#
# Prerequisites:
#   - Build Godot engine for Android first:
#     cd /path/to/godotengine
#     scons platform=android target=template_debug arch=arm64
#     scons platform=android target=template_release arch=arm64

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# Configuration
GODOT_ENGINE_DIR="${GODOT_ENGINE_DIR:-/Users/leandro/github/godotengine}"
GODOT_ANDROID_DIR="$PROJECT_DIR/godot/android/build/libs"

# Parse arguments
BUILD_TYPE="${1:-debug}"
if [[ "$BUILD_TYPE" != "debug" && "$BUILD_TYPE" != "release" ]]; then
    echo "Usage: $0 [debug|release]"
    exit 1
fi

echo "🔄 Updating Godot Android library ($BUILD_TYPE)..."
echo "   Godot Engine: $GODOT_ENGINE_DIR"
echo "   Project: $PROJECT_DIR"

# Determine source file name based on build type
# Scons moves the .so to the android platform lib directory automatically
if [[ "$BUILD_TYPE" == "debug" ]]; then
    SOURCE_SO="$GODOT_ENGINE_DIR/platform/android/java/lib/libs/debug/arm64-v8a/libgodot_android.so"
    TARGET_AAR="$GODOT_ANDROID_DIR/debug/godot-lib.template_debug.aar"
else
    SOURCE_SO="$GODOT_ENGINE_DIR/platform/android/java/lib/libs/release/arm64-v8a/libgodot_android.so"
    TARGET_AAR="$GODOT_ANDROID_DIR/release/godot-lib.template_release.aar"
fi

# Check source file exists
if [[ ! -f "$SOURCE_SO" ]]; then
    echo "❌ Source library not found: $SOURCE_SO"
    echo ""
    echo "Build it first with:"
    echo "  cd $GODOT_ENGINE_DIR"
    echo "  scons platform=android target=template_$BUILD_TYPE arch=arm64"
    exit 1
fi

# Check target AAR exists
if [[ ! -f "$TARGET_AAR" ]]; then
    echo "❌ Target AAR not found: $TARGET_AAR"
    echo "   Run 'cargo run -- install --targets android' to download templates."
    exit 1
fi

# Create temp directory
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

echo "📦 Extracting AAR..."
unzip -q "$TARGET_AAR" -d "$TEMP_DIR"

# Backup original
BACKUP_AAR="${TARGET_AAR}.backup"
if [[ ! -f "$BACKUP_AAR" ]]; then
    echo "💾 Creating backup: $BACKUP_AAR"
    cp "$TARGET_AAR" "$BACKUP_AAR"
fi

# Replace the library
echo "📋 Copying new library..."
TARGET_SO="$TEMP_DIR/jni/arm64-v8a/libgodot_android.so"
cp "$SOURCE_SO" "$TARGET_SO"

# Show size comparison
ORIGINAL_SIZE=$(stat -f%z "$BACKUP_AAR" 2>/dev/null || stat -c%s "$BACKUP_AAR")
NEW_SO_SIZE=$(stat -f%z "$SOURCE_SO" 2>/dev/null || stat -c%s "$SOURCE_SO")
echo "   Original AAR: $(numfmt --to=iec-i --suffix=B $ORIGINAL_SIZE 2>/dev/null || echo "$ORIGINAL_SIZE bytes")"
echo "   New .so file: $(numfmt --to=iec-i --suffix=B $NEW_SO_SIZE 2>/dev/null || echo "$NEW_SO_SIZE bytes")"

# Recreate AAR
echo "📦 Recreating AAR..."
cd "$TEMP_DIR"
rm -f "$TARGET_AAR"
zip -q -r "$TARGET_AAR" .

# Verify
NEW_AAR_SIZE=$(stat -f%z "$TARGET_AAR" 2>/dev/null || stat -c%s "$TARGET_AAR")
echo "   New AAR: $(numfmt --to=iec-i --suffix=B $NEW_AAR_SIZE 2>/dev/null || echo "$NEW_AAR_SIZE bytes")"

echo ""
echo "✅ Successfully updated Godot Android library!"
echo ""
echo "Next steps:"
echo "  1. Build your project: cargo run -- build --target android"
echo "  2. Export APK: cargo run -- export --target android --format apk"
