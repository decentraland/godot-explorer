#!/usr/bin/env bash
# Copy the Godot engine AAR, the Rust GDExtension and the exported project data into
# rn/android/app/godot. Without that folder the app builds with the JS mock bridge. Needs, from the repo root:
#   cargo run -- build -r --target android
#   cargo run -- export --target android --format apk --release
# GODOT_BUILD=debug syncs the debug template instead (after the matching debug build and export).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
APK="${GODOT_APK:-$ROOT/exports/decentraland.godot.client.apk}"
BUILD="${GODOT_BUILD:-release}"
GODOT_LIBS="$ROOT/godot/android/build/libs/$BUILD"
DEST="$ROOT/rn/android/app/godot"

[ -f "$APK" ] || { echo "missing $APK (run: cargo run -- export --target android --format apk)"; exit 1; }

rm -rf "$DEST"
mkdir -p "$DEST/libs" "$DEST/jniLibs/arm64-v8a"

cp "$GODOT_LIBS/godot-lib.template_$BUILD.aar" "$DEST/libs/"
cp "$GODOT_LIBS/arm64-v8a/deps/libwebrtc.jar" "$DEST/libs/"
# The Sentry GDExtension needs its Android plugin (singleton SentryAndroidGodotPlugin) or it crashes.
cp "$ROOT/godot/addons/sentry/bin/android/sentry_android_godot_plugin.$BUILD.aar" "$DEST/libs/"
# DclAndroidPlugin (push, deeplinks, ExoPlayer, Reown, ...); its Maven deps are in app/godot.gradle.
cp "$ROOT/godot/addons/dcl-godot-android/bin/$BUILD/dcl-godot-android-$BUILD.aar" "$DEST/libs/"

# Stripped native libs and the project data both come from the exported APK.
# libgodot_android.so is inside the AAR, so it is not copied here.
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
unzip -q "$APK" 'lib/arm64-v8a/libdclgodot.so' 'lib/arm64-v8a/libsentry*.so' \
  'lib/arm64-v8a/libc++_shared.so' -d "$TMP"
cp "$TMP"/lib/arm64-v8a/*.so "$DEST/jniLibs/arm64-v8a/"
# Prefer the unstripped copies Godot's Gradle build keeps: AGP strips them for the APK and puts
# their symbols in the AAB (Play Console symbolication), as the engine's own template does.
case "$BUILD" in
  release) VARIANT=StandardRelease ;;
  *) VARIANT=StandardDebug ;;
esac
UNSTRIPPED="$ROOT/godot/android/build/build/intermediates/merged_native_libs/standard${VARIANT#Standard}/merge${VARIANT}NativeLibs/out/lib/arm64-v8a"
if [ -d "$UNSTRIPPED" ]; then
  for lib in "$DEST"/jniLibs/arm64-v8a/*.so; do
    if [ -f "$UNSTRIPPED/$(basename "$lib")" ]; then
      cp "$UNSTRIPPED/$(basename "$lib")" "$lib"
    fi
  done
fi
# dexopt/ is the exported app's own baseline profile; AGP adds the RN app's, so skip it.
unzip -q "$APK" 'assets/*' -x 'assets/ios/*' 'assets/dexopt/*' -d "$DEST"

# Host-only: a translucent Godot surface lets the avatar preview draw above the RN UI.
printf '[display]\n\nwindow/per_pixel_transparency/allowed=true\n' > "$DEST/assets/override.cfg"

# unzip keeps the archive's 1981 timestamps and assets.sparsepck keeps its size across exports, so
# Gradle would reuse a stale index (truncated or garbage scripts at runtime). Fresh mtimes force it.
find "$DEST/assets" -type f -exec touch {} +

echo "synced: $(du -sh "$DEST/jniLibs" | cut -f1) jniLibs, $(du -sh "$DEST/assets" | cut -f1) assets"
