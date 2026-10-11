#!/usr/bin/env bash
# VIBE-CODED SPIKE: copy the Godot engine AAR, the Rust GDExtension and the exported project
# data into the RN Android app. Needs, from the repo root:
#   cargo run -- build -r --target android
#   cargo run -- export --target android --format apk --release
# GODOT_BUILD=debug syncs the debug template instead (after the matching debug build and export).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
APK="${GODOT_APK:-$ROOT/exports/decentraland.godot.client.apk}"
BUILD="${GODOT_BUILD:-release}"
GODOT_LIBS="$ROOT/godot/android/build/libs/$BUILD"
APP="$ROOT/rn/android/app"

[ -f "$APK" ] || { echo "missing $APK (run: cargo run -- export --target android --format apk)"; exit 1; }

rm -rf "$APP/libs" "$APP/src/main/jniLibs" "$APP/src/main/assets"
mkdir -p "$APP/libs" "$APP/src/main/jniLibs/arm64-v8a"

cp "$GODOT_LIBS/godot-lib.template_$BUILD.aar" "$APP/libs/"
cp "$GODOT_LIBS/arm64-v8a/deps/libwebrtc.jar" "$APP/libs/"

# Stripped native libs and the project data both come from the exported APK.
# libgodot_android.so is inside the AAR, so it is not copied here.
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
unzip -q "$APK" 'lib/arm64-v8a/libdclgodot.so' 'lib/arm64-v8a/libsentry*.so' \
  'lib/arm64-v8a/libc++_shared.so' -d "$TMP"
cp "$TMP"/lib/arm64-v8a/*.so "$APP/src/main/jniLibs/arm64-v8a/"
unzip -q "$APK" 'assets/*' -x 'assets/ios/*' -d "$APP/src/main"

echo "synced: $(du -sh "$APP/src/main/jniLibs" | cut -f1) jniLibs, $(du -sh "$APP/src/main/assets" | cut -f1) assets"
