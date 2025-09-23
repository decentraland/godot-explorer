#!/bin/bash

# Build script for DCL Godot iOS Plugin
# This script builds the iOS plugin and copies necessary files to the Godot project

set -e  # Exit on error

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
IOS_PLUGIN_DIR="${SCRIPT_DIR}/dcl-godot-ios"
GODOT_IOS_PLUGINS_DIR="${SCRIPT_DIR}/../godot/ios/plugins"
PLUGIN_BIN_RELEASE="${IOS_PLUGIN_DIR}/bin/release"

echo "🔨 Building DCL Godot iOS Plugin..."
echo "================================================"

# Check if we're on macOS (iOS builds require macOS)
if [[ "$OSTYPE" != "darwin"* ]]; then
    echo "❌ Error: iOS builds require macOS with Xcode installed"
    exit 1
fi

# Check for Xcode
if ! command -v xcodebuild &> /dev/null; then
    echo "❌ Error: Xcode is not installed or xcodebuild is not in PATH"
    echo "   Please install Xcode from the App Store"
    exit 1
fi

echo "📱 Xcode version:"
xcodebuild -version | head -1

# Check for SCons (required for building)
if ! command -v scons &> /dev/null; then
    echo "❌ Error: SCons is not installed"
    echo "   SCons is required to build the iOS plugin"
    echo ""
    echo "   To install SCons, run one of the following:"
    echo "   📦 Using pip:      pip install scons"
    echo "   📦 Using pip3:     pip3 install scons"
    echo "   📦 Using Homebrew: brew install scons"
    echo ""
    echo "   After installation, make sure scons is in your PATH"
    exit 1
fi

echo "🔧 SCons version:"
scons --version | head -1

# Step 1: Navigate to the iOS plugin directory
echo "📂 Navigating to: ${IOS_PLUGIN_DIR}"
cd "${IOS_PLUGIN_DIR}"

# Check if Godot headers are present
if [ ! -d "./godot" ] || [ -z "$(ls -A ./godot 2>/dev/null)" ]; then
    echo "⚠️  Warning: Godot source headers not found in ${IOS_PLUGIN_DIR}/godot"
    echo "   The iOS plugin requires Godot source headers to build."
    echo ""
    
    # Check if it's a git submodule
    if [ -f ".gitmodules" ] && grep -q "godot" ".gitmodules" 2>/dev/null; then
        echo "   The 'godot' directory appears to be a git submodule."
        echo "   To initialize and update the submodule, run:"
        echo ""
        echo "   📦 From the iOS plugin directory:"
        echo "      git submodule update --init --recursive"
        echo ""
        echo "   📦 Or from the project root:"
        echo "      git submodule update --init --recursive plugins/dcl-godot-ios/godot"
        echo ""
    else
        echo "   To set up Godot headers, you need to:"
        echo "   1. Clone Godot source: git clone https://github.com/godotengine/godot.git ${IOS_PLUGIN_DIR}/godot"
        echo "   2. Or create a symlink to existing Godot source:"
        echo "      ln -s /path/to/godot-source ${IOS_PLUGIN_DIR}/godot"
        echo ""
    fi
    
    echo "   Note: The plugin needs Godot headers matching your Godot version"
    echo ""
    
    # Check if there are pre-built plugins we can use
    if [ -d "${GODOT_IOS_PLUGINS_DIR}/dcl_godot_ios" ]; then
        echo "ℹ️  Found existing dcl_godot_ios plugin in Godot iOS plugins directory"
        echo "   Location: ${GODOT_IOS_PLUGINS_DIR}/dcl_godot_ios"
        echo "   The existing plugin will be used instead of building new one"
        exit 0
    else
        echo "❌ Cannot proceed without Godot headers"
        echo "   Please set up Godot source as described above"
        exit 1
    fi
fi

# Step 2: Generate Headers
echo "🔧 Generating headers..."
if [ -f "./scripts/generate_headers.sh" ]; then
    chmod +x ./scripts/generate_headers.sh
    ./scripts/generate_headers.sh 2>&1 | head -20 || {
        echo "⚠️  Warning: Header generation failed"
        echo "   This is often due to missing Godot source in ./godot directory"
        echo "   Continuing anyway to attempt XCFramework build..."
    }
else
    echo "⚠️  Warning: generate_headers.sh not found, skipping..."
fi

# Step 3: Compile Plugins (Build XCFramework)
echo "🏗️  Building XCFramework..."
if [ -f "./scripts/release_xcframework.sh" ]; then
    chmod +x ./scripts/release_xcframework.sh
    ./scripts/release_xcframework.sh
    
    if [ $? -eq 0 ]; then
        echo "✅ XCFramework build completed successfully!"
    else
        echo "❌ XCFramework build failed!"
        exit 1
    fi
else
    echo "❌ Error: release_xcframework.sh not found in ${IOS_PLUGIN_DIR}/scripts/"
    exit 1
fi

# List the built artifacts
if [ -d "${PLUGIN_BIN_RELEASE}" ]; then
    echo "📦 Built artifacts:"
    ls -l "${PLUGIN_BIN_RELEASE}"
else
    echo "⚠️  Warning: Release binary directory not found at ${PLUGIN_BIN_RELEASE}"
fi

# Step 4: Copy plugin files to Godot iOS plugins
if [ -d "${PLUGIN_BIN_RELEASE}" ]; then
    echo "📋 Copying plugin files to Godot iOS plugins..."
    
    # Create target directory if it doesn't exist
    mkdir -p "${GODOT_IOS_PLUGINS_DIR}"
    
    # Copy all files from bin/release to godot/ios/plugins/
    cp -r "${PLUGIN_BIN_RELEASE}"/* "${GODOT_IOS_PLUGINS_DIR}/" 2>/dev/null || {
        echo "⚠️  Warning: No files to copy from ${PLUGIN_BIN_RELEASE}"
        echo "         This might be expected if no new files were generated"
    }
    
    echo "✅ Plugin files copied to: ${GODOT_IOS_PLUGINS_DIR}"
    
    # List what was copied
    echo "📂 Contents of iOS plugins directory:"
    ls -la "${GODOT_IOS_PLUGINS_DIR}/"
else
    echo "❌ Error: Release directory not found at ${PLUGIN_BIN_RELEASE}"
    echo "   Build may have failed or output is in a different location"
    exit 1
fi

echo "================================================"
echo "✨ iOS plugin build completed!"

# Optional: Display additional information
echo ""
echo "ℹ️  Notes:"
echo "   - XCFramework includes both device and simulator architectures"
echo "   - Debug and Release builds are available"
echo "   - Plugin files are ready for Godot iOS export"