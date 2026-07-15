#!/usr/bin/env bash
# build_for_profile.sh — Android build optimized for simpleperf profiling.
#
# Combines:
#   - libdclgodot.so   : Rust `dev-release` profile (optimized, with debug info)
#   - libgodot_android.so : Godot DEBUG export template (unstripped + GNU
#                         build-id present, ~764 MB)
#
# Bypasses the default `cargo run -- export ... --release` path, which uses
# the Godot RELEASE template — stripped + no build-id, so simpleperf can't
# resolve symbols inside libgodot_android.so. With this script the resulting
# APK is symbol-rich; tradeoff is slower runtime, so use only for profiling
# sessions, not for FPS / draw-call A/B benchmarks.
#
# Usage:
#   scripts/bench/build_for_profile.sh
#   scripts/bench/launch_devices.sh --android --gp-benchmark \
#       --param bench-tag=profile --pull-results &
#   scripts/bench/profile_android.sh profile
#
# Reads $ANDROID_SERIAL from scripts/bench/.env if present.

set -euo pipefail

ENV_FILE="$(dirname "$0")/.env"
[[ -f "$ENV_FILE" ]] && set -a && source "$ENV_FILE" && set +a

cd "$(git rev-parse --show-toplevel)"

echo "[build_for_profile] Step 1/3: Rust dev-release for Android (libdclgodot.so + symbols)"
cargo run -- build --target android --release

echo "[build_for_profile] Step 2/3: Godot DEBUG export (libgodot_android.so unstripped)"
# Note: NO --release flag → xtask picks --export-debug → Godot uses the debug
# template instead of the release one. The Rust .so was already built
# dev-release in step 1; copy_library + Godot's packager wire them together.
cargo run -- export --target android --format apk

ANDROID_SERIAL="${ANDROID_SERIAL:-}"
if [[ -n "$ANDROID_SERIAL" ]]; then
  echo "[build_for_profile] Step 3/3: install on $ANDROID_SERIAL"
  adb -s "$ANDROID_SERIAL" shell "am force-stop org.decentraland.godotexplorer" >/dev/null || true
  adb -s "$ANDROID_SERIAL" install -r exports/decentraland.godot.client.apk
else
  echo "[build_for_profile] Step 3/3: skipped (set ANDROID_SERIAL in .env to auto-install)"
  echo "  Manual: adb install -r exports/decentraland.godot.client.apk"
fi

echo "[build_for_profile] done."
