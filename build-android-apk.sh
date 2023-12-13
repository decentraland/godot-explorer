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
ln -sf ${EXPLORER_PATH}/.bin/godot/templates/templates/ 4.2.stable

echo "Build for Android"
cd ${EXPLORER_PATH}/rust/decentraland-godot-lib
bash android-build.sh

echo "Setup Android Debug Keys"
cd /opt/
keytool -keyalg RSA -genkeypair -alias androiddebugkey \
    -keypass android -keystore debug.keystore -storepass android \
    -dname "CN=Android Debug,O=Android,C=US" -validity 9999 -deststoretype pkcs12

export GODOT_ANDROID_KEYSTORE_DEBUG_PATH=/opt/debug.keystore
export GODOT_ANDROID_KEYSTORE_DEBUG_USER=androiddebugkey
export GODOT_ANDROID_KEYSTORE_DEBUG_PASSWORD=android

echo "Export Godot APK"
cd ${EXPLORER_PATH}/godot/

${EXPLORER_PATH}/.bin/godot/godot4_bin \
    -e --headless --export-debug Android ${EXPLORER_PATH}/android.apk