
# Use nightly toolchain
rustup default nightly
rustup target add wasm32-unknown-emscripten

# Disable no-wasm and webrtc_sys_build features
sed -i '/disable-wasm/s/^/# disabled /' Cargo.toml
sed -i '/disable-wasm/s/^/\/\/ disabled /' build.rs

cargo +nightly build -Zbuild-std --no-default-features --target wasm32-unknown-emscripten || true

# Re enable no-wasm and webrtc_sys_build features
sed -i '/^# disabled/s/^# disabled //' Cargo.toml
sed -i '/^\/\/ disabled/s/^\/\/ disabled //' build.rs