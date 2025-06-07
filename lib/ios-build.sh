#!/bin/bash


export FFMPEG_DIR=~/github/ffmpeg-kit/prebuilt/apple-ios-arm64/ffmpeg

# Option 1: Use local rusty_v8 archive
# Uncomment and set path to your local rusty_v8 .tar.gz file
export RUSTY_V8_ARCHIVE=/Users/kuruk/Projects/rusty_v8/out/v0.106.0/librusty_v8_release_aarch64-apple-ios.a.gz
export RUSTY_V8_SRC_BINDING_PATH=/Users/kuruk/Projects/rusty_v8/out/v0.106.0/src_binding_release_aarch64-apple-ios.rs

# Option 2: Use mirror (default)
#export RUSTY_V8_MIRROR=https://github.com/leanmendoza/rusty_v8/releases/download

GN_ARGS=use_custom_libcxx=false RUST_BACKTRACE=full cargo build --target aarch64-apple-ios --release --no-default-features -F use_deno -F use_livekit
mkdir -p ../godot/lib/ios/
cp target/aarch64-apple-ios/release/libdclgodot.dylib ../godot/lib/ios/libdclgodot.dylib