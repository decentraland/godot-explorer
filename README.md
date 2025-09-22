# Decentraland Godot Explorer

[![codecov](https://codecov.io/gh/decentraland/godot-explorer/branch/main/graph/badge.svg)](https://codecov.io/gh/decentraland/godot-explorer)
[![CI](https://github.com/decentraland/godot-explorer/actions/workflows/ci.yml/badge.svg)](https://github.com/decentraland/godot-explorer/actions)
[![Android](https://github.com/decentraland/godot-explorer/actions/workflows/android_builds.yml/badge.svg)](https://github.com/decentraland/godot-explorer/actions)

Decentraland Godot Explorer is a cross-platform metaverse client built with Godot 4.4.1 and Rust, supporting desktop, mobile, and VR platforms.

## ✨ Features

- **Cross-Platform**: Native support for Linux, Windows, macOS, Android, iOS, and VR
- **High Performance**: Rust core with Godot rendering engine
- **Decentraland SDK7**: Full compatibility with Decentraland scenes
- **Voice Chat**: Integrated spatial audio via LiveKit WebRTC
- **Web3 Integration**: Ethereum wallet support for NFTs and transactions
- **Developer Friendly**: Hot reload, comprehensive testing, and debugging tools

## 🚀 Quick Start

1. **Prerequisites**:
   - [Rust 1.79+](https://www.rust-lang.org/tools/install)
   - Git
   - Platform-specific dependencies (see below)

2. **Clone and setup**:
   ```bash
   git clone https://github.com/decentraland/godot-explorer
   cd godot-explorer
   # Windows users: use a short path like C:/gexplorer
   
   # Check system health
   cargo run -- doctor
   
   # Install Godot and build tools
   cargo run -- install --targets <your-platform>
   ```

3. **Run the project**:
   ```bash
   # Run the client (builds automatically)
   cargo run -- run
   
   # Or run the editor
   cargo run -- run -e
   ```

## 📦 Platform-Specific Dependencies

### Linux (Ubuntu/Debian)
```bash
sudo apt-get update && sudo apt-get install -y \
  libasound2-dev libudev-dev \
  clang curl pkg-config \
  libavcodec-dev libavformat-dev libavutil-dev libavfilter-dev libavdevice-dev \
  libssl-dev libx11-dev libgl1-mesa-dev libxext-dev
```

### macOS
```bash
brew install ffmpeg@6 pkg-config

# Set environment variables (add to ~/.zshrc)
export PKG_CONFIG_PATH="/opt/homebrew/opt/ffmpeg@6/lib/pkgconfig:$PKG_CONFIG_PATH"
```

### Windows
1. Download [FFmpeg shared libraries](https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-full-shared.7z)
2. Set environment variables:
   - `FFMPEG_DIR` = path to unzipped FFmpeg
   - `LIBCLANG_PATH` = path to LLVM\x64\bin (comes with Visual Studio)
3. Add `%FFMPEG_DIR%\bin` to PATH

### Android Development
- Android SDK with NDK 27.1.12297006
- Set `ANDROID_SDK` or `ANDROID_HOME` environment variable
- Run: `rustup target add aarch64-linux-android`

## 📋 Quick Reference

| Command | Description |
|---------|-------------|
| `cargo run -- doctor` | Check system health and dependencies |
| `cargo run -- install` | Install Godot and protoc |
| `cargo run -- build` | Build for host platform |
| `cargo run -- run` | Build and run the client |
| `cargo run -- run -e` | Build and run the editor |
| `cargo run -- export --target android` | Export Android APK |

## 🛠️ Development Workflow

### Building and Running

```bash
# Build for host platform
cargo run -- build

# Run the client (builds automatically)
cargo run -- run

# Run the editor
cargo run -- run -e

# Run with specific features
cargo run -- run --no-default-features --features use_livekit,use_deno
```

### Android Development

```bash
# Install Android dependencies
cargo run -- install --targets android

# Build for Android
cargo run -- build --target android

# Export APK (automatically uses .bin/release.keystore)
cargo run -- export --target android --format apk --release

# Export AAB for Play Store
cargo run -- export --target android --format aab --release
```

### iOS Development (macOS only)

```bash
# Install iOS dependencies
cargo run -- install --targets ios
rustup target add aarch64-apple-ios

# Build for iOS
cargo run -- build --target ios

# Export iOS app
cargo run -- export --target ios
```

## 🧪 Testing

```bash
# Run integration tests
cargo run -- run --itest

# Run scene tests
cargo run -- run --stest

# Generate test coverage
rustup component add llvm-tools-preview
cargo install grcov
cargo run -- coverage --dev
```

## 🎮 Supported Platforms

- **Desktop**: Linux, Windows, macOS
- **Mobile**: Android (API 29+), iOS
- **VR**: Quest (Meta), OpenXR compatible devices

## 📁 Project Structure

```
godot-explorer/
├── godot/              # Godot project files
│   ├── src/            # GDScript source code
│   └── project.godot   # Project configuration
├── lib/                # Rust library
│   ├── src/            # Rust source code
│   └── Cargo.toml      # Rust dependencies
├── src/                # Build system (xtask)
└── exports/            # Build outputs
```

## 🐳 Docker Support

For CI/CD or consistent build environments:

```bash
# Linux/Android builds
docker run -v $(pwd):/app/ -it kuruk/dcl-godot-android-builder-rust

# Inside container
cargo run -- install --targets android linux
cargo run -- build --target android
cargo run -- export --target android --format apk --release
```

## 🔧 Troubleshooting

1. **Check system health**:
   ```bash
   cargo run -- doctor
   ```

2. **Windows long path issues**: Clone to a short path like `C:/gexplorer`

3. **Missing dependencies**: The doctor command will show what's missing and how to install it

4. **Android build failures**: Ensure NDK 27.1.12297006 is installed and ANDROID_SDK is set

## 🤝 Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on:
- Code style and formatting
- Pull request process
- Testing requirements
- Documentation standards

## 📚 Documentation

- [Project Overview by Claude](https://github.com/decentraland/godot-explorer/blob/main/CLAUDE.md)

## 🔗 Links

- [Decentraland](https://decentraland.org)
- [Godot Engine Fork](https://github.com/decentraland/godotengine)
- [Discord Community](https://dcl.gg/discord)

---

Powered by the Decentraland DAO

![Decentraland DAO logo](https://bafkreibci6gg3wbjvxzlqpuh353upzrssalqqoddb6c4rez33bcagqsc2a.ipfs.nftstorage.link/)
