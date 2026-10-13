# Phase 5: engine-side port

December 2026, full team; what does not fit ships after the January release. 5.1 starts early.

## Goal

The engine-side GDScript (about 14k lines plus fetcher, tools and tests) moves into `dcl_engine`.
The Godot project is script-less. The desktop build is the emulator. Rust stops calling anything by
string name.

## Exit criteria

- [ ] `find godot/src -name '*.gd'` returns nothing (addons per decisions D-open-5).
- [ ] `godot/project.godot` has one autoload, the Rust root, or none.
- [ ] No `.call("` / `.callv(` / `call_deferred("` with a GDScript target in `lib/`; the eleven
      component `.tscn` files and `avatar.tscn` carry no script.
- [ ] `.gdlintrc`, `gdformat`/`gdlint` CI steps, `check-gdscript` and `test-gdscript` are gone; a CI
      step fails on any `.gd` under `godot/src`.
- [ ] `cargo run -- export --target linux|macos|windows` exports the emulator.

What ships in January: 5.1 (the backpack preview needs it) and whatever else lands by the release
branch cut. The rest removes debt inside the engine and does not block a user-facing feature.

## Work items

- [ ] **5.1 Avatar, emotes, impostors** (XL, first). `avatar.gd` (1,885),
  `avatar_emote_controller.gd` (1,307), `wearables.gd` loader half, `mesh_assembler.gd` (225),
  `nameplate_layer.gd` (361), `emote_loader`, impostor files and the `ImpostorCapturer` /
  `AvatarLODCoordinator` autoloads, `avatar_preview.tscn` framing → `DclAvatar` and friends in Rust.
  `avatar_scene.rs` and `avatar_shape.rs` stop calling the twelve methods by name and `avatar.tscn`
  loses its script. Scene emotes (`register_scene_emote_content`, `stop_emote_from_scene`) and
  network emotes become typed calls.
- [ ] **5.2 Player and camera** (L). `logic/player/player.gd` (1,514), `mobile_camera_input.gd`
  (405), `dcl_global_camera_controller.gd` (254), camera modes, third/first person, `move_to`,
  `camera_look_at`, `player_look_at` → `Engine::player()`; `DclCamera3D` and `DclVirtualCamera` are
  already Rust.
- [ ] **5.3 Joystick, gamepad and emote wheel** (M). `mobile/joystick/virtual_joystick.gd` (338),
  `organisms/joypad` (642), `organisms/emotes` (635, the emote wheel, D-24) → Rust `Control`
  nodes; touch adoption in `dcl_ui_control.rs` unchanged; `set_hud_insets` from the core places
  them.
- [ ] **5.4 SDK components** (XL, one PR per component). `light_source_component.gd` (1,042) and
  `light_source_manager.gd`; `video_player.gd` (728), `av_player.gd`, `exo_player.gd` (the
  ExoPlayer plugin stays); text shape (`text_layout.gd` 435, `text_markup.gd` 340, label3d
  renderer); `gltf_container.gd` (532), `gltf_loading_coordinator.gd` (491); `avatar_attach`,
  `nft_shape`, `audio_source`, `audio_stream`, `mesh_renderer`, `avatar_modifier_area`,
  `camera_mode_area`. Each becomes a `GodotClass` with the functions `scene_runner/components/*`
  call today, then those `.call("…")` sites become `bind_mut()`.
- [ ] **5.5 Scene fetcher and loading** (L). `logic/scene_fetcher.gd` (1,429): entity fetch,
  optimized manifests from the cache dir, SDK6 adaptation-layer URL from `core.urls()`, realm
  change orchestration, scene hot reload for creators; `loading_screen_progress_logic.gd`;
  `navigator.gd`. → `dcl_engine::fetcher` behind `Engine::enter_world`.
- [ ] **5.6 World root in Rust** (L). `explorer.gd` (1,893) non-UI half and `explorer.tscn` →
  built by `Engine`; `Global` facade deleted; `GltfLoadingCoordinator`, `LightSourceManager`,
  `EmptyParcelProps`, `DebugWs`, `DevLocalhostRelay` autoloads ported or dropped;
  `thread_safety.gd` → Rust.
- [ ] **5.7 Debug tooling and benchmarks** (M). The debug hub (`tool/debug_server`) keeps its
  WebSocket but offers typed commands (teleport, screenshot, state dump, log filter) instead of
  GDScript `eval`; `gp_benchmark_runner.gd` (1,175) → Rust so `benchmark.yml` keeps running;
  `tools/` (2,662) ported where CI uses it, dropped otherwise.
- [ ] **5.8 Tests** (L). `godot/src/test` (5,711) and `client_tests`: port by subject to Rust
  `#[itest]`s (fetcher, player, avatar, emote, config import, notifications) when the subject is
  ported; delete with the subject otherwise. `test_runner.gd`, `validate_all_scripts.gd`, xtask
  `check-gdscript` and `test-gdscript`, the `linux_builds.yml` steps go. `cargo run -- run --itest`
  is the only engine test entry point.
- [ ] **5.9 Addons** (per D-open-5). Keep as export tooling, exempt from the `.gd` rule:
  `dcl_build_plugin` (141, XR mode and Android deps at export) and `dcl-godot-android` (76,
  registers the Android plugin). Drop: `dcl_mobile_preview` (779, previews GDScript UI scenes in
  the 2D editor; the runtime phone emulation is `--emulate-*` in the root, not this addon),
  `resource_tracker` (1,217), `dcl-ios-devtools` (161, Info.plist keys for Godot-exported debug
  iOS builds), `sentry` (D-16), and `dcl_dev_tools` (1,924) once `resource_counter.gd` is ported
  for the benchmark (5.7). Keep `emote_converter` (267) until the base emotes next change, then
  port it to xtask.
- [ ] **5.10 Desktop build is the emulator** (S). Export presets, `README.md`, `docs/MOBILE_PREVIEW.md`,
  `docs/scene-architecture.md` updated; `--emulate-ios/android` reduced to window sizes.
- [ ] **5.11 Lint and docs** (S). Delete `.gdlintrc`; add the no-`.gd` CI step; remove the GDScript
  sections from `CLAUDE.md` and `REVIEW.md` (naming, formatting, file-size rules, `call_deferred`
  patterns) and the gdtoolkit fork install from `static_checks.yml`; prune `dcl-godot-android` and
  `DclGodotiOS` to what the engine still needs.

## Verification

- `cargo run -- run --itest` and `--stest` after every port; the scene-test snapshots in `tests/`
  must not change.
- Creator preview: `cargo run -- run -- --preview` against the SDK example scenes after 5.5.
- Genesis Plaza on both phones after 5.1, 5.2 and 5.4: fps and memory within the phase-2 baseline.

## Depends on / unblocks

Depends on 2.1 (everything lands behind `Engine`). 5.1 is needed by 4.3 for preview quality. Closes
the project: zero `.gd` is the milestone.
