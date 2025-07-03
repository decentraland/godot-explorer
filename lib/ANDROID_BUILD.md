# Android Build Guide

This guide explains how to build the Decentraland Godot Explorer for Android.

## Prerequisites

1. **Android SDK and NDK**
   - Install Android SDK
   - Install Android NDK version 27.1.12297006
   - Set appropriate environment variables (see below)

2. **Rust toolchain**
   ```bash
   # Add Android target
   rustup target add aarch64-linux-android
   ```

3. **cargo-ndk (Recommended)**
   ```bash
   cargo install cargo-ndk
   ```

## Environment Variables

Set one of the following:
- `ANDROID_NDK_HOME` - Direct path to NDK
- `ANDROID_NDK` - Direct path to NDK
- `ANDROID_SDK` or `ANDROID_HOME` - Path to SDK (NDK expected at `*/ndk/27.1.12297006`)

Default location if none set: `~/Android/Sdk/ndk/27.1.12297006`

## Building

### Method 1: Using cargo-ndk (Recommended)

From the project root:
```bash
cargo run -- build --target android
```

This automatically:
- Downloads V8 bindings
- Configures cross-compilation
- Builds with correct features
- Places output in `lib/target/libdclgodot_android/`

### Method 2: Direct cargo-ndk

```bash
cd lib
bash android-build-ndk.sh
```

### Method 3: Traditional build (Fallback)

```bash
cd lib
bash android-build.sh
```

## Features

The Android build includes:
- `use_deno` - V8/Deno runtime support
- `use_livekit` - WebRTC/LiveKit support
- `use_ffmpeg` - Audio/Video processing

## Troubleshooting

1. **cargo-ndk not found**
   - Install with: `cargo install cargo-ndk`
   - Or use traditional build method

2. **NDK not found**
   - Check environment variables
   - Verify NDK version 27.1.12297006 is installed
   - Check default path: `~/Android/Sdk/ndk/27.1.12297006`

3. **V8 binding download fails**
   - Check internet connection
   - Manually download from: https://github.com/dclexplorer/rusty_v8/releases/download/v0.106.0/

4. **Build fails with linker errors**
   - Verify NDK version is correct
   - Check ANDROID_NDK_HOME is set properly
   - Ensure Android target is added to rustup

## Output

The built library will be at:
- `lib/target/aarch64-linux-android/release/libdclgodot.so`
- Copied to: `lib/target/libdclgodot_android/libdclgodot.so`

## Integration with Godot

After building, the library is ready to be included in the Godot Android export. The `build-android-apk.sh` script handles the full APK build process.