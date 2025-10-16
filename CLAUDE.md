# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Decentraland Godot Explorer is a cross-platform metaverse client that combines:
- **Godot Engine 4.5.1** (custom fork) for 3D rendering and UI
- **Rust** for core systems and performance-critical components
- **GDScript** for game logic
- **JavaScript/V8** runtime for executing Decentraland SDK scenes

## Essential Commands

All commands use the xtask pattern via `cargo run --`:

### System Health & Dependencies
```bash
# Check system health and dependencies
cargo run -- doctor

# Install dependencies (specify platforms: linux, windows, macos, android, ios)
cargo run -- install                      # Installs protoc and Godot only
cargo run -- install --targets linux    # Also installs Linux export templates
cargo run -- install --targets android  # Also installs Android tools and templates
```

### Development
```bash
# Build the Rust library
cargo run -- build                        # Build for host platform
cargo run -- build -r                     # Release mode
cargo run -- build --target android       # Android build (no cargo-ndk, uses direct cargo)
cargo run -- build --target ios           # iOS build (macOS only)

# Run the client (automatically builds first)
cargo run -- run                          # Run client
cargo run -- run -r                       # Release mode
cargo run -- run -e                       # Run editor
cargo run -- run -e --target android      # Run editor and also build for Android
cargo run -- run --target android         # Build, export APK, and deploy to device
cargo run -- run --target ios             # Build, export IPA, and deploy to device
cargo run -- run --target android --only-lib  # Build and push .so only (requires debug APK)
cargo run -- run --target android --only-lib -- --skip-lobby  # Hotreload with app parameters

# Feature flags
cargo run -- build --no-default-features --features use_livekit,use_deno
cargo run -- run --no-default-features --features use_livekit,use_deno

# Run tests
cargo run -- run --itest                  # Integration tests
cargo run -- run --stest                  # Scene tests
```

### Android Development
```bash
# Export Android builds (keystores are generated automatically if needed)
cargo run -- export --target android --format apk           # Build debug APK (uses debug keystore)
cargo run -- export --target android --format apk --release # Build signed APK (uses release keystore)
cargo run -- export --target android --format aab --release # Build AAB for Play Store
cargo run -- export --target quest --format apk --release   # Meta Quest build

# Note: Keystores are automatically created in .bin/ folder:
# - .bin/debug.keystore for debug builds
# - .bin/release.keystore for release builds
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

# Import assets
cargo run -- import-assets
```

### Export & Distribution
```bash
# Export for target OS
cargo run -- export --target linux
cargo run -- export --target windows
cargo run -- export --target macos
cargo run -- export --target android --format apk
cargo run -- export --target android --format aab
cargo run -- export --target ios
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
   # First install Android tools and dependencies
   cargo run -- install --targets android
   
   # Build for Android
   cargo run -- build --target android
   
   # Export APK or AAB (keystore is automatically generated and configured)
   cargo run -- export --target android --format apk --release
   
   # Or use Docker (for CI/CD):
   docker run -v $(pwd):/app/ -it kuruk/dcl-godot-android-builder-rust
   ```

## Important Notes

- The project uses a forked Godot 4.5.1 - don't update the engine version
- Windows users should clone to short paths (e.g., `C:/gexplorer`)
- The Rust toolchain is pinned in `rust-toolchain.toml` (1.79)
- For coverage testing, install: `rustup component add llvm-tools-preview && cargo install grcov`
- Integration with Decentraland SDK7 requires the JavaScript runtime to be properly initialized
- **Android builds**: No longer use cargo-ndk due to NDK 27 issues. Direct cargo build with `GN_ARGS=use_custom_libcxx=false`
- **FFmpeg**: In process of deprecation (not intention to fix it) previously automatically disabled for Android/iOS builds (TODO: mobile implementation)
- **Dependencies**: Run `cargo run -- doctor` to check system health and missing dependencies
- **Build order**: Commands now check dependencies and suggest next steps automatically

## New Features (Recent Updates)

### Enhanced Developer Experience
- **Colored output**: All xtask commands now use colored output for better readability
- **Progress indicators**: Long-running operations show progress bars
- **Dependency checking**: Commands validate prerequisites and provide helpful error messages
- **Platform detection**: Automatically detects OS and suggests platform-specific commands
- **Smart defaults**: FFmpeg automatically disabled, dead code for docs purposes

### Improved Android Workflow
```bash
# Complete Android build workflow
cargo run -- install --targets android           # Install Android dependencies
cargo run -- build --target android                # Build Rust library
cargo run -- export --target android --format apk  # Export APK
```

### Command Dependencies
The build system now enforces proper command order:
- `build` requires: protoc installed
- `run` requires: Godot installed (builds automatically)
- `export` requires: Godot installed, host built, target platform built
- `import-assets` requires: Godot installed (builds automatically)

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