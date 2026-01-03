#!/bin/bash

echo "Testing dSYM generation for iOS (Release configuration)..."

# Check if exports folder exists
XCODE_PROJECT=$(find exports -name "*.xcodeproj" -type d 2>/dev/null | head -1)

if [ -z "$XCODE_PROJECT" ]; then
  echo "Error: Xcode project not found in exports/"
  echo "Please run: cargo run -- export --target ios"
  exit 1
fi

echo "Found Xcode project: $XCODE_PROJECT"

# Create build directory
mkdir -p build/ios

# Build the Xcode project to generate dSYMs with Release configuration
echo "Building Xcode project with Release configuration and dSYM generation..."
echo "Note: This may fail with code signing/entitlement errors, but dSYMs should still be generated"
echo ""

# Don't exit on error for this command
set +e
xcodebuild build \
  -project "$XCODE_PROJECT" \
  -scheme decentraland-godot-client \
  -configuration Release \
  -derivedDataPath "build/ios/DerivedData" \
  -sdk iphoneos \
  DEBUG_INFORMATION_FORMAT=dwarf-with-dsym \
  ONLY_ACTIVE_ARCH=NO \
  CODE_SIGNING_ALLOWED=NO \
  CODE_SIGNING_REQUIRED=NO \
  ENABLE_BITCODE=NO

BUILD_EXIT_CODE=$?
set -e

if [ $BUILD_EXIT_CODE -ne 0 ]; then
  echo ""
  echo "⚠️  Build exited with code $BUILD_EXIT_CODE (likely due to entitlements)"
  echo "Checking if dSYMs were still generated..."
fi

# Find dSYMs
echo ""
echo "Searching for dSYM files..."
DSYM_FILES=$(find "build/ios/DerivedData" -name "*.dSYM" -type d)

if [ -z "$DSYM_FILES" ]; then
  echo "❌ No dSYM files found!"
  exit 1
else
  echo "✅ dSYM files found:"
  echo "$DSYM_FILES"
  echo ""

  # Show details of each dSYM
  for dsym in $DSYM_FILES; do
    echo "Details of $dsym:"
    ls -lh "$dsym"

    # Check if it contains DWARF debug info
    DWARF_FILE=$(find "$dsym" -name "*.app.dSYM" -o -name "decentraland-godot-client")
    if [ -n "$DWARF_FILE" ]; then
      echo "DWARF debug info size:"
      du -sh "$dsym"
    fi
    echo ""
  done

  echo "✅ dSYM generation test successful!"
fi