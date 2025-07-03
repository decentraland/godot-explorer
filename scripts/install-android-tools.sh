#!/bin/bash

# Script to install Android build tools for Decentraland Godot Explorer

set -e

echo "Installing Android build tools..."

# Check if rustup is installed
if ! command -v rustup &> /dev/null; then
    echo "Error: rustup is not installed. Please install Rust first."
    echo "Visit: https://rustup.rs/"
    exit 1
fi

# Add Android target
echo "Adding Android target to rustup..."
rustup target add aarch64-linux-android

# Install cargo-ndk
echo "Installing cargo-ndk..."
cargo install cargo-ndk

# Check Android SDK/NDK
echo ""
echo "Checking Android SDK/NDK setup..."

if [[ -n "${ANDROID_NDK_HOME}" ]]; then
    echo "✓ ANDROID_NDK_HOME is set: ${ANDROID_NDK_HOME}"
elif [[ -n "${ANDROID_NDK}" ]]; then
    echo "✓ ANDROID_NDK is set: ${ANDROID_NDK}"
elif [[ -n "${ANDROID_SDK}" ]]; then
    echo "✓ ANDROID_SDK is set: ${ANDROID_SDK}"
    echo "  Looking for NDK at: ${ANDROID_SDK}/ndk/27.1.12297006"
elif [[ -n "${ANDROID_HOME}" ]]; then
    echo "✓ ANDROID_HOME is set: ${ANDROID_HOME}"
    echo "  Looking for NDK at: ${ANDROID_HOME}/ndk/27.1.12297006"
else
    echo "⚠ No Android SDK/NDK environment variables found"
    echo "  Will look for NDK at: ~/Android/Sdk/ndk/27.1.12297006"
fi

echo ""
echo "Android build tools installation complete!"
echo ""
echo "To build for Android, run:"
echo "  cargo run -- build --target android"
echo ""
echo "Note: Make sure you have Android NDK version 27.1.12297006 installed"