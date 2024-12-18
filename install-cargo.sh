#!/bin/sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain 1.77.2 -y
. "$HOME/.cargo/env"
rustup target add aarch64-linux-android
rustup target add x86_64-linux-android
