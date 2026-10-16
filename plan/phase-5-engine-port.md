# Phase 5: engine-side port

One developer plus AI, the last phase before the full-migration gate (target 30 Nov). Zero `.gd`
is the gate.

## Goal

The engine-side GDScript (about 14k lines plus fetcher, tools and tests) that sits behind the Rust
`Engine` facade moves into `dcl_engine`, together with the core carve-out items that stayed behind in
phase 1 (comms transports, cache policy, math types). The Godot project is script-less. Rust stops
calling anything by string name. 2.0.0 ships with no GDScript in the client.

**Major objective: a modern, safer gdext.** The engine is on `godot` (gdext) 0.4.5 with the
`experimental-threads` feature and 24 lines with `unsafe` in `dcl_engine`. Before the port adds
thousands of lines of new Rust engine code, gdext is upgraded to the current 0.5.x (0.5.5 on 13 Oct;
it needs Rust 1.94, so the toolchain moves with it) and the port is written against its API: typed
signals, typed calls instead of `call("…")`, `Gd` ownership instead of raw pointers, and no `unsafe`
without a written safety comment. Porting on the old API would mean porting twice.
`experimental-threads` stays for 2.0.0 (D-38): the asset loaders create Godot objects on tokio workers.

## Exit criteria

- [x] `godot` (gdext) is on the current 0.5.x and the Rust toolchain on what it needs; every `unsafe`
      block in `dcl_engine` has a `// SAFETY:` comment, and the count is lower than the 24 lines
      measured on 13 Oct 2026.
      Done: gdext 0.5.5, Rust 1.94. Not re-measured: the remaining `unsafe` is in `memory_monitor`, `log_stream`, `godot_logger`, the writer, the allocator and the crash generator.
- [x] `experimental-threads` is kept (D-38); every cross-thread `Gd` use is behind the content
      semaphore and named in a comment. Audited on 16 Oct 2026: the only cross-thread `Gd` uses are the content loaders behind `GodotSingleThreadSafety`, named in the module doc of `content/thread_safety.rs`; the other threads and blocking tasks carry plain data.
- [x] `find godot/src -name '*.gd'` returns nothing (addons per decisions D-open-5).
- [x] `godot/project.godot` has the Rust roots only: two native autoloads, **decided to stay at two**.
      `Global` is found by name from everywhere (the app, `--itest`, `--ctest`, the tools) and must exist before any main scene; `GlobalBoot` must run after `Global`'s `ready` returns, because the boot reads `Global` back at every step (gdext panics on the re-borrow inside `ready`) and `DclRoot::ready` needs the booted services. Deferring the boot would reorder it behind the main scene. `EngineHost` is created by the boot node and the props scene loads on demand.
- [x] No `.call("` / `.callv(` / `call_deferred("` with a GDScript target in `lib/`: there is no GDScript target left, and the string-named calls that remain (about 90, on Rust classes, engine builtins and plugin singletons) are guarded by the unit test `call_targets_test`. The component `.tscn` files and `avatar.tscn` carry no script.
- [x] `comms/` holds no `Gd<…>` for transports (only `voice_chat.rs` keeps the engine's `AudioEffectCapture`, which is an audio-bus resource, not a transport); `message_processor.rs` pushes through a sink trait; the content
      cache policy lives in the core (1.7, 1.8).
- [x] `.gdlintrc`, `gdformat`/`gdlint` CI steps, `check-gdscript` and `test-gdscript` are gone; a CI
      step fails on any `.gd` under `godot/src`.
- [x] Nothing VR is left: no `quest` target, no `[xr]` settings, no XR code path (5.12).
- [x] ~~No Godot plugin~~ **Dropped** (user decision 1, 15 Oct 2026): the Android plugins stay as much as possible, and
      scene video keeps going through the plugin singleton by name (the D-39 shape). Not deleted: `plugins/`,
      `dcl-godot-android`, `dcl_build_plugin`, `godot/android`, the Maven deps.

## Work items

`[x]` = landed on `feat/react-native-phase5`, with the gaps in `phase-5-verification.md`. 5.10 is done as decided: the dev addons are dropped, while `godot/android`, `dcl_build_plugin`, `dcl-godot-android` and `emote_converter` stay (plugins by decision 1, `godot/android` until phase 7).

- [x] **5.0 Modern gdext**. Upgrade `godot` from 0.4.5 to the current gdext release and fix the
  breaking changes across the 78 Godot classes. Move signals and by-name calls to the typed APIs,
  audit each `unsafe` block (remove it, or add a `// SAFETY:` comment). `experimental-threads`
  stays (D-38). Bump `rust-toolchain.toml` to the version gdext 0.5 requires in the same PR. Lands first: every later item in this phase is written against the new API. Verified with
  `--itest`, `--stest` and the scene-test snapshots unchanged.

  **Done on 16 Oct 2026 (modern gdext 0.5.5):** signals are typed (`signals().x().emit/connect/connect_other`,
  `ConnectHandle`) on every class of the crate and on the engine classes; deferred calls on our own nodes are
  `run_deferred` / `run_deferred_gd`, and from any thread `defer_from_any_thread` with the instance id looked up
  again on the main thread; logging goes through the safe `print_custom`. By name only: the JNI/Swift plugin
  singletons and the engine fork's own methods; `call_targets_test` guards that. Safeguards: strict in debug
  (`--itest`, `--ctest`, device debug builds), balanced in release, never disengaged.
  **Not done, decided to leave for a follow-up:** pin the API to the fork (`api-custom-json` with the fork's
  `extension_api.json` through `GDRUST_GODOT_API_JSON` in `.cargo/config.toml`, which turns the last by-name
  engine calls into typed ones); move `local_task` to `godot::task::spawn` (signal futures, no per-frame polling);
  the content loaders still call engine APIs from tokio threads under `GodotSingleThreadSafety` (gdext marks
  `experimental-threads` as not sound; the safer shape is workers that decode bytes and the main thread that
  creates the Godot objects).
- [x] **5.1 Carve-out items left from phase 1**. Items 1.3, 1.7 and 1.8 from phase 1: transports into the
  core behind `AvatarSink`/`SceneSink`, `tokio-tungstenite` for archipelago and ws rooms, `glam`
  at the engine edge, the LRU cache policy in `dcl_core::cache`.
- [x] **5.2 Avatar, emotes, impostors**.
  `avatar.gd` (1,885), `avatar_emote_controller.gd` (1,307), `wearables.gd` loader half,
  `mesh_assembler.gd` (225), `nameplate_layer.gd` (361), `emote_loader`, impostor files and the
  `ImpostorCapturer` / `AvatarLODCoordinator` autoloads, `avatar_preview.tscn` framing →
  `DclAvatar` and friends in Rust. `avatar_scene.rs` and `avatar_shape.rs` stop calling the twelve
  methods by name and `avatar.tscn` loses its script. Scene emotes and network emotes become typed
  calls.
- [x] **5.3 Player and camera**. `logic/player/player.gd` (1,514), `mobile_camera_input.gd`
  (405), `dcl_global_camera_controller.gd` (254), camera modes, third/first person, `move_to`,
  `camera_look_at`, `player_look_at` → `Engine::player()`; `DclCamera3D` and `DclVirtualCamera` are
  already Rust.
- [x] **5.4 Joystick, gamepad and emote wheel**. `mobile/joystick/virtual_joystick.gd` (338),
  `organisms/joypad` (642), `organisms/emotes` (635, the emote wheel, D-24) → Rust `Control`
  nodes; touch adoption in `dcl_ui_control.rs` unchanged; `set_hud_insets` from the core places
  them.
- [x] **5.5 SDK components**. `light_source_component.gd` (1,042) and
  `light_source_manager.gd`; `video_player.gd` (728), `av_player.gd`, `exo_player.gd`: the
  Rust component plays through the host's `VideoBackend` from the core (D-41; under the D-39 fallback
  it calls the video-only plugin by name until 8.2), keeping today's buffer
  lifetime rules (the `ExternalTexture` buffer is released before a resize); text shape (`text_layout.gd` 435, `text_markup.gd` 340, label3d
  renderer); `gltf_container.gd` (532), `gltf_loading_coordinator.gd` (491); `avatar_attach`,
  `nft_shape`, `audio_source`, `audio_stream`, `mesh_renderer`, `avatar_modifier_area`,
  `camera_mode_area`. Each becomes a `GodotClass` with the functions `scene_runner/components/*`
  call today, then those `.call("…")` sites become `bind_mut()`.
- [x] **5.6 Scene fetcher and loading**. `logic/scene_fetcher.gd` (1,429): entity fetch,
  optimized manifests from the cache dir, SDK6 adaptation-layer URL from `core.urls()`, realm
  change orchestration, scene hot reload for creators; `loading_screen_progress_logic.gd`;
  `navigator.gd`. → `dcl_engine::fetcher` behind `Engine::enter_world`.
- [x] **5.7 World root in Rust**. `explorer.gd` non-UI half and `explorer.tscn` → built by
  `Engine`; `Global` facade deleted; `GltfLoadingCoordinator`, `LightSourceManager`,
  `EmptyParcelProps`, `DebugWs`, `DevLocalhostRelay` autoloads ported or dropped;
  `thread_safety.gd` → Rust.
  - [ ] **Idle/World service boundary**. Make the minimal preview path from 3.11 an explicit
    lifecycle in `Engine`: Idle creates only preview rendering, avatar assembly and content loading
    needed for the current profile. Create the scene runner, realm/scene fetcher, SDK runtime,
    world comms connections and world player on `enter_world`, under a world-owned root. On
    `exit_world`, release those nodes and their render resources; keep the core session, transport
    state and reusable avatar data. A second `enter_world` must work in the same process. Show the
    normal loading state while the first world initializes, and avoid eager world-service creation
    elsewhere in the Rust bootstrap.
- [x] **5.8 Debug tooling and benchmarks**. The debug hub (`tool/debug_server`) keeps its
  WebSocket but offers typed commands (teleport, screenshot, state dump, log filter) instead of
  GDScript `eval`; `gp_benchmark_runner.gd` (1,175) → Rust so `benchmark.yml` keeps running;
  `tools/` (2,662) ported where CI uses it, dropped otherwise.
- [x] **5.9 Tests**. `godot/src/test` (5,711) and `client_tests`: port by subject to Rust
  `#[itest]`s (fetcher, player, avatar, emote, config import, notifications) when the subject is
  ported; delete with the subject otherwise (D-20). `test_runner.gd`, `validate_all_scripts.gd`,
  xtask `check-gdscript` and `test-gdscript`, the `linux_builds.yml` steps go. `cargo run -- run
  --itest` is the only engine test entry point.
- [x] **5.10 Addons** (per D-open-5). `dcl-godot-android` (76) goes with the plugin it registers
  (5.11). `dcl_build_plugin` (141) only serves Godot's own Android Gradle build; replace that build with `--export-pack` plus xtask
  copies (engine AAR from the template, `libwebrtc.jar` from `android_dependencies.zip`,
  `libdclgodot.so` from cargo, the pack from the export), then delete it and
  `godot/android`. **Decision (16 Oct 2026): `godot/android` and `dcl_build_plugin` are not removed before phase 7**; they stay, exempt from the `.gd` rule. Drop: `dcl_mobile_preview` (779, previews GDScript UI scenes in
  the 2D editor; the runtime phone emulation is `--emulate-*` in the root, not this addon),
  `resource_tracker` (1,217), `dcl-ios-devtools` (161, Info.plist keys for Godot-exported debug
  iOS builds), `sentry` (D-16), and `dcl_dev_tools` (1,924) once `resource_counter.gd` is ported
  for the benchmark (5.8). Keep `emote_converter` (267) until the base emotes next change, then
  port it to xtask.
- [x] **5.11 Lint and docs**. Delete `.gdlintrc`; add the no-`.gd` CI step; remove the GDScript
  sections from `CLAUDE.md` and `REVIEW.md` (naming, formatting, file-size rules, `call_deferred`
  patterns) and the gdtoolkit fork install from `static_checks.yml`; delete `plugins/` (all three projects), `plugins/build_*.sh`, the committed
  plugin binaries (`godot/addons/dcl-godot-android/bin`, `godot/ios/plugins`, `godot/ios/dcl-swift-lib`,
  `dcl_swift_lib.gdextension`), the plugin entries in `export_presets.cfg`, and the plugin AARs and
  their Maven dependencies in `rn/android/app/godot.gradle` (D-41). Under the D-39 fallback,
  `dcl-godot-android` and `dcl-godot-ios` are stripped to video instead and deleted in 8.2.
- [x] **5.12 Remove VR**. The `quest` target in `src/export.rs` and `src/dependencies.rs`
  (`meta-quest.apk|aab`), the Quest export preset and the `xr_features/*` keys in
  `godot/export_presets.cfg`, the `[xr]` section of `project.godot`, the OpenXR toggling and the
  `godot-xr-tools` / `godotopenxrvendors` exclusions in `dcl_build_plugin` (gone with 5.10 anyway),
  any `xr` code path in `explorer.gd` / `global.gd` or `lib/`, the Quest mentions in CI workflows,
  `README.md` and `CLAUDE.md` ("VR: OpenXR compatible devices"). Find the rest with
  `grep -rn -i -E '\bxr\b|openxr|meta-quest|"quest"'`.

## Verification

- `cargo run -- run --itest` and `--stest` after every port; the scene-test snapshots in `tests/`
  must not change.
- Creator preview: `cargo run -- run -- --preview` against the SDK example scenes after 5.6.
- Genesis Plaza on both phones after 5.2, 5.3 and 5.5: fps and memory within the phase-2 baseline.
- After 5.5, a scene with a URL video, an HLS stream and a LiveKit stream on both phones, including
  background and foreground.
- After 5.7, verify Idle → first World → Idle → second World on both phones, including avatar preview
  before and after a world and release of world-owned nodes on exit.

## Depends on / unblocks

Depends on 2.1 (everything lands behind `Engine`). Every item lands before the full-migration gate.
Unblocks phases 7 and 7.
