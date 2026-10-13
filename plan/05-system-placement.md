# Shared systems: where each one lives, and when it moves

Rule of thumb: anything needing the identity key, anything the engine and the shell both read, and
anything that must work while the engine is not running goes in the core. Items reference the work
items in the phase files.

| System | Today | Target | Moves in |
|---|---|---|---|
| Tokio runtime | `scene_runner/tokio_runtime.rs`, a Node reached through `DclGlobal` | core, owned by `Core` | 1.1 |
| HTTP queue | `http_queue_requester.rs` (pure) under `RustHttpQueueRequester` + `Promise` | core; the Godot class is a thin adapter until GDScript callers are gone | 1.2 |
| URLs, env, deep-link parsing | `urls/`, `env/`, `deep_link.rs` (pure) under `DclUrls`, `DclParseDeepLink` | core | 1.2, 1.10 |
| Friends, blacklist | `social/` (1.5k, pure) under `dcl_social_service.rs` (1,414) and `dcl_social_blacklist.rs` (293) | core; first migration | 1.2, 4.1 |
| Session and identity | `auth/dcl_player_identity.rs` (1,597, shell), flow in `lobby.gd` (1,428), persistence in `config_data.gd` → `settings.cfg`, `thirdweb_guest.rs`, `device_anchor.rs` | core `session`, persisted by the core; the shell drives screens | 1.4, 3.2, 3.6 |
| Realm `/about`, catalyst list | `logic/realm.gd` (440) over `DclRealm` | core `realm` | 1.5 |
| Feature flags | `feature_flags.gd` (142) pushing values into Rust | core, fetched at boot | 1.5 |
| Analytics | `analytics/metrics.rs` (1,518, Godot `Timer`), ~106 GDScript track calls | core; shell and engine track through it; no Segment RN SDK | 1.6, 4.12 |
| Comms, chat, voice | `comms/` (12.3k); `message_processor.rs` holds `Gd<AvatarScene>`; `archipelago.rs`/`ws_room.rs` on `Gd<WebSocketPeer>` | transports in the core; avatar fan-out in the engine through a sink; chat events reach the shell through the core | 1.7 |
| Content cache | `content/resource_provider.rs` (1,123, LRU) + `user://content` inside `content_provider.rs` (3,233); scattered GDScript caches | cache policy and data dir in the core; consumers in the engine; RN's image cache for UI thumbnails. Two caches on purpose. | 1.8 |
| Notifications | `notifications_manager.gd` (1,539) + `lib/src/notifications.rs` (777) bridge | core polls and holds read state; shell renders and schedules OS notifications; push token from RN native | 1.9, 4.6 |
| Destination, teleport, access | `destination_resolver.gd` (547), `world_permissions_helper.gd`, checks in `global.gd` | core `destination`; the shell asks and gets an answer | 1.5, 4.2 |
| Scene-requested actions | `handle_restricted_actions.rs` → `/root/Global` → `modal_manager.gd` | engine event → core policy → shell modal | 2.3, 4.9 |
| Attestation | `auth/attestation_service.gd` (869), own HTTP, JWT cache on disk, hardcoded prod BFF | core, with the OS attest call provided by the host | 3.5 |
| Deep-link routing | `deep_link_router.gd` (367) | shell receives the link, core parses and routes through the state machine | 3.7 |
| Sentry | sentry-godot 2.1.1 addon + `project_main_loop.gd` policy (355) + `crash_context.rs` | one native crash handler (`@sentry/react-native`); Rust panics and engine errors forwarded through the core; policy in the core with tests; addon dropped | 3.9 (D-16) |
| Boot gates: version, device support, campaigns, upgrade nudge, review prompt, session tracker | `version_gate.gd`, `device_support_coordinator.gd`, `campaigns.gd`, `upgrade_nudge_coordinator.gd`, `review_prompt_coordinator.gd`, `session_tracker.gd` | logic in the core, UI in the shell | 3.6, 4.11 |
| Public GETs: places, events, base catalog | GDScript generators and RN (`discover/api.ts`, `backpack/catalog.ts`) with hardcoded hosts | shell, typed client, hosts from `core.urls()` | 4.2, 4.13 |
| Signed requests: likes, reminders, favorites, notifications read, world permissions, credits, account deletion | `Global.async_signed_fetch`, `rn_req_misc.gd fetch.signed` with a host allow-list | typed core functions; no generic signed fetch across the FFI | per domain in phase 4 |
| Backpack equip rules | `wearables.gd` (681) + rules in `backpack.gd` (1,267); copied into `backpack/{taxonomy,emotes,catalog,platformPolicy}.ts` | core: "can I equip X" returns the resulting avatar; the engine renders it; TS copies deleted | 4.3 |
| Settings | `config_data.gd` (767) over `dcl_config.rs` `ConfigFile` | one store in the core; the shell edits, the engine reads | 4.5 |
| Profile | `profile_service.rs`, `logic/player/player_identity.gd`, `content/profile.rs` | core | 4.7 |
| IAP and credits | `iap_manager.gd` (1,078), `marketplace_tracker.gd` (363), `DclStoreKitPlugin`, `dcl_swift_lib` | store purchase in the shell (RN IAP library); quote, verify, balance in the core | 4.8 |
| Modals (30 kinds) | `modal_manager.gd` (1,900) | RN modals; policy in the core where there is one | 4.9 |
| Loading screen, HUD, menu, chat UI | `loading_screen.gd`, `explorer.gd` UI, `organisms/menu`, `pages/chat` | RN (already built from Figma) | 4.4, 4.10 |
| Bug report | `bug_report/` (651) | core sends; shell collects | 4.11 |
| Avatar, emotes, impostors | `avatar.gd` (1,885), `avatar_emote_controller.gd` (1,307), loaders, impostor autoloads | engine | 5.1 (early: Idle preview needs it) |
| Player controller, camera | `logic/player/player.gd` (1,514), `mobile_camera_input.gd` (405), `dcl_global_camera_controller.gd` (254) | engine | 5.2 |
| Joystick, on-screen gamepad | `mobile/joystick/virtual_joystick.gd` (338), `organisms/joypad` (642) | engine | 5.3 |
| SDK components in GDScript | `light_source_component.gd` (1,042), `video_player.gd` (728) + `av_player`/`exo_player`, text shape (~900), `gltf_container.gd` (532), `gltf_loading_coordinator.gd` (491), `avatar_attach`, `nft_shape`, audio | engine | 5.4 |
| Scene fetcher, loading orchestration | `logic/scene_fetcher.gd` (1,429), `loading_screen_progress_logic.gd` | engine (scene hot reload for creators included) | 5.5 |
| Native plugins | `dcl-godot-android` (push, deep links, Reown, Firebase, ExoPlayer), `DclGodotiOS`, `dcl_swift_lib` (StoreKit), sentry plugins | OS intake becomes RN libraries in the shell; the engine keeps only what scenes need (ExoPlayer video) | 3–4 |
| Debug tooling | `tool/debug_server` (debug hub eval), `dev_localhost_relay.gd`, `gp_benchmark_runner.gd` (1,175), `tools/` | typed debug commands in the engine; benchmark runner in Rust for `benchmark.yml`; the rest dropped | 5.7 |
| GDScript tests | `godot/src/test` (5.7k) | Rust `#[itest]` for what survives; deleted with their subject | 5.8 |
