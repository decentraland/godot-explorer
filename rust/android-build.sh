#!/bin/bash

# Store the original content of decentraland-godot-lib/Cargo.toml
original_content=$(cat decentraland-godot-lib/Cargo.toml)

# Modify decentraland-godot-lib/Cargo.toml to use the local path
sed -i 's/v8 = "0.74.2"/v8 = { path = "\/home\/user\/Documents\/github\/rusty_v8" }/' decentraland-godot-lib/Cargo.toml

# Run the specified commands
export TARGET_CC=/home/user/.NDK/arm64/bin/aarch64-linux-android-clang
export TARGET_AR=/home/user/.NDK/arm64/bin/llvm-ar
cd decentraland-godot-lib
(V8_FROM_SOURCE=1 cargo build --target aarch64-linux-android -vv --verbose --release) || true
cd ..
(cp target/aarch64-linux-android/release/libdecentraland_godot_lib.so ../godot/lib/android/libdecentraland_godot_lib.so) || true

# Revert decentraland-godot-lib/Cargo.toml back to its original content
echo "$original_content" > decentraland-godot-lib/Cargo.toml