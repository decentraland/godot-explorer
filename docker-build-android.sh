#!/bin/bash
if [ ! -d /app/godot/android/ ]
then
    echo "Checkout godot android"
    git clone https://github.com/decentraland/godot-explorer-android-template /app/godot/android
fi

echo "Build for Linux x86_64"
cd /app/rust/xtask
cargo run -- install
cargo run -- run --only-build

echo "Copy export templates"
mkdir -p /root/.local/share/godot/export_templates/4.2.stable/


echo "Build for Android"
cd /app/rust/decentraland-godot-lib
./android-build.sh

echo "Setup Android Debug Keys"
cd /opt/
keytool -keyalg RSA -genkeypair -alias androiddebugkey \
    -keypass android -keystore debug.keystore -storepass android \
    -dname "CN=Android Debug,O=Android,C=US" -validity 9999 -deststoretype pkcs12

export GODOT_ANDROID_KEYSTORE_DEBUG_PATH=/opt/debug.keystore
export GODOT_ANDROID_KEYSTORE_DEBUG_USER=androiddebugkey
export GODOT_ANDROID_KEYSTORE_DEBUG_PASSWORD=android

echo "Export Godot APK"
cd /app/godot/
/app/.bin/godot/godot4_bin \
    -e --headless --rendering-driver opengl3 --headless \
    --export-debug Android /app/android.apk

ls -la | grep android.apk