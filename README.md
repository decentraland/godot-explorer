
# Decentraland Godot Rust
[![codecov](https://codecov.io/gh/decentraland/godot-explorer/branch/main/graph/badge.svg)](https://codecov.io/gh/decentraland/godot-explorer)

## Set up project

1. Clone the repo using `git clone https://github.com/decentraland/godot-explorer`
2. Install [rust](https://www.rust-lang.org/tools/install)
3. Download and install third party libraries
    - **Linux** (apt-get based):
      - Install alsa and udev: `sudo apt-get update; sudo apt-get install --no-install-recommends libasound2-dev libudev-dev`
      - Install ffmpeg deps: `sudo apt install -y --no-install-recommends clang curl pkg-config libavcodec-dev libavformat-dev libavutil-dev libavfilter-dev libavdevice-dev`
      - Install Livekit deps: `sudo apt update -y; sudo apt install -y libssl-dev libx11-dev libgl1-mesa-dev libxext-dev`
    - **MacOS**: `brew install ffmpeg pkg-config`
    - **Windows**: 
      - download and unzip `https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-full-shared.7z`
      - set `LIBCLANG_PATH` = `path to LLVM\x64\bin` (this is packaged with visual studio, or can be downloaded separately)
      - set `FFMPEG_DIR` = `root folder where ffmpeg has been unzipped`
      - add `ffmpeg\bin` to your `PATH`
    - the `.github/workflows/ci.yml` file can be useful to guide you

2. Go to `rust/xtask` folder, and run `cargo run -- install`.

## Running and editting the project

1. Ensure you are in `rust/xtask` folder first
2. You can run `cargo run -- run` to build the Rust library and execute the client. 
- With adding `-r` it builds the library in release mode. Note: the Godot executable is an editor, so it's a `release_debug` build, see the Target section [here](https://docs.godotengine.org/en/stable/contributing/development/compiling/introduction_to_the_buildsystem.html) for more infromation.
- With adding `-e` it also builds the library, but the project edition is executed instead of the client.

## Contributing

More details on [CONTRIBUTING.md](CONTRIBUTING.md)

## Debugging the library
This repos is set up to be opened with Visual Studio Code. In the section `Run and Debug` in the Activity bar, you can find the configuration for your platform.

## Run test with coverage
1. Ensure you are in `rust/xtask` folder first
2. Run `cargo run -- coverage --dev`. It'll create a `coverage` folder with the index.html with the all information. For running this commands you need to have lvvm tools and grcov, you can install them with `rustup component add llvm-tools-preview` and `cargo install grcov`.

# Mobile targets
See `rust/decentraland-godot-lib/builds.md`


Powered by the Decentraland DAO
![Decentraland DAO logo](https://bafkreibci6gg3wbjvxzlqpuh353upzrssalqqoddb6c4rez33bcagqsc2a.ipfs.nftstorage.link/)
