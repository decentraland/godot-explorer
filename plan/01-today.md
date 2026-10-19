# Today: the tree on 13 Oct 2026

What exists on `feat/react-native` at `918f290fd4`. The proposal's numbers were taken on 12 Oct,
before the Figma UI rebuild (`2339f916a9`); these supersede them.

## Numbers

| | |
|---|---|
| GDScript | 419 files, 86,739 lines under `godot/src` |
| of which UI (`src/ui`) | 218 files, 40,400 lines (components 22.4k, pages 14.9k, `explorer.gd` 1,893) |
| app logic (`src/logic`) | 36 files, 7,976 lines |
| engine-side (`src/decentraland_components`, `logic/player`, `mobile/joystick`) | ~14,200 lines |
| tools and tests (`src/tool`, `src/tools`, `src/test`, `src/client_tests`) | ~12,100 lines |
| editor addons (`godot/addons`, GDScript only) | 4,565 lines |
| Rust (`lib/src`) | ~95,600 lines, one crate `dclgodot` (cdylib), **not a workspace** |
| Godot classes exported from Rust | 78 `#[derive(GodotClass)]` sites |
| React Native (`rn/src`) | ~27,700 lines of TS/TSX; all 286 Figma frames have a screen |
| JSON bridge (`godot/src/rn_bridge`) | 15 files, 1,336 lines |
| Native hosts | Android 269 lines Kotlin; iOS 397 lines ObjC++/Swift |
| Autoloads | 17 (`UiSounds`, `Global`, `SessionTracker`, `SplashOverlay`, `NotificationsManager`, `ConnectionQualityMonitor`, `EmptyParcelProps`, `Iap`, `MarketplaceTracker`, `ImpostorCapturer`, `AvatarLODCoordinator`, `DebugWs`, `DevLocalhostRelay`, `GltfLoadingCoordinator`, `LightSourceManager`, `TouchFeedback`, `RnBridge`) |

Largest `.gd` files: `modal_manager.gd` 1,900 (at the cap), `explorer.gd` 1,893, `avatar.gd` 1,885,
`global.gd` 1,861, `settings.gd` 1,644, `notifications_manager.gd` 1,539, `player.gd` 1,514,
`scene_fetcher.gd` 1,429, `lobby.gd` 1,428, `avatar_emote_controller.gd` 1,307, `backpack.gd` 1,267.
`.gdlintrc`: `max-file-lines: 1900`, `max-public-methods: 45`.

## How the client boots

`run/main_scene` is `src/main.tscn` → `main.gd` (130 lines) routes on CLI flags and otherwise loads
`ui/pages/auth/lobby.tscn`. `--skip-lobby`, `--guest-profile`, previews and the benchmark are decided
inside `lobby.gd`, not `main.gd`. `global.gd` (`extends DclGlobal`) is the composition root: its
`_ready` (lines 439–798) creates every Rust service node and every GDScript manager in a fixed order
and adds them under `/root` with `call_deferred`. CLI parsing itself is Rust (`dcl_cli.rs`, 1,153).

## Rust: nothing is Godot-free as-is

The modules the proposal puts in `dcl_core` all reach shared state through the `DclGlobal` node.
The edges, so phase 1 can cut them in order:

- **Runtime.** `scene_runner/tokio_runtime.rs` is a Godot `Node` built in `INode::init`; everything
  gets a handle through `DclGlobal::try_singleton()?.bind().tokio_runtime` (`TokioRuntime::static_clone_handle`).
  ~15 files spawn through it: `content_provider.rs` (27 refs), `dcl_social_service.rs` (21),
  `dcl_player_identity.rs` (13), `communication_manager.rs`, `metrics.rs`, `pulse_room.rs`,
  `signed_login.rs`, `message_processor.rs`, `rust_http_queue_requester.rs`, `profile_service.rs`,
  `avatar_scene.rs`, `scene_entity_coordinator.rs`. Four local runtimes are fine where they are
  (`dcl/js/mod.rs` per scene thread, `js/inspector.rs`, `adapter/livekit.rs`, `pulse/native.rs`).
- **Small type leaks** that drag pure code into Godot: `GodotTokioCall` in `dcl_tokio_rpc.rs` (used by
  `auth_identity.rs`, `remote_wallet.rs`, `decentraland_auth_server.rs`); `TextureQuality` in
  `dcl_config.rs` (used by `content/bundle.rs`); the hash in `dcl_hashing.rs` (used by
  `content/cache_file_name.rs`); the request type in `tools/network_inspector.rs` (used by
  `http_queue_requester.rs`).
- **Godot math types** in otherwise pure code: `Vector2/Vector2i/Vector3/Color` in `comms/pulse/*`,
  `movement_compressed.rs`, `comms/profile.rs`, `dcl/mod.rs`, `scene_apis.rs`, `common/scene.rs`;
  `to_godot()` impls in the generated `proto_components.rs`; `SceneEntityId` implements
  `GodotConvert/Var/Export` in `dcl/components/mod.rs`.
- **`user://` through `FileAccess`:** `auth/thirdweb_guest.rs` (`thirdweb_session.json`),
  `auth/device_anchor.rs` (`device_anchor.txt`), `dcl_config.rs` (`settings.cfg`), `dcl/js/mod.rs`.
- **Comms holds engine handles:** `message_processor.rs` (2,354) owns `Gd<AvatarScene>` and
  `Option<Gd<DclSocialBlacklist>>` and reaches `scene_runner` through `DclGlobal`;
  `communication_manager.rs` (3,764) calls `DclGlobal::singleton()` 36 times (identity, avatars,
  realm, scene runner, cli); `archipelago.rs` and `ws_room.rs` use `Gd<WebSocketPeer>`; `livekit.rs`
  and `signed_login.rs` read realm/version through `DclGlobal` with no `godot::` import.
- **Protos.** `lib/build.rs` (891) generates protos, `dcl_rpc` and the component codegen into one
  `OUT_DIR`; `social/` (pure) needs `social_service` protos and comms needs `rfc4`.
- **Promise glue:** `godot_classes/promise.rs` + `content/thread_safety.rs`. `content_provider.rs`
  (24 `make_to_async`), `dcl_social_service.rs` (17), `dcl_player_identity.rs` (7),
  `rust_http_queue_requester.rs` (2). This is the "1.4k lines of Promise glue" over friends.

Already pure and tested under plain `cargo test`: `deep_link.rs` (72 tests), `env` (12), `urls` (7),
`social/` (1.5k lines), `http_queue_requester.rs`, `content/resource_provider.rs` (the LRU disk cache),
`auth/{wallet,ephemeral_auth_chain,ethereum_provider,review_login,deploy_profile}.rs`,
`comms/adapter_trait.rs`, `pulse/{transport,native,lsd_realm}.rs`, `loading_funnel.rs` (18).

## Rust → GDScript by name

Hard-coded node paths: `scene_runner/global_get_node_helper.rs` (`/root/explorer`,
`/root/explorer/UI/DialogStack/Stack`, `/root/realm`, `/root/explorer/world/Player/Avatar`),
`rpc_calls/handle_restricted_actions.rs` (`/root/Global` ×3), `dcl_ui_control.rs`
(`/root/explorer/UI/MobileCameraInput`), `dcl_global.rs` (`explorer`, `Global`).

Methods called by string, by caller (the checklist for phases 2 and 5):

| Caller | GDScript methods |
|---|---|
| `handle_restricted_actions.rs` | `async_show_external_link_modal`, `async_show_teleport_modal`, `async_show_realm_teleport_modal`, `async_show_change_realm_modal`, `async_load_nft`, `move_to`, `camera_look_at`, `player_look_at`, `avatar_look_at_independent`, `async_play_emote`, `stop_emote_from_scene`, `register_scene_emote_content` |
| `avatars/avatar_scene.rs` | `async_update_avatar_from_profile`, `async_play_emote`, `stop_emote_from_network`, `take_unfinished_emote`, `push_voice_frame`, `spawn_voice_channel`, `set_blocked_and_hidden`, `set_client_version`, `set_room_debug`, `setup_trigger_detection`, `set_profile_request_state`, `on_avatar_*` |
| `scene_runner/scene_manager.rs` | `apply_audio_props`, `get_realm_string`, `async_take_and_compare_snapshot`, `open_profile_by_avatar`, `clear_emote_owned_by_scene`; signals `loading_*`, `scene_crashed`, `scene_killed`, `scene_crash_report`, `pointer_tooltip_changed`, `low_memory_warning` |
| `scene_runner/components/*` | `change_gltf`, `async_update_avatar`, `set_point/set_spot/set_light_enabled/...`, `set_box/set_plane/set_sphere/set_cylinder`, `set_texture`, `fetch_texture_by_hash`, `add_raycast`, `refresh_overlapping_detectors`, `init_livekit_audio`, `stream_buffer`, `init_player_collider_filter` |
| `comms/communication_manager.rs`, `message_processor.rs` | `_on_realm_changed_deferred`, `_on_update_profile`, `change_adapter`, `on_adapter_changed`, `scene_access_checked`, `set_avatar_version_by_address` |
| `auth/dcl_player_identity.rs` | `_error_getting_wallet`, `_set_thirdweb_guest_flag`, `_set_thirdweb_guest_upgraded_flag`, `try_set_remote_wallet`, `_mobile_auth_completed`, `_mobile_auth_failed` |
| `notifications.rs` | `db_insert_notification`, `db_query_notifications`, `db_count_notifications`, `os_schedule_notification`, `os_get_scheduled_ids`, `cancel_queued_local_notification`, `_sync_notification_queue` |
| `profile_service.rs`, `thread_safety.rs`, `dcl_video_player.rs`, `dcl_audio_stream.rs`, `dcl_ios_plugin.rs`, `dcl_swift_lib.rs` | `save_to_settings_file`, `set_thread_safety_checks_enabled`, `_backend_*`, plugin methods |

`.tscn` files loaded from Rust: `avatar.tscn`, `player_collider_filter.tscn`, `nft_dialog.tscn`
and the eleven component scenes (`audio_source`, `audio_stream`, `avatar_attach`,
`avatar_modifier_area`, `camera_mode_area`, `gltf_container`, `light_source_component`,
`mesh_renderer`, `nft_shape`, `text_shape/dcl_text_shape`, `video_player`).

## GDScript that does HTTP or persists state

`Global.async_signed_fetch` (`global.gd:1549`) over the Rust `http_requester`; `locations.gd` is the
only raw `HTTPRequest` left. The files that move to the core, with what they own:

| File | Lines | Owns |
|---|---|---|
| `auth/attestation_service.gd` | 869 | App Attest / Play Integrity → mobile-bff session token; `user://attest_session.json`; prod BFF URL hardcoded at :73 |
| `logic/destination_resolver.gd` | 547 | place existence + access checks; signed POST |
| `logic/realm.gd` | 440 | realm `/about`; hardcoded catalyst list :13–22 |
| `logic/scene_fetcher.gd` | 1,429 | scene entities, optimized manifests from `user://content/`; SDK6 adaptation-layer URL :12 (engine side) |
| `notifications_manager.gd` | 1,539 | poll, mark-read, local OS notifications, toast queue |
| `iap/iap_manager.gd`, `iap/marketplace_tracker.gd` | 1,078 + 363 | StoreKit, credits signed calls, owned-wearables poll |
| `config/config_data.gd` | 767 | settings schema + `session_account`/`guest_profile` in `user://settings.cfg` |
| `feature_flags.gd` | 142 | mobile-bff flags (feed comms and Sentry sampling) |
| `campaigns.gd`, `version_gate.gd`, `device_support_coordinator.gd` | 198, 89, 182 | campaign tokens, force-upgrade gate, end-of-support check |
| `session_tracker.gd` | | `user://session_state.cfg`, App Opened chaining |
| `logic/world_permissions_helper.gd`, `logic/content/opensea_nft_fetcher.gd`, `logic/player/player_identity.gd`, `bug_report/bug_report_service.gd`, `connection_quality_monitor.gd` | | private worlds, NFT metadata, profile, Intercom proxy, realm HEAD |

Scene-requested actions: `dcl/js/restricted_actions.rs` → `RpcCall::*` → `handle_restricted_actions.rs`
→ `/root/Global` → `modal_manager.gd` (`async_show_change_realm_modal` :354,
`async_show_external_link_modal` :197, `async_show_teleport_modal` :268,
`async_show_realm_teleport_modal` :374); `open_nft_dialog` loads `nft_dialog.tscn` into
`/root/explorer/UI/DialogStack/Stack`; `move_player_to` and emotes call `/root/explorer` and the
avatar directly. In RN mode nothing intercepts these: the Godot modals still open and `rn_ui_filter.gd`
hides them.

Sentry: addon `godot/addons/sentry` is sentry-godot 2.1.1 (binaries only); the policy is
`project_main_loop.gd` (355 lines: manual init, `_before_send` with per-source keep rates, remote
rate from feature flags). No Rust sentry crate; Rust reaches the `SentrySDK` singleton dynamically
(`content/crash_context.rs`). The RN iOS app also bundles `SentryObjC.framework` from the Godot export.

## The JSON bridge

`RnHostPlugin` (Android `RnHostPlugin.kt`; iOS the engine fork's `godot_host_send_command` /
`godot_host_set_event_callback`) carries JSON both ways. `rn_bridge.gd` dispatches by `match`, emits
`state` at 4 Hz and runs `rn_ui_filter.gd` every 0.2 s to hide the Godot UI.

- Commands: `backpack_open|place|close`, `equip`, `unequip`, `set_body_shape`, `set_color`,
  `play_emote`, `set_emote_slot`, `randomize_avatar`, `set_name`, `enter_world`, `exit_world`,
  `teleport`, `send_chat`, `emote`, `focus_camera`, `set_hud_insets`, `set_chat_width`, `chat_open`,
  `hide_ui`.
- Request/response (`rn_requests.gd` → `rn_req_*.gd`): `settings.{get_all,set,cache_size,clear_cache,version}`,
  `account.{sign_out,delete}`, `friends.{list,requests,nearby,accept,reject,cancel,send,unblock,block}`,
  `notifications.{list,read}`, `profile.{get,names,deploy}`, `fetch.signed` (allow-listed hosts),
  `places.{favorite,vote}`, `clipboard.copy`, `screenshot.capture`, `scene.*`.
- Events: `ready`, `world`, `loading`, `state`, `res`, `avatar`, `settings_changed`, `chat`,
  `unread_chat`, `hud_state`, `loading_progress`, `open_backpack`.
- RN side: `engine.ts` keeps a snapshot `{started, ready, avatar, inWorld, hud}` and a command queue
  until `ready`; `events.ts` has its own `booting|loading|playing` phase. Two state machines.
- Boot args differ per platform: Android `--rn-host --rn-avatar-only --force-mobile --no-pulse`
  (`MainActivity.kt:102`); iOS `--rn-host --skip-lobby --guest-profile --realm … --location 0,0`
  with no `--rn-avatar-only` (`GodotHost.mm:106`).
- Rules copied into TypeScript: `backpack/emotes.ts` (base emotes), `taxonomy.ts`, `catalog.ts`
  (body-shape URNs, collection URLs), `platformPolicy.ts`, `settings/keys.ts` (`settings.cfg` keys),
  `flow/flowMachine.ts` (mirrors `lobby.gd`), `discover/catalog.ts`.
- Hardcoded hosts in non-mock TS: `discover/api.ts`, `discover/eventTime.ts`, `backpack/catalog.ts`,
  `backpack/platformPolicy.ts`, `settings/links.ts`, `hud/Hud.tsx`, `flow/screens.tsx`, `menu/api.ts`.
  `decentraland.zone` appears nowhere: RN has no environment switch.

Open gaps recorded in `docs/rn-ui-spec/PLAN-STATUS.md` and `rn/README.md`: iOS backpack overlay
unwired (no `set_surface_on_top`), Android back button not forwarded, push/deep links/IAP via the
Godot plugins only on the Android emulator, `fetch.signed` has no path/method limit, friends and
notifications untested with a signed-in account, 14–18 fps on the release template not yet compared
with the Godot-exported app.
