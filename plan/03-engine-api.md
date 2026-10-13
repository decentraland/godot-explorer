# The engine API (core ↔ engine)

Today the contract between app and engine is implicit: a composition root in GDScript, six hardcoded
`/root/…` paths and about 40 methods Rust calls by string name (list in `01-today.md`). Writing it down is
the first engine task because the emulator, the core and the shell all depend on it. It becomes a
Rust type in phase 2 (`dcl_engine::Engine`) and is finished when phase 7 removes the last by-name call.

Threading: the engine is driven on Godot's main thread. Core → engine calls are enqueued and drained
in the root node's `process` (the pattern `dcl_tokio_rpc.rs` uses today). Engine → core calls are
plain calls into a thread-safe core. The comms position hop (core transports → engine avatars) is an
in-process sink call, not an FFI, and stays per-frame cheap.

## Core → engine

| Call | What the engine does |
|---|---|
| `start(core, GraphicsPreset, AudioSettings)` | Once per process. Creates the world root, registers its event sink with the core, enters Idle. |
| `enter_world(RealmAbout, Parcel, Identity, Profile)` | Idle → World. Realm change, scene fetch, loading episode. Replaces `rn_world.gd async_enter` and `explorer.gd` realm handling. |
| `exit_world()` | World → Idle. Tears scenes and comms fan-out down; the core keeps transports and session. Replaces `rn_world.gd async_exit`. |
| `teleport(Parcel)` / `change_realm(RealmAbout, Parcel)` | In World. Replaces `navigator.gd`. |
| `set_self_profile(Avatar)` | Re-assembles the local avatar; used by backpack equip and profile edit. |
| `play_emote(urn)` / `stop_emote()` | Local avatar. |
| `show_avatar_preview(rect_px)` / `hide_avatar_preview()` | Idle only. Replaces `backpack_open/place/close` and `rn_backpack.gd`. |
| `set_preview_focus(target)` | Replaces `focus_camera`. |
| `apply_graphics(GraphicsPreset)` / `apply_audio(AudioSettings)` | Settings changed in the core. |
| `set_hud_insets(top, right, bottom, left)` | Where RN controls sit; scene UI and joystick avoid them. |
| `on_background()` / `on_foreground()` / `on_low_memory()` | OS lifecycle relayed by the host through the core. |
| `set_log_filter(str)` | Today `DclGlobal.set_rust_log_filter`. |

## Engine → core (events)

| Event | Source today |
|---|---|
| `loading_started(session, expected)`, `loading_progress(percent, ready, total, mbps)`, `loading_finished`, `loading_timeout` | `scene_manager.rs` signals + `loading_screen.gd` |
| `scene_crashed(scene, report)`, `scene_killed(scene)` | `scene_manager.rs` |
| `player_moved(position, parcel)`, `parcel_changed(parcel, scene)` | `state` event at 4 Hz in `rn_bridge.gd` |
| `pointer_tooltip_changed(list)` | `scene_manager.rs` |
| `thermal_or_fps_cap_changed(preset)` | `dcl_dynamic_graphics_manager.rs` |
| `hardware_benchmark_result(report)` | `hardware_benchmark` via `global.gd` |
| `text_input_focus(bool, height)` | `dcl_ui_input.rs` / `global.gd` virtual keyboard tracking |
| `avatar_profile_needed(address)` | `message_processor.rs` profile request state |
| `scene_request(id, SceneRequest)` | see below |

### Scene-requested actions

`SceneRequest` is `OpenExternalUrl{url}`, `Teleport{parcel, realm?}`, `ChangeRealm{realm, parcel?}`,
`OpenNft{urn}`. The engine emits the request and suspends the RPC; the core applies policy (allow-list,
guest rules, rate limits) and asks the shell to render the modal; the shell answers through the core
(`answer_scene_request(id, accepted)`); the core resumes the RPC. The emulator answers with auto-accept
for teleport and realm changes and prints external URLs and NFTs. `move_player_to`, `camera_look_at`
and emote RPCs stay inside the engine: no modal, no core policy, but they go through `Engine` handles
instead of `/root/explorer` paths.

## Who holds what

- The engine owns: scene tree, scenes, avatars (remote and local), player, camera, scene UI, joystick,
  gamepad and the emote wheel (D-24), loading orchestration, resource loading and the content cache
  *consumers*.
- The core owns: identity, profile, realm and destination, comms transports, friends, blacklist,
  settings and graphics presets, feature flags, analytics, the content cache *policy* and data dir.
- The engine reads core state through `Arc<Core>` (identity for signed fetches, blacklist for hiding
  avatars, settings for graphics). It never writes core state; it emits events.

## From by-name calls to the API

| Today | Target |
|---|---|
| `global_get_node_helper.rs` paths | handles owned by `Engine` (world root, player, avatar, dialog stack gone) |
| `handle_restricted_actions.rs` → `modal_manager.gd` | `scene_request` event |
| `handle_restricted_actions.rs` → `/root/explorer` `move_to`, `camera_look_at`, … | `Engine::player()` typed calls (phase 7.2) |
| `avatar_scene.rs` → `avatar.gd` methods | `DclAvatar` Rust class with the same functions (phase 7.1) |
| `scene_runner/components/*` → component `.gd` | Rust component classes, typed `bind_mut()` (phase 7.4) |
| `communication_manager.rs` → `global.gd` realm/profile callbacks | core `realm`/`session` watchers (phase 1.7) |
| `message_processor.rs` → `Gd<AvatarScene>` | `AvatarSink` trait implemented by the engine (phase 1.7) |
| `dcl_player_identity.rs` → `lobby.gd` callbacks | core `session` events (phase 1.4) |
| `notifications.rs` → `notifications_manager.gd` | core `notifications` (phase 1.9, 4.6) |
| `profile_service.rs` → `save_to_settings_file` | core `settings` (phase 4.5) |
| `dcl_video_player.rs`, `dcl_audio_stream.rs` → `_backend_*` | Rust video/audio components (phase 7.4) |
| `dcl_ios_plugin.rs`, `dcl_swift_lib.rs`, `DclAndroidPlugin` | OS intake in the shell's native host; engine keeps only what scenes need (phase 3–4) |
