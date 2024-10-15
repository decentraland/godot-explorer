#!/bin/bash

USE_RELEASE=true
TARGET_TYPE=$( [ "$USE_RELEASE" = true ] && echo "release" || echo "debug" )
RELEASE_PARAM=$( [ "$USE_RELEASE" = true ] && echo "--release" )
export FFMPEG_DIR=~/github/ffmpeg-kit/prebuilt/apple-ios-arm64/ffmpeg
#export RUSTY_V8_MIRROR=https://github.com/leanmendoza/rusty_v8/releases/download
export RUSTY_V8_MIRROR=http://127.0.0.1:8080
V8_BINDING_FILE_NAME=src_binding_${TARGET_TYPE}_aarch64-apple-ios.rs
V8_BINDING=$RUSTY_V8_MIRROR/v129.0.0/$V8_BINDING_FILE_NAME
export RUSTY_V8_SRC_BINDING_PATH=$(pwd)/target/$V8_BINDING_FILE_NAME
# download if not exists
if [ ! -f "target/$V8_BINDING_FILE_NAME" ]; then
    curl -L -o target/$V8_BINDING_FILE_NAME $V8_BINDING
fi

GN_ARGS=use_custom_libcxx=false RUST_BACKTRACE=full cargo build --target aarch64-apple-ios ${RELEASE_PARAM} --no-default-features -F use_deno
mkdir -p ../godot/lib/ios/
cp target/aarch64-apple-ios/release/libdclgodot.dylib ../godot/lib/ios/libdclgodot.dylib