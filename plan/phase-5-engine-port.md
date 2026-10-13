# Phase 5: engine-side port

One developer plus AI, the last phase before the full-migration gate (target 30 Nov). Zero `.gd`
is the gate.

## Goal

The engine-side GDScript (about 14k lines plus fetcher, tools and tests) that sits behind the Rust
`Engine` facade moves into `dcl_engine`, together with the core carve-out items that stayed behind in
phase 1 (comms transports, cache policy, math types). The Godot project is script-less. Rust stops
calling anything by string name. 2.0.0 ships with no GDScript in the client.

**Major objective: a modern, safer gdext.** The engine is on `godot` (gdext) 0.4.5 with the
`experimental-threads` feature and 32 `unsafe` lines in `dcl_engine`. Before the port adds
thousands of lines of new Rust engine code, gdext is upgraded to its current release (5.0) and the
port is written against its modern API: typed signals, typed calls instead of `call("…")`, `Gd`
ownership instead of raw pointers, and no `unsafe` without a written safety comment. Porting
on the old API would mean porting twice.

## Exit criteria

- [ ] `godot` (gdext) is on its current release; every `unsafe` block in `dcl_engine` has a
      `// SAFETY:` comment, and the count is lower than the 32 lines measured on 13 Oct 2026.
- [ ] `experimental-threads` is removed, or kept only where a documented cross-thread `Gd` use
      needs it.
- [ ] `find godot/src -name '*.gd'` returns nothing (addons per decisions D-open-5).
- [ ] `godot/project.godot` has one autoload, the Rust root, or none.
- [ ] No `.call("` / `.callv(` / `call_deferred("` with a GDScript target in `lib/`; the eleven
      component `.tscn` files and `avatar.tscn` carry no script.
- [ ] `comms/` holds no `Gd<…>`; `message_processor.rs` pushes through a sink trait; the content
      cache policy lives in the core (1.7, 1.8).
- [ ] `.gdlintrc`, `gdformat`/`gdlint` CI steps, `check-gdscript` and `test-gdscript` are gone; a CI
      step fails on any `.gd` under `godot/src`.
- [ ] Nothing VR is left: no `quest` target, no `[xr]` settings, no XR code path (5.12).

## Work items

- [ ] **5.0 Modern gdext**. Upgrade `godot` from 0.4.5 to the current gdext release and fix the
  breaking changes across the 78 Godot classes. Move signals and by-name calls to the typed APIs,
  audit each `unsafe` block (remove it, or add a `// SAFETY:` comment), and drop
  `experimental-threads` where the thread use can go through the core or a channel instead.
  Lands first: every later item in this phase is written against the new API. Verified with
  `--itest`, `--stest` and the scene-test snapshots unchanged.
- [ ] **5.1 Carve-out items left from phase 1**. Items 1.3, 1.7 and 1.8 from phase 1: transports into the
  core behind `AvatarSink`/`SceneSink`, `tokio-tungstenite` for archipelago and ws rooms, `glam`
  at the engine edge, the LRU cache policy in `dcl_core::cache`.
- [ ] **5.2 Avatar, emotes, impostors**.
  `avatar.gd` (1,885), `avatar_emote_controller.gd` (1,307), `wearables.gd` loader half,
  `mesh_assembler.gd` (225), `nameplate_layer.gd` (361), `emote_loader`, impostor files and the
  `ImpostorCapturer` / `AvatarLODCoordinator` autoloads, `avatar_preview.tscn` framing →
  `DclAvatar` and friends in Rust. `avatar_scene.rs` and `avatar_shape.rs` stop calling the twelve
  methods by name and `avatar.tscn` loses its script. Scene emotes and network emotes become typed
  calls.
- [ ] **5.3 Player and camera**. `logic/player/player.gd` (1,514), `mobile_camera_input.gd`
  (405), `dcl_global_camera_controller.gd` (254), camera modes, third/first person, `move_to`,
  `camera_look_at`, `player_look_at` → `Engine::player()`; `DclCamera3D` and `DclVirtualCamera` are
  already Rust.
- [ ] **5.4 Joystick, gamepad and emote wheel**. `mobile/joystick/virtual_joystick.gd` (338),
  `organisms/joypad` (642), `organisms/emotes` (635, the emote wheel, D-24) → Rust `Control`
  nodes; touch adoption in `dcl_ui_control.rs` unchanged; `set_hud_insets` from the core places
  them.
- [ ] **5.5 SDK components**. `light_source_component.gd` (1,042) and
  `light_source_manager.gd`; `video_player.gd` (728), `av_player.gd`, `exo_player.gd` (the
  ExoPlayer plugin stays); text shape (`text_layout.gd` 435, `text_markup.gd` 340, label3d
  renderer); `gltf_container.gd` (532), `gltf_loading_coordinator.gd` (491); `avatar_attach`,
  `nft_shape`, `audio_source`, `audio_stream`, `mesh_renderer`, `avatar_modifier_area`,
  `camera_mode_area`. Each becomes a `GodotClass` with the functions `scene_runner/components/*`
  call today, then those `.call("…")` sites become `bind_mut()`.
- [ ] **5.6 Scene fetcher and loading**. `logic/scene_fetcher.gd` (1,429): entity fetch,
  optimized manifests from the cache dir, SDK6 adaptation-layer URL from `core.urls()`, realm
  change orchestration, scene hot reload for creators; `loading_screen_progress_logic.gd`;
  `navigator.gd`. → `dcl_engine::fetcher` behind `Engine::enter_world`.
- [ ] **5.7 World root in Rust**. `explorer.gd` non-UI half and `explorer.tscn` → built by
  `Engine`; `Global` facade deleted; `GltfLoadingCoordinator`, `LightSourceManager`,
  `EmptyParcelProps`, `DebugWs`, `DevLocalhostRelay` autoloads ported or dropped;
  `thread_safety.gd` → Rust.
- [ ] **5.8 Debug tooling and benchmarks**. The debug hub (`tool/debug_server`) keeps its
  WebSocket but offers typed commands (teleport, screenshot, state dump, log filter) instead of
  GDScript `eval`; `gp_benchmark_runner.gd` (1,175) → Rust so `benchmark.yml` keeps running;
  `tools/` (2,662) ported where CI uses it, dropped otherwise.
- [ ] **5.9 Tests**. `godot/src/test` (5,711) and `client_tests`: port by subject to Rust
  `#[itest]`s (fetcher, player, avatar, emote, config import, notifications) when the subject is
  ported; delete with the subject otherwise (D-20). `test_runner.gd`, `validate_all_scripts.gd`,
  xtask `check-gdscript` and `test-gdscript`, the `linux_builds.yml` steps go. `cargo run -- run
  --itest` is the only engine test entry point.
- [ ] **5.10 Addons** (per D-open-5). `dcl_build_plugin` (141) and `dcl-godot-android` (76) only
  serve Godot's own Android Gradle build; replace that build with `--export-pack` plus xtask
  copies (engine AAR from the template, `libwebrtc.jar` from `android_dependencies.zip`,
  `libdclgodot.so` from cargo, the pack from the export), then delete both addons and
  `godot/android`. Until then they are exempt from the `.gd` rule. Drop: `dcl_mobile_preview` (779, previews GDScript UI scenes in
  the 2D editor; the runtime phone emulation is `--emulate-*` in the root, not this addon),
  `resource_tracker` (1,217), `dcl-ios-devtools` (161, Info.plist keys for Godot-exported debug
  iOS builds), `sentry` (D-16), and `dcl_dev_tools` (1,924) once `resource_counter.gd` is ported
  for the benchmark (5.8). Keep `emote_converter` (267) until the base emotes next change, then
  port it to xtask.
- [ ] **5.11 Lint and docs**. Delete `.gdlintrc`; add the no-`.gd` CI step; remove the GDScript
  sections from `CLAUDE.md` and `REVIEW.md` (naming, formatting, file-size rules, `call_deferred`
  patterns) and the gdtoolkit fork install from `static_checks.yml`; prune `dcl-godot-android` and
  `DclGodotiOS` to what the engine still needs.
- [ ] **5.12 Remove VR**. The `quest` target in `src/export.rs` and `src/dependencies.rs`
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

## Depends on / unblocks

Depends on 2.1 (everything lands behind `Engine`). Every item lands before the full-migration gate.
Unblocks phases 7 and 7.
