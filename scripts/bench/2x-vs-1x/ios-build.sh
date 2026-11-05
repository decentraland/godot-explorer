#!/usr/bin/env bash
# Build the 2.x React Native iOS app from the CI Godot export (#3206).
#
# The R2 bucket publishes what CI exported (pck + xcodeproj + frameworks) but not a signed app, so
# this script picks that export up, links it into rn/ios/Godot, builds DclRn in Release and installs
# it on the paired iPhone with the team's development profile.
#
#   ios-build.sh /path/to/ios-2x-export.tar.gz /path/to/ios.zip
set -euxo pipefail

TARBALL=${1:?godot ios export tarball}
TEMPLATES=${2:?godot ios.zip from the export templates}
ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
EXPORT="$ROOT/exports"
TEAM=${DEVELOPMENT_TEAM:-8T73XM973P}

mkdir -p "$EXPORT"
tar xzf "$TARBALL" -C "$EXPORT"
test -f "$EXPORT/decentraland-godot-client.pck"

rm -rf /tmp/ios-templates && mkdir -p /tmp/ios-templates
unzip -q -o "$TEMPLATES" -d /tmp/ios-templates
HOST_LIB=$(find /tmp/ios-templates -path '*ios-arm64/libgodot.a' | head -1)
test -n "$HOST_LIB"

cd "$ROOT/rn"
[ -d node_modules ] || npm ci
GODOT_IOS_EXPORT="$EXPORT" GODOT_HOST_LIB="$HOST_LIB" bash scripts/sync-godot-ios.sh

cd ios
if ! command -v pod >/dev/null 2>&1; then
  export GEM_HOME="$HOME/.gem"
  gem install cocoapods --no-document
  export PATH="$GEM_HOME/bin:$PATH"
fi
pod install

xcodebuild build \
  -workspace DclRn.xcworkspace -scheme DclRn -configuration Release \
  -destination 'generic/platform=iOS' \
  -derivedDataPath "$ROOT/.bench-tmp/ios-build" \
  -allowProvisioningUpdates \
  CODE_SIGN_STYLE=Automatic \
  DEVELOPMENT_TEAM="$TEAM" \
  | tee /tmp/ios-xcodebuild.log | tail -40

APP=$(find "$ROOT/.bench-tmp/ios-build/Build/Products/Release-iphoneos" -maxdepth 1 -name '*.app' | head -1)
echo "APP=$APP"
