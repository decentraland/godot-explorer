# Phase 1: carve out `dcl_core`

Rust only, no user-visible change.

## Goal

`lib/` is a Cargo workspace. `dcl_core` has no `godot` dependency, runs under `cargo test` with no
engine, and holds what the pre-world screens need with no engine running: the runtime, auth and
session, HTTP queue, URLs, deep-link parsing, friends, analytics, the notifications feed, and three
things that enter Rust for the first time: session persistence, the realm rules and feature flags. The Godot `Dcl*` classes become adapters. The client built from the branch behaves identically.

**Scope:** comms transports (1.7), the content cache policy (1.8) and the Godot math types (1.3) are
done inside phase 5 (5.1), still before the full-migration gate (D-37). Until then they stay in
`dcl_engine`, and the shell reaches them through the engine's event sink.

## Exit criteria

- [x] `grep godot lib/crates/dcl_core/Cargo.toml` finds nothing.
- [x] `cargo test -p dcl_core` passes with no Godot installed (146 tests; it needs `protoc` and the
      protocol tree from `cargo run -- install`, like the rest of the workspace).
- [x] `lib/crates/dcl_engine/src/scene_runner/tokio_runtime.rs` is gone; no `TokioRuntime::` reference remains.
- [ ] `cargo run -- run` (desktop) and the Android APK from the branch pass the regression below.
      Desktop `--itest` (29) and `test-gdscript` (16) pass; the phone regression is not run yet.
- [x] `cargo clippy --workspace -- -D warnings` and `cargo fmt --all` clean.

Regression (on a phone, Godot-exported app from the branch): sign in as guest; sign in by email;
restart and recover the session; enter Genesis Plaza; chat; friends list; change a setting and
restart; open a `decentraland://` deep link; sign out. Also upgrade an installed 1.15 build and
check the session survives (the `settings.cfg` → `session.json` import, risks R-6).

## Work items

Done as an ordered stack of commits; the numbering below is the original one, the order taken
differed because `social/` needs the wallet and auth chain first.

- [x] **1.0 Workspace skeleton.** `lib/` is a workspace: `crates/dcl_core` (no `godot`) and
  `crates/dcl_engine` (package `dclgodot`, same cdylib and output paths). CI runs `cargo test -p
  dcl_core`, clippy/check/test on the workspace, and fails a PR that adds a `.gd` file.
- [x] **1.1 Runtime and `Core`.** `dcl_core::Core` (`install`/`get`/`try_get`, `runtime()`,
  `handle()`, `data_dir()`, `spawn`) owns the tokio runtime; `DclGlobal::init` installs it. The
  `TokioRuntime` node is gone. The four local runtimes were left alone.
- [x] **1.2 Leaf modules.** Moved with their tests: `env`, `urls`, `deep_link`, `clock`,
  `USER_AGENT`, the HTTP queue requester (`HttpQueueRequester`, split `RequestResponse` into core
  `HttpResponse`/`HttpError` plus the Godot wrapper classes that `Deref` to them), the pure
  network-inspector event types, `social/` and its protos (`dcl_core/build.rs` compiles
  `decentraland.social_service`; the engine re-exports it). `GodotTokioCall` became
  `dcl_core::OpenUrlRequest`. Adapters left: `DclUrls`, `DclParseDeepLink`,
  `RustHttpQueueRequester`, `DclSocialService`, `DclSocialBlacklist`.
- [x] **1.4 Auth and session.** `dcl_core::session` holds `wallet`, `ephemeral_auth_chain`,
  `auth_identity` (not in the original list), `ethereum_provider`, `remote_wallet`,
  `decentraland_auth_server`, `review_login`, `thirdweb_guest` and `device_anchor` (`FileAccess` →
  `std::fs` under `Core::data_dir()`, same file names and formats), the thirdweb account flows
  (`thirdweb_flows`) and `identity::Session`: the wallet, ephemeral chain, thirdweb markers and
  mobile-auth attempt token, returning `SessionEvent`s. `session::store` is the core's
  `session.json` (account and guest profile per profile slot, analytics id); values are opaque
  `var_to_str` strings the host chooses, written atomically. `config_data.gd` reads and writes
  them through `DclSessionStore` and imports the old `settings.cfg` `[session]`/`[analytics]` keys
  once; until `session.json` exists the old keys are kept, so nothing is lost.
  **Not done as written:** the `_mobile_auth_*`/`_set_thirdweb_*` calls were never callbacks into
  `lobby.gd`; they are `DclPlayerIdentity` calling itself through `call_deferred`, and they stay
  (adapter-internal) because the core flows return values instead of touching state, so no event
  channel was needed. `DclPlayerIdentity` is still 1,296 lines, almost all Promise and signal glue;
  `deploy_profile.rs` stays in the engine (needs `comms::profile::UserProfile`, deferred).
- [x] **1.5 Realm, feature flags, destination.** `dcl_core::realm`: URL normalisation, URN
  parsing, `DAO_SERVERS`, `/about` validation and the derived realm fields (`commit_from_about`);
  `realm.gd` keeps the fetch, the private-world gate and the signals and reads them through
  `DclRealmRules`. (The "catalyst list" is a constant, not a fetch.) `dcl_core::flags`: the flags,
  their parsing and the 5 s fail-open fetch; `feature_flags.gd` only applies them to comms,
  Sentry and metrics. `dcl_core::destination`: the readers of what a place's servers say (world
  parcel index and scene rows, comms handshake, scene info). **Not moved:** the async resolver and
  the verdicts that return `Destination`'s GDScript enums stay in `destination_resolver.gd`; they
  also need the live realm and scene coordinator, and signed fetch goes through the identity node.
- [x] **1.6 Analytics.** `dcl_core::analytics`: `data_definition`, `frame`, the install-attribution
  state machine (fed plain readings instead of reading the plugin) and `EventQueue` (stamping,
  size limits, batching, retry buffer, the Segment send). `Metrics` keeps the node: plugin
  signals, device info, the ~40 `#[func]`s and `process`. **Not done as written:** the flush stays
  on a Godot `Timer`, not a tokio interval, because each tick polls main-thread plugins.
  `Metrics` is still 1,317 lines.
- [x] **1.9 Notifications.** There was no DB or poll in `lib/.../notifications.rs` (it is all
  itests); the local-reminder DB is the native plugin's; an RN notifications library replaces it in 4.6 (D-41). `dcl_core::notifications::Feed`
  holds the feed: supported-type filter, blocked-sender filter, id dedupe, the toast age rule,
  read state, newest-first order and the toast queue. **Not moved:** the signed fetch, the 30 s
  poll `Timer` and the toasts stay in `notifications_manager.gd` (signing goes through the
  identity node), reading the feed through `DclNotificationFeed`.
- [x] **1.10 Deep link parsing.** `deep_link.rs` (72 tests) is in the core; `DclParseDeepLink`
  is the adapter. Routing waits for the state machine (phase 3.7).
- [x] **1.11 Adapters audit.** Below.

### Adapter audit (lines on 14 Oct 2026)

Anything over ~150 lines names the item that shrinks it.

| Adapter | Lines | What it is now | Goes in |
|---|---|---|---|
| `DclSocialService` | 1,414 | ~20 Promise wrappers around `SocialServiceManager` | phase 3 (typed FFI replaces the Promise glue), then deleted in 4.1 |
| `Metrics` | 1,317 | plugin signals, device info, `#[func]` surface | 4.12 (the ~154 GDScript call sites disappear with their screens) |
| `DclPlayerIdentity` | 1,296 | Promise/signal glue over `Session` and the thirdweb flows | 3.x (FFI session) and 4.7 |
| `ProfileService` | 370 | Promise wrappers around profile fetch and deploy | 4.7 |
| `DclSocialBlacklist` | 293 | state read by comms | with comms in 7.x |
| `DclUrls` | 212 | one `#[func]` per URL | 4.13 (RN reads `core.urls()`) |
| `DclRealmRules` | 201 | static `#[func]`s over `dcl_core::realm` | with `realm.gd` in 7.x |
| `DclParseDeepLink` | 201 | result copied into Godot fields | 3.7 |
| `RustHttpQueueRequester` | 179 | Promise wrappers | phase 5 with the last GDScript HTTP caller |
| `DclNotificationFeed` | 159 | Variant ↔ JSON over `Feed` | 4.6 |
| `RequestResponse` (+ error) | 157 | Godot wrappers over `HttpResponse` | with `RustHttpQueueRequester` |
| `DclDestinationRules` | 105 | static `#[func]`s | 4.2 |
| `DclRealm` | 98 | `#[var]` mirror of the realm | 7.x |
| `DclSessionStore` | 77, `DclFeatureFlags` 77, `DclTokioRpc` 53 | under the cap | stay until phase 3 |

### Moved to phase 5 (D-37)

Kept here so the numbering in the other files stays stable; they land with 5.1, before the gate.

- **1.3 Protos and math types.** `to_godot()` impls and the `GodotConvert` for `SceneEntityId`
  as engine extension traits; `glam` (D-22) replacing `Vector2/2i/3/Color` in `comms/pulse/*`,
  `movement_compressed.rs`, `comms/profile.rs`, `dcl/mod.rs`, `scene_apis.rs`, `common/scene.rs`.
- **1.7 Comms transports**. `MessageProcessor` sinks instead of `Gd<AvatarScene>` and
  `Gd<DclSocialBlacklist>`; the 36 `DclGlobal` reads in `communication_manager.rs` → core
  session/realm/flags; `archipelago.rs` and `ws_room.rs` on `tokio-tungstenite`; `livekit.rs` and
  `signed_login.rs` reading realm and version from the core. Until then `CommunicationManager`
  stays an engine node that reads identity and realm from `Core::get()`.
- **1.8 Cache policy**. `ResourceProvider` (LRU, limits, eviction) and the content dir →
  `dcl_core::cache`. For 2.0.0 the core only knows the cache dir, enough for `cache_size` and
  `clear_cache` from Settings with no engine running (4.5).

## Verification

- `cargo test -p dcl_core` on every PR (146 tests today).
- `cargo run -- run --itest` and `--stest` on every PR (unchanged GDScript test runner in this phase).
- The regression above on a phone, from the branch's Android build (`android_builds.yml` uploads
  it to R2), at the end of the phase.

## Depends on / unblocks

Nothing before it. Unblocks phase 2 (the root needs `Core`) and phase 3 (the FFI exposes the core).
