# Phase 2: Rust root node and the emulator

The engine API becomes real.

## Goal

`main.gd` and the composition in `global.gd._ready` are replaced by a Rust root node that boots from
the command line, creates the core and calls `enter_world`. Scene-requested actions go through the
core. This is the desktop emulator from day one and it forces `03-engine-api.md` to exist as a type.

`global.gd` survives as a shrinking facade (`Global.scene_runner`, `Global.config`, …) for the
engine-side GDScript that phase 5 ports; it stops creating services and stops being the root.

## Exit criteria

- [x] `godot/src/main.gd` and `main.tscn` are deleted; `run/main_scene` is `dcl_root.tscn`, one node of the
      Rust `DclRoot` class.
- [x] `cargo run -- run --itest` (29) passes on the Rust root, and `test-gdscript` (16). `--stest` was not run:
      it needs a scene-test input and the content servers; the scene-test boot itself was.
- [x] `cargo run -- run -- --realm … --location …` and `-- --preview <url>` enter the world as a throwaway guest,
      no lobby (checked against Genesis Plaza). **Reworded:** "no GDScript in the boot path" is not true yet:
      `Global._ready` (the autoload before the root), the lobby and the explorer are still GDScript until
      phases 4/5. What is true: routing is Rust and `main.gd` is gone.
- [x] `grep -rn '"/root/' lib/crates/dcl_engine/src` finds nothing.
- [x] `handle_restricted_actions.rs` builds `SceneRequest`s and hands them to `DclEngine::scene_request`; the
      only GDScript it still reaches goes through the engine handles (move/look/emote) and the Godot-UI sink.
- [x] (Android only, 19 Oct: bottom input focuses and shows the keyboard, typing updates `value:`, the action key sets `submitted: hello`, the keyboard closes and the scene shifts above it; iOS and the rotate / RN-`TextInput` rows were not run) Soft keyboard reaches a scene `UiInput` on Android and iOS in the RN host (risks R-3): scene and checklist in
      `tests/ime-uiinput-scene/`, **not run** — needs the phones.
- [ ] Release-build fps and memory measured on the QA phone, RN host vs Godot-exported app (risks R-1): `--perf-log`
      and the runbook in `scripts/bench/rn-vs-godot.md`, **not run** — needs the phones.

## Work items

- [x] **2.1 `dcl_engine::Engine` facade — reduced.** `DclEngine` is a persistent node under `/root`
  (`Global.engine`) that owns the world handles (explorer, player, avatar, camera input, dialog stack; attached by
  `explorer.gd`, dropped on `tree_exiting`) and the typed calls `teleport`, `play_local_emote`, `set_hud_insets`,
  `move_player_to`, look-at, emote start/stop/register, `ui_has_focus`, `set_emulator_mode`,
  `answer_scene_request`. The RN bridge calls them. **Not done:** `start`, `enter_world`/`exit_world`,
  `set_self_profile`, `show/hide_avatar_preview`, `apply_graphics/audio`, lifecycle, the command queue and the
  event sink for loading/player events — they have no engine-side owner yet (the logic is in `rn_world.gd`,
  `rn_backpack.gd` and `rn_req_settings.gd`) and move with the FFI in phase 3 and the port in phase 5.
- [x] **2.2 `DclRoot` as the main scene.** Replaces `main.gd`: startup display/audio config, then the routing.
  Services are created in Rust, from inside `Global._ready` (autoload order matters: `SessionTracker` and `Iap`
  read `Global.metrics` in their own `_ready`): `DclBootstrap.create_core_services`, `create_app_services`,
  `create_extra_services` and `start_special_profile` replace ~190 lines of `global.gd`, from a `ServiceSpec`
  table. About twenty of the services are still GDScript classes instantiated from Rust by script path.
- [x] **2.3 Scene-requested actions through the core.** `dcl_core::scene_requests` (`SceneRequest`, pending map,
  ids) and `engine/scene_requests.rs` (`SceneRequestSink`, `GodotUiSink`, `EmulatorSink`). **Behaviour kept:**
  the Godot sink still answers the scene's RPC `Ok` when the modal is shown; suspend-until-answered waits for
  RN rendering the modals (phase 3). The `nft_dialog.tscn` load moved into the sink.
- [x] **2.4 Node paths.** `global_get_node_helper.rs` deleted; `MobileCameraInput`, `ui_has_focus` and the
  avatar/explorer lookups use `DclEngine`.
- [x] **2.5 Test runners on the root.** `boot_mode` is a pure, unit-tested function; every mode `main.gd` routed
  is a `DclRoot` mode, plus the new emulator mode. The GDScript `test_runner.gd` is still started by `Global`.
- [x] **2.6 Emulator CLI and docs** — flags kept as aliases. `--realm`/`--preview` without a host start the
  emulator; `--preview <url>` names the realm when `--realm` is absent; `--rn-avatar-only`, `--lighting-renderer`
  and `--perf-log` are registered; `--emulate-*` documented as window size. README, `docs/MOBILE_PREVIEW.md`,
  `rn/README.md` updated. **Deferred to phase 3:** removing `--skip-lobby`, `--guest-profile`, `--rn-host`,
  `--rn-avatar-only` — the lobby still exists on this branch and the hosts still pass the last two.
- [x] **2.7 Boot-arg parity in the hosts** — partly. iOS now starts `--rn-host --rn-avatar-only` (Idle) like
  Android. Android additionally passes `--force-mobile --no-pulse`; whether iOS should is an open question tied
  to 2.8 (`--force-mobile` skips the virtual-keyboard height poll). iOS change not built here.
- [ ] **2.8 IME check.** Test scene and checklist written (`tests/ime-uiinput-scene/`); the run on both phones
  and the R-3 entry are yours.
- [ ] **2.9 Measure the RN host.** `--perf-log` (engine logs `[Perf] fps=… rss_mb=…` every 5 s while in a world,
  same line on logcat and the iOS console) and `scripts/bench/rn-vs-godot.md`; the runs and the R-1 entry are
  yours.

## Verification

- `cargo run -- run --itest` / `--stest` on every PR (now on the Rust root).
- `cargo run -- run -- --preview` against a local SDK scene with `UiInput`, `teleportTo`,
  `openExternalUrl` to exercise 2.3.
- Phase 1's phone regression still passes: the lobby still exists on the branch at this point; the
  Rust root loads it when no `--realm`/`--preview` is given and the host is not the emulator.

## Depends on / unblocks

Depends on 1.1 (`Core`). Unblocks phase 3 (the host attaches `Engine` to the core) and phase 5
(every port lands behind `Engine`).
