#!/usr/bin/env bash
# Xcode "Embed Godot" build phase: copies the synced Godot artifacts (rn/ios/Godot, written by
# sync-godot-ios.sh) into the app bundle. Device builds only; the simulator always uses the mock bridge.
set -euo pipefail

GODOT_DIR="$SRCROOT/Godot"
if [ "$PLATFORM_NAME" != "iphoneos" ] || [ ! -f "$GODOT_DIR/godot.xcconfig" ]; then
  echo "Godot not linked for $PLATFORM_NAME: the app runs with the mock bridge"
  exit 0
fi

RESOURCES="$TARGET_BUILD_DIR/$UNLOCALIZED_RESOURCES_FOLDER_PATH"
FRAMEWORKS="$TARGET_BUILD_DIR/$FRAMEWORKS_FOLDER_PATH"
mkdir -p "$FRAMEWORKS"

# The engine looks for <executable>.pck next to the executable.
cp "$GODOT_DIR/project.pck" "$RESOURCES/$EXECUTABLE_NAME.pck"
cp "$GODOT_DIR/override.cfg" "$RESOURCES/"
if [ -f "$GODOT_DIR/godot_cmdline.txt" ]; then
  cp "$GODOT_DIR/godot_cmdline.txt" "$RESOURCES/"
fi

for framework in "$GODOT_DIR"/Frameworks/*.framework; do
  rsync -a --delete "$framework" "$FRAMEWORKS/"
  if [ "${CODE_SIGNING_ALLOWED:-NO}" = "YES" ] && [ -n "${EXPANDED_CODE_SIGN_IDENTITY:-}" ]; then
    codesign --force --sign "$EXPANDED_CODE_SIGN_IDENTITY" --timestamp=none \
      "$FRAMEWORKS/$(basename "$framework")"
  fi
done
