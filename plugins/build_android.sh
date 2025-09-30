#!/bin/bash

# Build script for DCL Godot Android Plugin
# This script builds the Android plugin and copies necessary files to the Godot project

set -e  # Exit on error

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
ANDROID_PLUGIN_DIR="${SCRIPT_DIR}/dcl-godot-android"
GODOT_ADDONS_DIR="${SCRIPT_DIR}/../godot/addons"
PLUGIN_DEMO_ADDONS="${ANDROID_PLUGIN_DIR}/plugin/demo/addons/dcl-godot-android"

echo "🔨 Building DCL Godot Android Plugin..."
echo "================================================"

# Check for Android SDK
if [ -z "${ANDROID_HOME}" ]; then
    # Try common Android SDK locations
    if [ -d "${HOME}/Library/Android/sdk" ]; then
        export ANDROID_HOME="${HOME}/Library/Android/sdk"
        echo "📱 Found Android SDK at: ${ANDROID_HOME}"
    elif [ -d "${HOME}/Android/Sdk" ]; then
        export ANDROID_HOME="${HOME}/Android/Sdk"
        echo "📱 Found Android SDK at: ${ANDROID_HOME}"
    elif [ -d "/usr/local/share/android-sdk" ]; then
        export ANDROID_HOME="/usr/local/share/android-sdk"
        echo "📱 Found Android SDK at: ${ANDROID_HOME}"
    else
        echo "❌ Error: ANDROID_HOME is not set and SDK not found in common locations"
        echo "   Please set ANDROID_HOME environment variable or install Android SDK"
        exit 1
    fi
else
    echo "📱 Using Android SDK from ANDROID_HOME: ${ANDROID_HOME}"
fi

# Step 1: Navigate to the Android plugin directory
echo "📂 Navigating to: ${ANDROID_PLUGIN_DIR}"
cd "${ANDROID_PLUGIN_DIR}"

# Create local.properties if it doesn't exist
if [ ! -f "local.properties" ]; then
    echo "📝 Creating local.properties with SDK location..."
    echo "sdk.dir=${ANDROID_HOME}" > local.properties
fi

# Step 2: Execute Gradle build
echo "🏗️  Running Gradle build..."
if [ -f "./gradlew" ]; then
    chmod +x ./gradlew
    ./gradlew assemble
    
    if [ $? -eq 0 ]; then
        echo "✅ Gradle build completed successfully!"
    else
        echo "❌ Gradle build failed!"
        exit 1
    fi
else
    echo "❌ Error: gradlew not found in ${ANDROID_PLUGIN_DIR}"
    exit 1
fi

# Step 3: Copy plugin files to Godot addons (if they exist)
if [ -d "${PLUGIN_DEMO_ADDONS}" ]; then
    echo "📋 Copying plugin files to Godot addons..."
    
    # Create target directory if it doesn't exist
    mkdir -p "${GODOT_ADDONS_DIR}/dcl-godot-android"
    
    # Copy all files from plugin demo addons to godot addons
    cp -r "${PLUGIN_DEMO_ADDONS}"/* "${GODOT_ADDONS_DIR}/dcl-godot-android/" 2>/dev/null || {
        echo "⚠️  Warning: No files to copy from ${PLUGIN_DEMO_ADDONS}"
    }
    
    echo "✅ Plugin files copied to: ${GODOT_ADDONS_DIR}/dcl-godot-android"
else
    echo "ℹ️  Note: Plugin demo addons directory not found or empty"
    echo "         Path checked: ${PLUGIN_DEMO_ADDONS}"
fi

echo "================================================"
echo "✨ Android plugin build completed!"

# Optional: Display build artifacts location
echo ""
echo "📦 Build artifacts location:"
echo "   ${ANDROID_PLUGIN_DIR}/plugin/build/"