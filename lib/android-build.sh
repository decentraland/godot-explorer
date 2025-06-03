#!/bin/bash

echo $0

set -e

if [[ -z "${ANDROID_NDK}" ]]; then
    # Tested with NDK 27.1.12297006
    if [[ -z "${ANDROID_SDK}" ]]; then
        ANDROID_NDK=$ANDROID_SDK/ndk/27.1.12297006
    else
        ANDROID_NDK=~/Android/Sdk/ndk/27.1.12297006
    fi
    ANDROID_NDK_HOME=$ANDROID_NDK
fi

if [[ -z "${FFMPEG_DIR}" ]]; then
    FFMPEG_DIR=~/Documents/github/ffmpeg-kit/prebuilt/android-arm64/ffmpeg
fi

echo "ANDROID_NDK: $ANDROID_NDK"
echo "ANDROID_SDK: $ANDROID_SDK"
echo "ANDROID_HOME: $ANDROID_HOME"
echo "FFMPEG_DIR: $FFMPEG_DIR"

# Check if the script is invoked with the x86_64 parameter
if [[ "$1" == "x86_64" ]]; then
    export TARGET_CC=$ANDROID_NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/x86_64-linux-android35-clang
    export TARGET_CXX=$ANDROID_NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/x86_64-linux-android35-clang++
    export TARGET_AR=$ANDROID_NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar
    export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER=$ANDROID_NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/x86_64-linux-android35-clang

    GN_ARGS=use_custom_libcxx=false RUST_BACKTRACE=full cargo build --release --no-default-features --target x86_64-linux-android -vv --verbose 
    mkdir -p target/libdclgodot_android_x86/
    cp target/x86_64-linux-android/release/libdclgodot.so target/libdclgodot_android_x86/
else
    # Run the specified commands
    export TARGET_CC=$ANDROID_NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android35-clang
    export TARGET_CXX=$ANDROID_NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android35-clang++
    export TARGET_AR=$ANDROID_NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar
    export CARGO_FFMPEG_SYS_DISABLE_SIZE_T_IS_USIZE=1
    export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$ANDROID_NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android35-clang"
    export CARGO_PROFILE_RELEASE_BUILD_OVERRIDE_DEBUG=true
    
    export CXXFLAGS="-v --target=aarch64-linux-android"
    export RUSTFLAGS="-L${ANDROID_NDK}/toolchains/llvm/prebuilt/linux-x86_64/lib/aarch64-unknown-linux-musl"

    # CI 
    export RUSTY_V8_MIRROR=https://github.com/dclexplorer/rusty_v8/releases/download
    V8_BINDING_FILE_NAME=src_binding_debug_aarch64-linux-android.rs
    V8_BINDING=$RUSTY_V8_MIRROR/v0.106.0/$V8_BINDING_FILE_NAME
    export RUSTY_V8_SRC_BINDING_PATH=$(pwd)/target/$V8_BINDING_FILE_NAME
    # download if not exists
    if [ ! -f "target/$V8_BINDING_FILE_NAME" ]; then
        curl -L -o target/$V8_BINDING_FILE_NAME $V8_BINDING
    fi
    GODOT_DIR=../godot
    
    # Local development
    # - V8 local build
    # export RUSTY_V8_SRC_BINDING_PATH=/home/user/github/rusty_v8_updated/target/aarch64-linux-android/release/gn_out/src_binding.rs
    # export RUSTY_V8_ARCHIVE=/home/user/github/rusty_v8_updated/target/aarch64-linux-android/release/gn_out/obj/librusty_v8.a
    # - godot local godot/android project
    # export GODOT_DIR=/mnt/c/explorer/godot

    GN_ARGS=use_custom_libcxx=false RUST_BACKTRACE=full cargo build --release --target aarch64-linux-android --no-default-features -F use_deno -F use_livekit

    # Dependencies 
    # - from web-rtc: libwebrtc.jar 
    # - from ffmpeg: libavcodec, libavfilter, libavdevice, libavformat, libavutil, libswresample, libswscale
    mkdir -p target/libdclgodot_android/
    cp target/aarch64-linux-android/release/libdclgodot.so target/libdclgodot_android/
fi