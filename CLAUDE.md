# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Decentraland Godot Explorer is a cross-platform metaverse client that combines:
- **Godot Engine 4.4.1** (custom fork) for 3D rendering and UI
- **Rust** for core systems and performance-critical components
- **GDScript** for game logic
- **JavaScript/V8** runtime for executing Decentraland SDK scenes

## Essential Commands

All commands use the xtask pattern via `cargo run --`:

### Development
```bash
# Install dependencies (specify platforms: linux, windows, macos, android, ios)
cargo run -- install --platforms linux
cargo run -- install --platforms android  # Installs cargo-ndk and Android target

# Build the Rust library
cargo run -- build
cargo run -- build -r  # Release mode
cargo run -- build --target android  # Android build using cargo-ndk

# Run the client
cargo run -- run
cargo run -- run -r    # Release mode
cargo run -- run -e    # Run editor

# Run tests
cargo run -- run --itest  # Integration tests
cargo run -- run --stest  # Scene tests
```

### Code Quality
```bash
# Rust formatting and linting
cd lib
cargo fmt --all
cargo clippy -- -D warnings

# GDScript formatting and linting
gdformat godot/
gdlint godot/

# Generate test coverage
cargo run -- coverage --dev
```

### Export & Distribution
```bash
# Export for target OS
cargo run -- export --target <OS>
```

## Architecture

### Directory Structure
- **`lib/`**: Core Rust library with all systems
  - `src/dcl/`: Decentraland-specific components (scene runner, SDK bindings)
  - `src/av/`: Audio/video processing (ffmpeg, video player)
  - `src/comms/`: WebRTC, voice chat (livekit)
  - `src/wallet/`: Ethereum integration
  - `src/content/`: Asset loading and caching
- **`godot/`**: Godot project
  - `src/decentraland_components/`: Custom Godot nodes for DCL features
  - `src/ui/`: UI components and HUD
  - `src/tool/`: Editor tools
- **`src/`**: xtask build system

### Key Systems

1. **Scene Management**: Scenes are loaded via the JavaScript runtime (deno_core/v8) which executes Decentraland SDK code
2. **Avatar System**: Handles wearables, animations, and customization through GLTF models
3. **Content Delivery**: Uses IPFS and content servers for asset distribution
4. **Voice Chat**: Integrated livekit WebRTC for spatial audio
5. **Ethereum Integration**: Wallet connection for Web3 features

### Platform Support
- Desktop: Linux, Windows, macOS
- Mobile: Android (API 29+), iOS
- VR: OpenXR compatible devices

## Development Workflow

1. **Always run formatting before commits**:
   ```bash
   cd lib && cargo fmt --all
   gdformat godot/
   ```

2. **Test changes with**:
   ```bash
   cargo run -- run
   ```

3. **For Android development**:
   ```bash
   # First install Android tools
   cargo run -- install --platforms android
   
   # Then build for Android
   cargo run -- build --target android
   
   # Or use Docker:
   docker run -v $(pwd):/app/ -it kuruk/dcl-godot-android-builder-rust
   ```

## Important Notes

- The project uses a forked Godot 4.4.1 - don't update the engine version
- Windows users should clone to short paths (e.g., `C:/gexplorer`)
- The Rust toolchain is pinned in `rust-toolchain.toml`
- For coverage testing, install: `rustup component add llvm-tools-preview && cargo install grcov`
- Integration with Decentraland SDK7 requires the JavaScript runtime to be properly initialized

## Common Tasks

### Adding a new Decentraland component:
1. Create the Rust implementation in `lib/src/dcl/components/`
2. Add GDExtension bindings in the component file
3. Create corresponding GDScript class in `godot/src/decentraland_components/`
4. Register in the scene runner

### Debugging scene loading:
1. Enable verbose logging: `RUST_LOG=debug cargo run -- run`
2. Check the scene runner logs in `lib/src/dcl/scene_runner.rs`
3. Verify content server responses in `lib/src/content/`

### Working with the avatar system:
- Avatar definitions are in `lib/src/avatar/`
- Wearables are loaded via GLTF in `lib/src/dcl/components/mesh_renderer/`
- Animation system uses Godot's AnimationPlayer nodes