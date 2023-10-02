#!/bin/bash

set -e

# Tested with NDK 25.2.9519653
ANDROID_NDK=~/Android/Sdk/ndk/25.2.9519653
export FFMPEG_DIR=~/Documents/github/ffmpeg-kit/prebuilt/android-arm64/ffmpeg

# Run the specified commands
export TARGET_CC=$ANDROID_NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android33-clang
export TARGET_CXX=$ANDROID_NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android33-clang++
export TARGET_AR=$ANDROID_NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar
export RUSTY_V8_MIRROR=https://github.com/leanmendoza/rusty_v8/releases/download
export CARGO_FFMPEG_SYS_DISABLE_SIZE_T_IS_USIZE=1

export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=$ANDROID_NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android33-clang

# Store the original content of Cargo.toml
cargo_file_path="Cargo.toml"
original_content=$(cat $cargo_file_path)
ffmpeg_dep='ffmpeg-next = { git = "https://github.com/decentraland/rust-ffmpeg/", branch="audioline-and-mobile-fix" }'
ffmpeg_dep_android='ffmpeg-next = { git = "https://github.com/decentraland/rust-ffmpeg/", branch="audioline-and-mobile-fix", features=["fix_usize_size_t"] }'
sed -i "s|$ffmpeg_dep|$ffmpeg_dep_android|g" "$cargo_file_path"

(ANDROID_NDK_HOME=$ANDROID_NDK GN_ARGS=use_custom_libcxx=false RUST_BACKTRACE=full cargo build --target aarch64-linux-android -vv --verbose --release) || true

# Revert Cargo.toml back to its original content
echo "$original_content" > $cargo_file_path

mkdir -p ../../godot/lib/android/
cp target/aarch64-linux-android/release/libdecentraland_godot_lib.so ../../godot/lib/android/libdecentraland_godot_lib.so
cp target/aarch64-linux-android/release/libdecentraland_godot_lib.so ../../godot/android/build/libs/debug/arm64-v8a/libdecentraland_godot_lib.so

rsync -avu ${FFMPEG_DIR}/lib/*.so ../../godot/lib/android/
rsync -avu ${FFMPEG_DIR}/lib/*.so ../../godot/android/build/libs/debug/arm64-v8a/
# Dependencies 
# - from web-rtc: libwebrtc.jar 
# - from ffmpeg: libavcodec, libavfilter, libavdevice, libavformat, libavutil, libswresample, libswscale
