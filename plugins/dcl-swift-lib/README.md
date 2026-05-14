# DclSwiftLib

Swift-based GDExtension for iOS. Implemented with [SwiftGodot](https://github.com/migueldeicaza/SwiftGodot) and packaged as an `xcframework` that Godot loads at runtime through `godot/dcl_swift_lib.gdextension`.

The Rust side exposes a typed wrapper (`DclSwiftLibPlugin`) that mirrors the pattern used by `DclIosPlugin` for `DclGodotiOS` — Rust and GDScript callers should go through the wrapper instead of touching the GDExtension class directly.

## Layout

```
plugins/dcl-swift-lib/
├── Package.swift                       # SwiftPM manifest (depends on SwiftGodot, pinned by SHA)
├── Makefile                            # `make xcframework` builds device + simulator
├── Sources/DclSwiftLib/DclSwiftLib.swift  # @Godot classes
├── build_ios_swift.sh                  # convenience wrapper around `make xcframework`
└── stub/                               # tiny C stub for non-iOS desktop builds
    ├── dcl_swift_lib_stub.c
    └── build_stub.sh
```

After a successful build the framework is installed at:

```
godot/ios/dcl-swift-lib/DclSwiftLib.framework
```

…which is the path referenced by `godot/dcl_swift_lib.gdextension`.

## Requirements

- macOS with **Xcode 15+** (Swift 5.9, iOS 17 / macOS 14 deployment targets)
- `xcodebuild` and `make` in `PATH`
- For the desktop stub: `clang` (ships with the Xcode command line tools)

## Build the iOS xcframework

From the repo root:

```sh
# Release (size-optimized, stripped — ~6.8 MB)
./plugins/build_swift_lib.sh release

# Debug (no size optimizations, full symbols — ~21 MB)
./plugins/build_swift_lib.sh debug
```

Both invocations call `make xcframework CONFIG=<Release|Debug>` inside `plugins/dcl-swift-lib/`, build the device + simulator slices, assemble the universal `xcframework`, and copy the device `.framework` into `godot/ios/dcl-swift-lib/`.

The same build step is triggered automatically by `plugins/build_ios.sh`, so the iOS plugin pipeline produces both the Obj-C plugin and the Swift framework in one shot.

## Build the desktop stub (macOS)

The desktop stub is a no-op `.dylib` that exports the same `dcl_swift_lib_init` entry symbol declared in the `.gdextension`. It lets Godot load the extension cleanly on macOS without registering any classes, so `DclSwiftLibPlugin.is_available()` returns `false` cleanly on the host.

```sh
./plugins/dcl-swift-lib/stub/build_stub.sh             # auto-detects macOS arm64
./plugins/dcl-swift-lib/stub/build_stub.sh macos.arm64 # explicit target
```

Output:

```
godot/macos/dcl-swift-lib/libdcl_swift_lib_stub.macos.arm64.dylib
```

## Using the wrapper

From GDScript:

```gdscript
if DclSwiftLibPlugin.is_available():
    print(DclSwiftLibPlugin.ping())     # -> "ok"
    print(DclSwiftLibPlugin.version())  # -> "0.1.0"
```

From Rust (`lib/src/godot_classes/dcl_swift_lib.rs`):

```rust
use crate::godot_classes::dcl_swift_lib::DclSwiftLibPlugin;

if DclSwiftLibPlugin::is_available() {
    let _ = DclSwiftLibPlugin::ping();
}
```

New methods should be added on the Swift side as `@Callable` and exposed through `DclSwiftLibPlugin` in Rust — callers should always go through the wrapper instead of `ClassDB.instantiate("DclSwiftLib")` directly.

## Updating the SwiftGodot pin

`Package.swift` pins SwiftGodot to a specific commit:

```swift
.package(
    url: "https://github.com/migueldeicaza/SwiftGodot",
    revision: "f60a71fd22f932f3eed2626e2282386f9ce7d14a"
),
```

To bump the dependency:

1. Pick the new commit on `barebone-split` (or another branch/tag).
2. Replace the `revision:` value with the new SHA.
3. Run `./plugins/build_swift_lib.sh release` and verify the smoke test still prints `ping() -> ok` on device.
