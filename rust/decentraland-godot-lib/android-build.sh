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
    TARGET="x86_64-linux-android"

    GN_ARGS=use_custom_libcxx=false RUST_BACKTRACE=full cargo build --target $TARGET -vv --verbose --release --no-default-features

    mkdir -p ../../godot/lib/android/x86_64/
    cp target/$TARGET/release/libdecentraland_godot_lib.so ../../godot/lib/android/x86_64/libdecentraland_godot_lib.so
    mkdir -p ../../godot/android/build/libs/debug/x86_64/
    cp target/$TARGET/release/libdecentraland_godot_lib.so ../../godot/android/build/libs/debug/x86_64/libdecentraland_godot_lib.so

else
    # Run the specified commands
    export TARGET_CC=$ANDROID_NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android33-clang
    export TARGET_CXX=$ANDROID_NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android33-clang++
    export TARGET_AR=$ANDROID_NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar
    export RUSTY_V8_MIRROR=https://github.com/leanmendoza/rusty_v8/releases/download
    export CARGO_FFMPEG_SYS_DISABLE_SIZE_T_IS_USIZE=1
    export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=$ANDROID_NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android33-clang


    GN_ARGS=use_custom_libcxx=false RUST_BACKTRACE=full cargo build --target aarch64-linux-android -vv --verbose --release

    mkdir -p ../../godot/lib/android/
    cp target/aarch64-linux-android/release/libdecentraland_godot_lib.so ../../godot/lib/android/libdecentraland_godot_lib.so
    mkdir -p ../../godot/android/build/libs/debug/arm64-v8a/
    cp target/aarch64-linux-android/release/libdecentraland_godot_lib.so ../../godot/android/build/libs/debug/arm64-v8a/libdecentraland_godot_lib.so

    # Dependencies 
    # - from web-rtc: libwebrtc.jar 
    # - from ffmpeg: libavcodec, libavfilter, libavdevice, libavformat, libavutil, libswresample, libswscale
fi