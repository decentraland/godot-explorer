#!/bin/bash
set -euo pipefail

# Set the explorer path to the current working directory.
EXPLORER_PATH="$(pwd)"

# Unzip Godot Android template if it does not exist.
if [ ! -d "${EXPLORER_PATH}/godot/android/" ]; then
    echo "Unzip out Godot Android template..."
    unzip "${HOME}/.local/share/godot/export_templates/4.4.1.stable/android_source.zip" \
        -d "${EXPLORER_PATH}/godot/android"
fi

# Set JAVA_HOME if not already set.
if [ -z "${JAVA_HOME:-}" ]; then
    export JAVA_HOME="/usr/lib/jvm/java-17-openjdk-amd64"
fi

echo "Build for Linux x86_64"
cd "${EXPLORER_PATH}"

cargo run -- build

echo "Build for Android (arm64)"
cd "${EXPLORER_PATH}/lib"
bash android-build.sh

echo "Build for Android (x86_64)"
cd "${EXPLORER_PATH}/lib"
bash android-build.sh x86_64

# Temporarily disable strict error checking for the debug key setup.
set +e
echo "Setup Android Debug Keys"
cd /opt/ || exit 1
keytool -keyalg RSA -genkeypair -alias androiddebugkey \
    -keypass android -keystore debug.keystore -storepass android \
    -dname "CN=Android Debug,O=Android,C=US" -validity 9999 -deststoretype pkcs12

export GODOT_ANDROID_KEYSTORE_DEBUG_PATH="/opt/debug.keystore"
export GODOT_ANDROID_KEYSTORE_DEBUG_USER="androiddebugkey"
export GODOT_ANDROID_KEYSTORE_DEBUG_PASSWORD="android"
# Re-enable strict error checking.
set -e

cd "${EXPLORER_PATH}/godot/"

echo "Export Godot android.apk"
# Define the export command.
COMMAND="${EXPLORER_PATH}/.bin/godot/godot4_bin -e --headless --export-debug android ${EXPLORER_PATH}/android.apk"
if ! eval "$COMMAND"; then
    echo "First attempt failed, retrying in 5 seconds..."
    sleep 5
    if ! eval "$COMMAND"; then
        echo "Second attempt failed."
    else
        echo "Second attempt succeeded."
    fi
else
    echo "First attempt succeeded."
fi

# Preserve the original behavior with "|| true".
${EXPLORER_PATH}/.bin/godot/godot4_bin -e --headless --export-debug quest "${EXPLORER_PATH}/meta-quest.apk" || true

echo "Setting up to export godot .aab"
# Update export presets.
sed -i 's/gradle_build\/export_format=0/gradle_build\/export_format=1/' "${EXPLORER_PATH}/godot/export_presets.cfg"
sed -i 's/architectures\/x86_64=true/architectures\/x86_64=false/' "${EXPLORER_PATH}/godot/export_presets.cfg"
sed -i 's/package\/signed=true/package\/signed=false/' "${EXPLORER_PATH}/godot/export_presets.cfg"

echo "Export Godot AAB"
${EXPLORER_PATH}/.bin/godot/godot4_bin -e --headless --export-release android "${EXPLORER_PATH}/android-unsigned.aab" || true
${EXPLORER_PATH}/.bin/godot/godot4_bin -e --headless --export-release quest "${EXPLORER_PATH}/meta-quest-unsigned.aab" || true

echo "Finished."
