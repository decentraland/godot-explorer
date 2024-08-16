#!/bin/bash

echo $0

set -e

if [[ -z "${ANDROID_NDK}" ]]; then
    # Tested with NDK 25.2.9519653
    if [[ -z "${ANDROID_SDK}" ]]; then
        ANDROID_NDK=$ANDROID_SDK/ndk/25.2.9519653
    else
        ANDROID_NDK=~/Android/Sdk/ndk/25.2.9519653
    fi
    ANDROID_NDK_HOME=$ANDROID_NDK
fi

if [[ -z "${FFMPEG_DIR}" ]]; then
    FFMPEG_DIR=~/Documents/github/ffmpeg-kit/prebuilt/android-arm64/ffmpeg
fi

# Check if the script is invoked with the x86_64 parameter
if [[ "$1" == "x86_64" ]]; then
    export TARGET_CC=$ANDROID_NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/x86_64-linux-android33-clang
    export TARGET_CXX=$ANDROID_NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/x86_64-linux-android33-clang++
    export TARGET_AR=$ANDROID_NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar
    export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER=$ANDROID_NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/x86_64-linux-android33-clang

    GN_ARGS=use_custom_libcxx=false RUST_BACKTRACE=full cargo build --release --no-default-features --target x86_64-linux-android -vv --verbose 

    mkdir -p ../godot/lib/android/x86_64/
    mkdir -p ../godot/android/build/libs/release/x86_64/

    cp target/x86_64-linux-android/release/libdclgodot.so ../godot/lib/android/x86_64/libdclgodot.so
    cp target/x86_64-linux-android/release/libdclgodot.so ../godot/android/build/libs/release/x86_64/libdclgodot.so

else
    # Run the specified commands
    export TARGET_CC=$ANDROID_NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android33-clang
    export TARGET_CXX=$ANDROID_NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android33-clang++
    export TARGET_AR=$ANDROID_NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar
    export RUSTY_V8_MIRROR=https://github.com/leanmendoza/rusty_v8/releases/download
    export CARGO_FFMPEG_SYS_DISABLE_SIZE_T_IS_USIZE=1
    export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=$ANDROID_NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android33-clang

    GN_ARGS=use_custom_libcxx=false RUST_BACKTRACE=full cargo build --release --target aarch64-linux-android

    # Dependencies 
    # - from web-rtc: libwebrtc.jar 
    # - from ffmpeg: libavcodec, libavfilter, libavdevice, libavformat, libavutil, libswresample, libswscale
    mkdir -p ../godot/lib/android/arm64/
    mkdir -p ../godot/android/build/libs/release/arm64-v8a/

    cp target/aarch64-linux-android/release/libdclgodot.so ../godot/lib/android/arm64/libdclgodot.so
    cp target/aarch64-linux-android/release/libdclgodot.so ../godot/android/build/libs/release/arm64-v8a/libdclgodot.so
fi