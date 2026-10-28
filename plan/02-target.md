# Target: how it will be

Three layers with one rule between them: the shell and the engine never talk to each other. Both
talk to the core, and the core holds the single app state machine.

```
Shell    React Native, TypeScript          every screen and modal, HUD over the engine surface
         + native host, Kotlin/Swift       engine view, OS intake (deep links, push, store, attest,
                                           Sentry), OS services for the core and engine (video)
   ↕ typed FFI (uniffi): TS for the screens, Kotlin/Swift for the host; callbacks for events
Core     dcl_core, Rust, no Godot          app state machine, session, attestation, realm and
                                           destination, signed requests, HTTP queue and cache policy,
                                           comms transports, friends, notifications poll, settings,
                                           feature flags, analytics, equip rules, credits, deep links
   ↕ in-process Rust calls, no FFI
Engine   dcl_engine, Rust (gdext) + Godot  scene runner, CRDT, V8, scene UI, avatars, emotes,
                                           impostors, player and camera, joystick and gamepad,
                                           content → resources, avatar preview. Modes: Idle, World
```

## Repository after

```
lib/
  Cargo.toml                 [workspace]; default-members = ["crates/dcl_engine"] so `cargo build`
                             in lib/ still produces the extension; [workspace.package] holds the
                             version xtask and rn/ read today from lib/Cargo.toml
  crates/dcl_core/           rlib. No `godot` in its Cargo.toml. Own build.rs for the protos it needs.
                             uniffi annotations behind feature "ffi". `cargo test -p dcl_core` needs
                             no engine.
  crates/dcl_engine/         [lib] name = "dclgodot", crate-type = ["cdylib", "staticlib"]. All
                             GodotClass types. Exports `gdext_rust_init` and, with "ffi", the uniffi
                             symbols. Depends on dcl_core.
godot/
  project.godot              main scene = the Rust root node; autoload list = that node only
  dclgodot.gdextension       unchanged (same library name, same entry symbol)
  assets/, shaders/, scenes  script-less .tscn, materials, fonts, translations
  addons/                    editor-only; see decisions D-open-5
  (no src/)
rn/                          the app. src/core/ is the typed FFI surface; src/godot/ keeps only the
                             engine view placeholder for no-engine builds. android/ and ios/ are the
                             host: the only native code we write, no Godot plugins (D-41)
src/                         xtask, unchanged commands
```

Deleted on the way: `godot/src/ui`, `godot/src/logic`, `godot/src/rn_bridge`, `godot/src/iap`,
`godot/src/auth`, `godot/src/config`, `godot/src/decentraland_components`, `godot/src/test`,
`godot/src/tool(s)`, `global.gd`, `main.gd`, `.gdlintrc`, the gdformat/gdlint/check-gdscript CI steps,
the sentry-godot addon, `plugins/` and the committed plugin binaries (D-41), `rn/src/GodotBridge.ts`, `rn/src/bridge/`, `rn/src/engine.ts`, `rn/src/events.ts`.

## One core per process

The core is a single instance. The engine library (`libdclgodot.so` / `.a`) statically links
`dcl_core`, and the FFI symbols the RN TurboModule binds are exported from **that same library**: on
Android the module loads `dclgodot` by name and gets the handle Godot's loader already opened; on iOS
everything is one static link. A separate `libdcl_core.so` for the FFI would give the process two
cores with two runtimes and two sessions. This is why the FFI lives in the engine crate's feature set
and not in a third shared library.

`dcl_core::Core` is created once per process (`Core::install(CoreConfig { data_dir, env, platform })`)
and reached by `Core::get()`; the engine takes an `Arc<Core>` at `Engine::start`. On desktop the Rust
root does both. On phones the host installs the core at launch, and the engine attaches to it when
Godot boots after the host created its view (D-42).

## Who starts what

- The app process starts the core at launch through the FFI, before any engine exists. Sign-in,
  session recovery, Discover, friends, notifications and settings all run with no engine.
- The core decides when the engine starts (D-42): in advance, right after the first RN screen, when
  3D is expected soon; otherwise on the first state that needs it (`AvatarPreview` or `Loading`);
  never at process launch. It lives until the process dies: Godot cannot restart in-process. On
  `engine_required` the native host creates the Godot view (Android `GodotFragment`, iOS
  `GDTViewController`); Godot boots, `DclRoot` starts the engine on Godot's thread and it attaches to
  the core. The JS side never does this. The engine renders only while a state shows it.
- Everything that must survive leaving a world lives in the core: session, settings, friends,
  notifications, comms transports.

## Engine modes

- **Idle**: no realm. Transparent viewport, optional avatar preview in a rect the shell hands to the
  core. Only preview rendering, avatar assembly and its content loading are initialized; world
  services remain absent. What the current host calls avatar-only.
- **World**: a realm and parcel loaded, opaque full-screen surface, scene UI, joystick, gamepad,
  camera. Scene runner, realm/scene fetch, SDK runtime, world comms connections and world player
  initialize on entry and release their resources on exit; core transport state survives. The HUD
  is RN above it; RN wins the hit test; the safe-area strips are reserved for RN.

Both modes are driven by core state, never by shell commands. The backpack is the single place where
the engine surface is transparent and sits above RN content. No other screen is designed around it.

## Desktop is the emulator

Desktop Godot loads the same extension. The main scene is the Rust root node: it reads the command
line (`--realm`, `--location`, `--preview <url>`, `--scene-test`, `--test-runner`), creates the core
with a guest or disposable identity, starts the engine and calls `enter_world`. No product UI, only
the scene, its UI and native input. Scene-requested actions are auto-accepted or printed. Creators
use it for preview, CI uses it for integration and scene tests, and it always runs the exact engine
that ships on phones. There is no Sentry in the emulator.

## What the Godot project becomes

A project file, script-less asset scenes, shaders, materials, fonts, translations and the extension.
The editor is used for assets and imports only. The last lint rule is: no `.gd` under `godot/src`.

## What does not change

The scene runner, V8 runtime, CRDT, SDK UI, content pipeline and comms transports stay in Rust. The
Godot fork, its version and its export pipeline stay; mobile exports are embedded in the RN app as
today (`rn/scripts/sync-godot-*.sh`, `cargo run -- export --target android|ios`). The protocol with
scenes and servers does not change; no SDK change is involved. The store identity
(`org.decentraland.godotexplorer`) stays, so the 2.0.0 release is an update of the existing app
and must find the sessions and caches the Godot-exported app wrote (see risks R-6).
