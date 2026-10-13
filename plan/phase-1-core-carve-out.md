# Phase 1: carve out `dcl_core` (reduced scope)

Rust only, no user-visible change.

## Goal

`lib/` is a Cargo workspace. `dcl_core` has no `godot` dependency, runs under `cargo test` with no
engine, and holds what the pre-world screens need with no engine running: the runtime, auth and
session, HTTP queue, URLs, deep-link parsing, friends, analytics, the notifications poll, and three
things that enter Rust for the first time: session persistence, the realm `/about` fetch and feature
flags. The Godot `Dcl*` classes become adapters. The client built from the branch behaves identically.

**Reduced scope (D-32):** comms transports, the content cache policy and the Godot math types stay
in `dcl_engine` for 2.0.0. They only matter in-world, where the engine exists, and the shell reaches
them through the engine's event sink. They move in phase 7.

## Exit criteria

- [ ] `grep godot lib/crates/dcl_core/Cargo.toml` finds nothing.
- [ ] `cargo test -p dcl_core` passes on CI with no Godot installed.
- [ ] `lib/src/scene_runner/tokio_runtime.rs` is gone; no `TokioRuntime::` reference remains.
- [ ] `cargo run -- run` (desktop) and the Android APK from the branch pass the regression below.
- [ ] `cargo clippy -- -D warnings` and `cargo fmt` cover the workspace.

Regression (on a phone, Godot-exported app from the branch): sign in as guest; sign in by email;
restart and recover the session; enter Genesis Plaza; chat; friends list; change a setting and
restart; open a `decentraland://` deep link; sign out.

## Work items

One PR each. 1.0 and 1.1 unblock the rest; phase 2 can start once 1.1 is in.

- [ ] **1.0 Workspace skeleton**. `lib/Cargo.toml` → `[workspace]` with
  `crates/dcl_core` (empty rlib) and `crates/dcl_engine` (the current crate moved, `[lib] name =
  "dclgodot"`, same crate types) and `default-members = ["crates/dcl_engine"]`. `[workspace.package]
  version` keeps `lib/Cargo.toml` as the place xtask (`src/`) and `rn/` read the version from. xtask
  `build`, `clippy`, `fmt`, `coverage` run on the workspace; `godot/dclgodot.gdextension`, the
  `sync-godot-*.sh` scripts and the CI symbol upload see the same output paths.
  CI: add `cargo test -p dcl_core` (Linux, no Godot) to `clippy.yml` or a new workflow; add the
  "no new `.gd` file" check for PRs into the branch (`git diff --diff-filter=A --name-only
  <base>... -- '*.gd'` must be empty). Done when CI is green and `cargo run -- run` works.
- [ ] **1.1 Runtime and `Core`**. `dcl_core::Core` with `CoreConfig { data_dir, env, platform }`,
  `Core::install` / `Core::get`, owning the tokio runtime. `DclGlobal::init` installs it with
  `OS.get_user_data_dir()` and the resolved env. Replace every `TokioRuntime::spawn` /
  `static_clone_handle` with `Core::get().runtime()`; delete the node and the `tokio_runtime` field.
  Leave the four local runtimes alone.
- [ ] **1.2 Leaf modules**. Move with their tests: `env`, `urls`, `deep_link`, `utils`,
  `http_request/http_queue_requester.rs`, `social/`. Fix the type leaks first: `GodotTokioCall` →
  `dcl_core::OpenUrlRequest`; the inspected-request type out of `network_inspector.rs`. The protos
  the core needs (`social_service`, kernel comms) get their own `dcl_core/build.rs`; the engine
  keeps the component codegen. Adapters left behind: `DclUrls`, `DclParseDeepLink`,
  `RustHttpQueueRequester`, `DclSocialService`, `DclSocialBlacklist` (the 17 `make_to_async`
  promises stay in the adapter).
- [ ] **1.4 Auth and session**. Move `wallet`, `ephemeral_auth_chain`, `ethereum_provider`,
  `remote_wallet`, `decentraland_auth_server`, `review_login`, `deploy_profile`, `thirdweb_guest`,
  `device_anchor` into `dcl_core::session`; `FileAccess` → `std::fs` under `core.data_dir()`.
  Session persistence enters Rust: `session_account`, `guest_profile`, `session_is_ephemeral`,
  `analytics_user_id` leave `config_data.gd` for the core's own `session.json` (D-18), with a
  one-time import from `user://settings.cfg`. `DclPlayerIdentity` (1,597) becomes an adapter whose
  `#[func]`s call the core; `lobby.gd` keeps working unchanged. The `_mobile_auth_*` and
  `_set_thirdweb_*` callbacks into `lobby.gd` become core session events the adapter re-emits as
  signals.
- [ ] **1.5 Realm, feature flags, destination**. `realm.gd` `/about` fetch and the catalyst list
  → `dcl_core::realm`; `DclRealm` and `realm.gd` read from it. `feature_flags.gd` → `dcl_core::flags`,
  fetched at boot; comms and the Sentry sample rate read the core. `destination_resolver.gd` (547)
  → `dcl_core::destination` with a GDScript adapter call (`async_resolve`) so Discover keeps working.
- [ ] **1.6 Analytics**. `Metrics` → `dcl_core::analytics` with a tokio interval instead of the
  Godot `Timer`; the `Metrics` node keeps its `#[func]`s as an adapter so the ~106 GDScript track
  calls keep working until phase 4. `install_attribution.rs`'s `DclAndroidPlugin` read becomes a
  value the host passes in.
- [ ] **1.9 Notifications poll**. The poll, pagination and read state from
  `notifications_manager.gd` and the DB calls in `lib/src/notifications.rs` → `dcl_core::notifications`
  with its own store in the data dir. The GDScript manager keeps rendering toasts and scheduling OS
  notifications from core events until phase 4.
- [ ] **1.10 Deep link parsing**. Already pure; move `deep_link.rs` (72 tests) and make
  `DclParseDeepLink` an adapter. Routing waits for the state machine (phase 3.7).
- [ ] **1.11 Adapters audit**. List every remaining `Dcl*` class whose body is "call the core and
  wrap in a Promise/signal"; each must be under ~150 lines or have a phase-4/7 item that deletes it.

### Deferred to phase 7 (D-32)

Kept here so the numbering in the other files stays stable; none of these ship in 2.0.0.

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

- `cargo test -p dcl_core` on every PR.
- `cargo run -- run --itest` and `--stest` on every PR (unchanged GDScript test runner in this phase).
- The regression above on a phone, from the branch's Android build (`android_builds.yml` uploads
  it to R2), at the end of the phase.

## Depends on / unblocks

Nothing before it. Unblocks phase 2 (the root needs `Core`) and phase 3 (the FFI exposes the core).
