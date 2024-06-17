#!/bin/bash

set +e
set +o pipefail

EXPLORER_PATH=$(pwd)
if [ ! -d ${EXPLORER_PATH}/godot/android/ ]
then
    echo "Checkout godot android template"
    git clone https://github.com/decentraland/godot-explorer-android-template ${EXPLORER_PATH}/godot/android
fi

echo "Build for Linux x86_64"
cd ${EXPLORER_PATH}/rust/xtask
cargo run -- install
cargo run -- run --only-build

echo "Link export templates"
mkdir -p ${HOME}/.local/share/godot/export_templates/
cd ${HOME}/.local/share/godot/export_templates/
ln -sf ${EXPLORER_PATH}/.bin/godot/templates/templates/ 4.2.1.stable

set -e 

echo "Build for Android (arm64)"
cd ${EXPLORER_PATH}/rust/decentraland-godot-lib
bash android-build.sh

echo "Build for Android (x86_64)"
cd ${EXPLORER_PATH}/rust/decentraland-godot-lib
bash android-build.sh x86_64

set +e

echo "Setup Android Debug Keys"
cd /opt/
keytool -keyalg RSA -genkeypair -alias androiddebugkey \
    -keypass android -keystore debug.keystore -storepass android \
    -dname "CN=Android Debug,O=Android,C=US" -validity 9999 -deststoretype pkcs12

export GODOT_ANDROID_KEYSTORE_DEBUG_PATH=/opt/debug.keystore
export GODOT_ANDROID_KEYSTORE_DEBUG_USER=androiddebugkey
export GODOT_ANDROID_KEYSTORE_DEBUG_PASSWORD=android

cd ${EXPLORER_PATH}/godot/

# Build the .aab without x86_64 architecture
echo "Export Godot android.apk"
${EXPLORER_PATH}/.bin/godot/godot4_bin -e --headless --export-debug Android ${EXPLORER_PATH}/android.apk || true


# Build the .aab without x86_64 architecture
echo "Setting up to export godot .aab"
# Use aab
sed -i 's/gradle_build\/export_format=0/gradle_build\/export_format=1/' ${EXPLORER_PATH}/godot/export_presets.cfg
# remove x86_64
sed -i 's/architectures\/x86_64=true/architectures\/x86_64=false/' ${EXPLORER_PATH}/godot/export_presets.cfg
# remove signed
sed -i 's/package\/signed=true/package\/signed=false/' ${EXPLORER_PATH}/godot/export_presets.cfg

# Build the .aab without x86_64 architecture
echo "Export Godot AAB"
${EXPLORER_PATH}/.bin/godot/godot4_bin -e --headless --export-release Android ${EXPLORER_PATH}/android-unsigned.aab || true

echo "Finished"