# Phase 2: Rust root node and the emulator

Side-project, Oct–Nov 2026. The engine API becomes real.

## Goal

`main.gd` and the composition in `global.gd._ready` are replaced by a Rust root node that boots from
the command line, creates the core and calls `enter_world`. Scene-requested actions go through the
core. This is the desktop emulator from day one and it forces `03-engine-api.md` to exist as a type.

`global.gd` survives as a shrinking facade (`Global.scene_runner`, `Global.config`, …) for the
engine-side GDScript that phase 5 ports; it stops creating services and stops being the root.

## Exit criteria

- [ ] `godot/src/main.gd` and `main.tscn` are deleted; `run/main_scene` is a `.tscn` whose root is
      the Rust `DclRoot` node and nothing else.
- [ ] `cargo run -- run --itest` and `cargo run -- run --stest` pass on the Rust root.
- [ ] `cargo run -- run -- --preview http://localhost:8000` renders a local scene with no GDScript
      in the boot path; `--realm … --location …` enters Genesis Plaza as a guest.
- [ ] `grep -rn '"/root/' lib/crates/dcl_engine/src` finds nothing.
- [ ] `handle_restricted_actions.rs` emits `scene_request` events and calls no GDScript.
- [ ] Soft keyboard reaches a scene `UiInput` on Android and iOS in the RN host (risks R-3).
- [ ] Release-build fps and memory measured on the QA phone, RN host vs Godot-exported app (risks R-1).

## Work items

- [ ] **2.1 `dcl_engine::Engine` facade** (L). The calls and events of `03-engine-api.md` as a
  struct the root owns: `start`, `enter_world`, `exit_world`, `teleport`, `set_self_profile`,
  `play_emote`, `show/hide_avatar_preview`, `apply_graphics`, `apply_audio`, `set_hud_insets`,
  lifecycle. Internally it still instantiates `explorer.tscn` for World and a transparent viewport
  with `avatar_preview.tscn` for Idle (what `rn_avatar_host.tscn` + `rn_backpack.gd` do), so no
  engine-side GDScript is ported here. The event sink is a trait the core implements. Core → engine
  calls are queued and drained in the root's `process`.
- [ ] **2.2 `DclRoot` as the main scene** (M). Reads `DclCli`, `Core::install`, `Engine::start`,
  then `enter_world` from `--realm/--location/--preview` or the test-runner flags. The twenty-odd
  services `global.gd` created and added under `/root` are created by the core (session, comms,
  metrics, …) or by the engine (scene runner, avatars, content provider, …) and exposed to the
  remaining GDScript through the `Global` facade. Autoloads that only the GDScript UI needed stay
  until phase 4 deletes their users.
- [ ] **2.3 Scene-requested actions through the core** (M). `handle_restricted_actions.rs` → `EngineEvent::SceneRequest` → `dcl_core::scene_requests` (policy, pending map, `answer`). The
  emulator sink auto-accepts teleport and realm changes and prints external URLs and NFT opens. The
  `nft_dialog.tscn` load from Rust goes. `move_player_to`, `camera_look_at` and the emote RPCs use
  `Engine` handles instead of `/root/explorer` and `/root/explorer/world/Player/Avatar`.
- [ ] **2.4 Node paths** (S). `global_get_node_helper.rs` (4 paths), `dcl_ui_control.rs`
  (`MobileCameraInput`), `dcl_global.rs` (`explorer`, `Global`) → handles owned by `Engine`.
- [ ] **2.5 Test runners on the root** (M). `--test-runner` (itest), `--scene-test`, client tests and
  the renderer tools (`avatar_renderer_mode`, `asset_renderer_mode`, `emote_test_mode`,
  `--lighting-renderer`, `asset_server`) that `main.gd` routed become `DclRoot` modes. The GDScript
  `test_runner.gd` is still invoked for the GDScript suite until phase 5.8.
- [ ] **2.6 Emulator CLI and docs** (S). `--realm`, `--location`, `--preview <url>`, `--scene-test`,
  `--rust-log`, graphics preset; `--skip-lobby`, `--guest-profile`, `--rn-host`, `--rn-avatar-only`
  disappear (there is no lobby to skip and no mode to switch). `docs/MOBILE_PREVIEW.md` and the
  README's run section updated; `--emulate-ios/--emulate-android` keep only their window size role.
- [ ] **2.7 Boot-arg parity in the hosts** (S). Android `BOOT_ARGS` and the iOS `GodotHost.mm` arg
  list become the same minimal set; the engine starts Idle on both. (Phase 3 removes the
  JSON-driven mode switch entirely.)
- [ ] **2.8 IME check** (S). A test scene with a `UiInput`; verify focus and the soft keyboard on
  both phones under the RN overlay. Record the result in `risks.md` R-3.
- [ ] **2.9 Measure the RN host** (S). Release build, Genesis Plaza, Moto Edge 60 Pro and an
  iPhone: fps, RSS, startup. Same scene in the Godot-exported app from the same commit. Record in
  `risks.md` R-1.

## Verification

- `cargo run -- run --itest` / `--stest` on every PR (now on the Rust root).
- `cargo run -- run -- --preview` against a local SDK scene with `UiInput`, `teleportTo`,
  `openExternalUrl` to exercise 2.3.
- Phase 1's phone regression still passes: the lobby still exists on the branch at this point; the
  Rust root loads it when no `--realm`/`--preview` is given and the host is not the emulator.

## Depends on / unblocks

Depends on 1.1 (`Core`) and 1.7 (comms sinks, for `exit_world` to keep transports alive). Unblocks
phase 3 (the host attaches `Engine` to the core) and phase 5 (every port lands behind `Engine`).
