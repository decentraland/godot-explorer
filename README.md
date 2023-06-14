# Decentraland Godot Rust

## Install dependencies

1. Install rust (https://www.rust-lang.org/tools/install)
2. Go to `rust` folder, and run `cargo xtask install`.

## Running and editting the project

You can run `cargo xtask run` to build the Rust library and execute the client. 
- With `-r` it builds the library in release mode. Note: the Godot executable is an editor, so it's a `release_debug` build, see the Target section [here](https://docs.godotengine.org/en/stable/contributing/development/compiling/introduction_to_the_buildsystem.html) for more infromation.
- With `-e` it also builds the library, but the project edition is executed instead of the client.

## Debugging the library
This repos is set up to be opened with Visual Studio Code. In the section `Run and Debug` in the Activity bar, you can find the configuration for your platform.

## Run test with coverage
Run `cargo xtask coverage --dev`. It'll create a `coverage` folder with the index.html with the all information. For running this commands you need to have lvvm tools and grcov, you can install them with `rustup component add llvm-tools-preview` and `cargo install grcov`.
