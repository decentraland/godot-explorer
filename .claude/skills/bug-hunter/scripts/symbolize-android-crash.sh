#!/usr/bin/env bash
# Symbolizes the libdclgodot.so frames of the newest native crash on the phone.
#   symbolize-android-crash.sh <short-or-full sha of the installed build> [tombstone.txt]
# The sha is in the app's version string (Settings, or the in-world FPS label):
# 2.0.0.<build>-<sha>-staging. Without a file, the crash comes from `adb logcat -b crash`.
# The library on the phone is stripped; CI keeps the unstripped one as the run artifact
# `libdclgodot-android-release`. The script checks the Build ID matches before symbolizing.
set -euo pipefail
SHA_IN="${1:?usage: $0 <sha> [tombstone.txt]}"
WORK="${TMPDIR:-/tmp}/bug-hunter-sym"
mkdir -p "$WORK"

if [ -n "${2:-}" ]; then cp "$2" "$WORK/crash.txt"; else adb logcat -b crash -d > "$WORK/crash.txt"; fi
grep -E "Abort message|Fatal signal|signal [0-9]+ \(" "$WORK/crash.txt" | tail -3 || true

frames=$(grep -E "#[0-9]+ pc [0-9a-f]+ +.*libdclgodot\.so" "$WORK/crash.txt" | tail -40 || true)
[ -z "$frames" ] && { echo "no libdclgodot.so frames in the newest crash" >&2; exit 1; }
want_id=$(echo "$frames" | grep -o "BuildId: [0-9a-f]*" | tail -1 | awk '{print $2}' || true)

SHA=$(git rev-parse "$SHA_IN")
LIB=$(find "$WORK/$SHA" -name libdclgodot.so 2>/dev/null | head -1 || true)
if [ -z "$LIB" ]; then
  for run in $(gh api "repos/decentraland/godot-explorer/actions/runs?head_sha=$SHA&per_page=30" \
      --jq '.workflow_runs[].id'); do
    if gh api "repos/decentraland/godot-explorer/actions/runs/$run/artifacts" \
        --jq '.artifacts[].name' | grep -qx libdclgodot-android-release; then
      gh run download "$run" -n libdclgodot-android-release -D "$WORK/$SHA"
      break
    fi
  done
  LIB=$(find "$WORK/$SHA" -name libdclgodot.so 2>/dev/null | head -1 || true)
fi
[ -z "$LIB" ] && { echo "no libdclgodot-android-release artifact for $SHA (expired, or no Android build)" >&2; exit 1; }

SYM=$(ls -d "${ANDROID_HOME:-$HOME/Library/Android/sdk}"/ndk/*/toolchains/llvm/prebuilt/*/bin/llvm-symbolizer | tail -1)
READELF="$(dirname "$SYM")/llvm-readelf"
have_id=$("$READELF" -n "$LIB" | awk '/Build ID/{print $3}')
if [ -n "$want_id" ] && [ "$have_id" != "$want_id" ]; then
  echo "Build ID mismatch: crash $want_id, artifact $have_id — wrong sha?" >&2; exit 1
fi
echo "$frames" | while read -r line; do
  pc=$(echo "$line" | grep -o "pc [0-9a-f]*" | awk '{print $2}')
  echo "== $(echo "$line" | grep -o '#[0-9]*') 0x$pc"
  # Inlined frames come first; keep the outermost ~12 so the line stays readable.
  "$SYM" --obj="$LIB" -f -C -i "0x$pc" | paste - - | tail -12 | sed 's/^/   /'
done
