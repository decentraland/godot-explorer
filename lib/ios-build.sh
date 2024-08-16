#!/bin/bash

export FFMPEG_DIR=~/github/ffmpeg-kit/prebuilt/apple-ios-arm64/ffmpeg
export RUSTY_V8_MIRROR=https://github.com/leanmendoza/rusty_v8/releases/download

GN_ARGS=use_custom_libcxx=false RUST_BACKTRACE=full cargo build --target aarch64-apple-ios --release --no-default-features -F use_deno -F use_livekit
mkdir -p ../godot/lib/ios/
cp target/aarch64-apple-ios/release/libdclgodot.dylib ../godot/lib/ios/libdclgodot.dylib