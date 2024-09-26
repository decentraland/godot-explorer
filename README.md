
# Decentraland Godot Rust
[![codecov](https://codecov.io/gh/decentraland/godot-explorer/branch/main/graph/badge.svg)](https://codecov.io/gh/decentraland/godot-explorer)

## Set up project

1. Clone the repo using `git clone https://github.com/decentraland/godot-explorer`
  - If you're in Windows we suggest to clone the repo in a very short path like `C:/gexplorer` due https://developercommunity.visualstudio.com/t/clexe-compiler-driver-cannot-handle-long-file-path/975889
2. Install [rust](https://www.rust-lang.org/tools/install)
3. Download and install third party libraries
    - **Linux** (apt-get based):
      - Install alsa and udev: `sudo apt-get update; sudo apt-get install --no-install-recommends libasound2-dev libudev-dev`
      - Install ffmpeg deps: `sudo apt install -y --no-install-recommends clang curl pkg-config libavcodec-dev libavformat-dev libavutil-dev libavfilter-dev libavdevice-dev`
      - Install Livekit deps: `sudo apt update -y; sudo apt install -y libssl-dev libx11-dev libgl1-mesa-dev libxext-dev`
    - **MacOS**: `brew install ffmpeg@6 pkg-config`
    - **Windows**: 
      - download and unzip `https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-full-shared.7z`
      - set `LIBCLANG_PATH` = `path to LLVM\x64\bin` (this is packaged with visual studio, or can be downloaded separately)
      - set `FFMPEG_DIR` = `root folder where ffmpeg has been unzipped`
      - add `ffmpeg\bin` to your `PATH`
    - the `.github/workflows/ci.yml` file can be useful to guide you

4. Run `cargo run -- install` in the repo root folder.

## Running and editing the project

1. Ensure you are in the root folder first
2. You can run `cargo run -- run` to build the Rust library and execute the client. 
- With adding `-r` it builds the library in release mode. Note: the Godot executable is an editor, so it's a `release_debug` build, see the Target section [here](https://docs.godotengine.org/en/stable/contributing/development/compiling/introduction_to_the_buildsystem.html) for more information.
- With adding `-e` it also builds the library, but the project edition is executed instead of the client.

## Docker Set up project with Docker (for Android and Linux)

Execute the following commands for building Godot:
```bash
# Run Docker
docker run -v {godot-explorer-repo-path}:/app/ -it kuruk/dcl-godot-android-builder:latest

# Compile for Android
cd lib
./android-build.sh # arm64
./android-build.sh x86_64 # android x86_64 if needed
cd ../../ # return

# Compile for Linux
cargo run -- install
cargo run -- run --only-build
cd ../../ # return

# Generate .APK
## Download Android Template, only one time needed
cd godot/
git clone https://github.com/decentraland/godot-explorer-android-template.git android # Only one time
cd ../

## Build Android and Export APK for arm64
./build-android-apk.sh
```

## Contributing

More details on [CONTRIBUTING.md](CONTRIBUTING.md)

## Debugging the library
This repo is set up to be opened with Visual Studio Code. In the section `Run and Debug` in the Activity bar, you can find the configuration for your platform.

## Run test with coverage
1. Ensure you are in the root folder first
2. Run `cargo run -- coverage --dev`. It'll create a `coverage` folder with the index.html with the all information. In order to run these commands, you need to have llvm-tools and grcov installed. You can install them with `rustup component add llvm-tools-preview` and `cargo install grcov`.

# Mobile targets
See `lib/builds.md`

Powered by the Decentraland DAO
![Decentraland DAO logo](https://bafkreibci6gg3wbjvxzlqpuh353upzrssalqqoddb6c4rez33bcagqsc2a.ipfs.nftstorage.link/)
