# Decentraland Godot Rust

## Dependencies

1. Install rust (https://www.rust-lang.org/tools/install)
2. Go to `rust` folder, and run `cargo xtask install`.

### 1. Editing the godot project

You can open the folder `godot/` with the Godot editor.

### 2. Editing the GD Extension Rust library

Open this repo with VSCode, run the task pressing `Cmd+Shift+P` or `Control+Shift+P`, type `Run task` then Enter and look for your platform when writing `Copy GDExtension Lib`.

## Run test with coverage
Run `cargo xtask coverage --dev`. It'll create a `coverage` folder with the index.html with the all information. For running this commands you need to have lvvm tools and grcov, you can install them with `rustup component add llvm-tools-preview` and `cargo install grcov`.
