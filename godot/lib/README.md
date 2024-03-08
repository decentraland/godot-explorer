Put decentraland_godot_lib libraries files here:
- Windows: decentraland_godot_lib.dll
- MacOS: libdecentraland_godot_lib.dylib
- Linux: libdecentraland_godot_lib.so
- iOS: libdecentralandgodot.dylib

# Building
First you need to run `cargo build` in `rust/`, then find it on `rust/target/debug`.

Opening with VSCode you can run the task with `Cmd+Shift+P` or `Control+Shift+P`, write `Run task` and look for your platform when writing `Copy GDExtension Lib`.
