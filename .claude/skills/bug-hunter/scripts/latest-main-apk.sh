#!/usr/bin/env bash
# Finds the newest commit on a branch whose Android CI build is in R2 and optionally installs it.
#   latest-main-apk.sh [--branch main] [--install] [--out DIR]
# Prints: <sha> <build number> <url>. `--install` runs `adb install -r`: CI builds share one signing
# key, so the update keeps the app's data (session, settings, cache). A local build has a different
# key and needs an uninstall first, which wipes the session: never do that without asking.
set -euo pipefail

BRANCH=main
INSTALL=0
OUT="${TMPDIR:-/tmp}/bug-hunter-apk"
while [ $# -gt 0 ]; do
  case "$1" in
    --branch) BRANCH="$2"; shift 2 ;;
    --install) INSTALL=1; shift ;;
    --out) OUT="$2"; shift 2 ;;
    *) echo "unknown arg $1" >&2; exit 2 ;;
  esac
done

BASE="https://mobile-artifacts.dclregenesislabs.xyz/android/$(echo "$BRANCH" | tr '/' '-')"
git fetch -q origin "$BRANCH"
for sha in $(git log --format=%H -15 "origin/$BRANCH"); do
  url="$BASE/$sha/decentraland.godot.client.apk"
  code=$(curl -s -o /dev/null -w "%{http_code}" -I "$url")
  if [ "$code" = "200" ]; then
    build=$(curl -s "$BASE/$sha/build_number.txt")
    echo "$sha $build $url"
    if [ "$INSTALL" = "1" ]; then
      mkdir -p "$OUT"
      curl -s -o "$OUT/app.apk" "$url"
      adb install -r "$OUT/app.apk"
      adb shell dumpsys package org.decentraland.godotexplorer | grep -E "versionCode|versionName" | head -2
    fi
    exit 0
  fi
  echo "no APK yet for ${sha:0:10} (CI still building or skipped)" >&2
done
echo "no APK found in the last 15 commits of $BRANCH" >&2
exit 1
