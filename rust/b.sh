#!/bin/bash

# Store the original content of decentraland-godot-lib/Cargo.toml
original_content=$(cat decentraland-godot-lib/Cargo.toml)

# Modify decentraland-godot-lib/Cargo.toml to use the local path
sed -i 's/v8 = "0.74.2"/v8 = { path = "\/home\/user\/Documents\/github\/rusty_v8" }/' decentraland-godot-lib/Cargo.toml

# Run the specified commands
export TARGET_CC=/home/user/Android/Sdk/ndk/25.2.9519653/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android33-clang
export TARGET_CXX=/home/user/Android/Sdk/ndk/25.2.9519653/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android33-clang++
export TARGET_AR=/home/user/Android/Sdk/ndk/25.2.9519653/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar
export FFMPEG_DIR=/home/user/Documents/github/ffmpeg-kit/prebuilt/android-arm64/ffmpeg

cd decentraland-godot-lib
(V8_FROM_SOURCE=1 GN_ARGS=use_custom_libcxx=false RUST_BACKTRACE=full cargo build --target aarch64-linux-android -vv --verbose --release) || true
cd ..
(cp target/aarch64-linux-android/release/libdecentraland_godot_lib.so ../godot/lib/android/libdecentraland_godot_lib.so) || true

# Revert decentraland-godot-lib/Cargo.toml back to its original content
echo "$original_content" > decentraland-godot-lib/Cargo.toml