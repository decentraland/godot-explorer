# Phase 3: FFI and the real entry flow

First user-visible change on the branch.

## Goal

A typed FFI from `dcl_core` to the RN app, the core app state machine, and real sign-in in RN with
every method 1.15 has: guest, email OTP, Google and Apple through the browser hand-off, MetaMask on
Android, WalletConnect on iOS. The JSON bridge, `godot/src/rn_bridge/` and the GDScript lobby are
deleted.

## Exit criteria

- [ ] On an Android phone and an iPhone, the RN app from `cargo run -- run --target android|ios`
      signs in with each method, recovers the session after a restart, enters Genesis Plaza
      through `core.enter_world`, leaves, and enters again.
- [ ] `godot/src/rn_bridge/` and `godot/src/ui/pages/auth/` are gone; `rn/src/GodotBridge.ts`,
      `rn/src/bridge/`, `rn/src/engine.ts` and `rn/src/events.ts` are gone; `rn/src/core/` is the
      only way RN reaches the core and the engine.
- [ ] One `Core` per process, verified: the TurboModule and the GDExtension share the library handle
      (an `installed()` counter asserts it in debug builds).
- [ ] `rn/src` has no hardcoded `decentraland.org|zone` or `dclexplorer.com` outside `*mock*` and
      `simulator.ts`; the eslint rule is on.
- [ ] Sentry (D-16) applied: one native crash handler in the app.

## Work items

- [ ] **3.0 The core boots without Godot**. On phones the host creates the core at app launch,
  before any engine. Today `DclGlobal::init` does that with Godot's `user_data_dir`, and it also
  installs the logger (which needs `Os::singleton()`) and `log_panics`. All three move to the core:
  `Core::install(CoreConfig)` is called by the host, or by `DclRoot` on desktop. The core installs the
  one global `tracing` subscriber, with the platform sink (logcat, os_log, stderr) and a slot the
  engine fills with its Godot layer when it starts; a second `.init()` panics on Android and iOS
  today. There is one chaining panic hook (today `log_panics` replaces gdext's). A second
  `Core::install` with a different config, or `Core::spawn` before install, fails in debug instead of
  being ignored or starting a private runtime. The content dir comes from `Core::data_dir()`, not
  `Os::get_user_data_dir()` (`content_provider.rs`), with a debug check at engine start that both
  match.
- [ ] **3.1 Binding setup**. uniffi proc-macros on a slice of the core
  (`state()`, `sign_in_guest()`, one async call, the event callback interface), built into
  `libdclgodot` with the `ffi` feature. Set up `uniffi-bindgen-react-native` (D-17) for the
  generated C++/JSI TurboModule and TS types. The checks below are its acceptance; a failed one
  is fixed inside the time box, and only if it cannot be fixed does the hand-written Kotlin/Swift
  TurboModule come back as a reopened decision. Pass/fail, in order:
  1. Builds and runs on RN 0.86 with the new architecture and Hermes, on an Android phone (NDK 27,
     our own cargo build, no cargo-ndk) and on an iPhone (static link in the embedded host).
  2. Binds to the already-loaded `libdclgodot` (bindgen in library mode against it), not to a
     second library (D-6).
  3. An async Rust call returns a JS promise; a JS-implemented callback interface receives events
     fired from a tokio thread, on the JS thread, with no crash at 1k events/s.
  4. The generated TS types serve as the interface `rn/src/core/mock.ts` implements.
  5. Bindings regenerate in CI from the host build; a changed Rust signature fails `tsc`.
  6. uniffi's Kotlin and Swift bindings come from the same `libdclgodot` next to ubrn's; a Kotlin
     and a Swift implementation of a Rust trait register with the core and are called from a tokio
     thread; on Android the Kotlin bindings (JNA) load the copy Godot opened (`libdclgodot.so`
     linked with a soname, checked in `/proc/self/maps`). This one is D-41's, not D-17's.
  Record the result against D-17 in `decisions.md`.
  **Result, Android (Moto, 14 Oct 2026; iOS not run, it needs a Mac):** 1 pass (RN 0.86, new arch,
  Hermes, our own cargo build with `-Wl,-soname,libdclgodot.so`); 2 pass (`libdcl-core.so` has
  `NEEDED libdclgodot.so`, one path in `/proc/PID/maps`, bindgen runs in library mode against it);
  3 pass (`debugDelay` resolves, 1000 events at 1k/s reach the JS listener); 4 and 5 pending in 3.3
  and 3.10; 6 Android pass (Kotlin implements a Rust trait, called from a tokio thread, JNA loads the
  same copy). **Finding:** a foreign trait has one vtable per process, so Kotlin and JS cannot both
  implement `AppListener` (the later registration wins and the other side's handles resolve in the
  wrong runtime). Each foreign trait belongs to one runtime: the shell's in `dcl_core`'s FFI (JS),
  the host's in the `dcl_host` crate (Kotlin/Swift), each with its own namespace and bindings.
- [ ] **3.2 Core state machine**. The states of `04-core-api.md` as `dcl_core::app::State`;
  `Booting` recovers the persisted session; `enter_world` resolves the destination, raises
  `engine_required` if no engine is attached, then drives Idle → Loading → InWorld from engine
  events; `exit_world` keeps the session. The engine start policy is D-42: `engine_required` in advance or
  on demand, calls that need the engine wait for its attach, rendering on only while a state shows
  it. Unit tests cover every transition, the attach wait and the policy with a fake engine sink; the
  three D-42 numbers are measured on both phones.
- [ ] **3.3 FFI surface v1**. Session, boot gates, destination, URLs, state, avatar preview rect,
  analytics, lifecycle, deep links; the callback interface with every event in `04-core-api.md`.
  `rn/src/core/index.ts` is the typed surface; `rn/src/core/mock.ts` implements it for no-engine
  builds and replaces `mockBridge.ts` / `mockHandlers.ts`.
- [ ] **3.4 Native hosts**. Android: `DclCoreModule` loads `dclgodot`, installs the core with
  the data dir the Godot-exported app used (D-19, risks R-6), registers the event callback;
  `MainActivity` creates `GodotFragment` on `engine_required` (the engine then attaches itself, D-42)
  and flips surface order on
  `avatar_preview_mode`. `GodotBridgeModule.kt` and `RnHostPlugin.kt` are deleted. The host
  reaches the core through uniffi's Kotlin/Swift bindings (D-41). iOS: same in
  `GodotHost.mm` / `AppDelegate.swift`; the missing `set_surface_on_top` equivalent is written
  (CAMetalLayer order). `GODOT_USE_BRANCH` set so `install --targets ios` fetches
  `ios_host_release.a` from the engine branch build (D-23).
- [ ] **3.5 Attestation**. `attestation_service.gd` (869) → `dcl_core::attestation`: session
  token exchange with the mobile-bff (URL from `urls`, not hardcoded), JWT cache under the data dir
  (importing `user://attest_session.json` once), retry policy. The host provides the OS call as a
  service trait (D-41): Play Integrity on Android, `DCAppAttestService` on iOS, the plugin code moved
  into the host as-is.
- [ ] **3.6 Entry flow on the core, every method**. `flow/auth.ts` stub → core calls. Guest;
  email OTP (the form and code modal are missing in RN today); Google and Apple through the
  browser hand-off (`open_url` event, app-return completes through `handle_deep_link`, all already
  Rust in `decentraland_auth_server.rs` / `remote_wallet.rs`); MetaMask and
  WalletConnect through Reown AppKit for RN, the core making the message to sign and receiving the
  signature; on iOS only if product wants native WalletConnect in 2.0.0, otherwise the web flow as in
  1.15 (D-25, D-41); the in-app browser and auth session through an RN library, keeping today's
  choice per flow;
  welcome-back from the real profile; T&C version; version gate and device-support gate as core
  calls with RN modals; the FTUE "Jump in" through `resolve_destination`.
- [ ] **3.7 Deep links**. RN `Linking` and the iOS `AppDelegate` → `core.handle_deep_link`;
  `deep_link_router.gd` routing (jump, open, events, places, sign-in resume with the 5-minute cap,
  config params, push attribution) → `dcl_core::deep_link::route` on the state machine.
- [ ] **3.8 Delete the bridge and the lobby, add the lint**. Remove `godot/src/rn_bridge/`
  (15 files; `rn_ui_filter.gd` last, or after the phase-4 sweep), the `RnBridge` autoload,
  `--rn-host` / `--rn-avatar-only` in `dcl_cli.rs`, the RN JSON layer, and `ui/pages/auth/`
  (`lobby.gd` 1,428, `login.gd`, `sign_in_with_email.gd`, the lobby avatar editor), dead from this
  commit. Add the eslint `no-restricted-syntax` rule for hardcoded hosts; move the eight files in
  `01-today.md` to `core.urls()`.
- [ ] **3.9 Sentry**. `@sentry/react-native` in the app with the native SDKs
  under it; the sentry-godot addon and `SentryObjC.framework` leave the RN build; the policy in
  `project_main_loop.gd` (per-source keep rates, remote rate from flags) → `dcl_core::report` with
  tests; Rust panics (`log_panics`) and engine errors reach the shell as `report` events and are
  captured there with the same tags `crash_context.rs` sets today. CI symbol upload for
  `libdclgodot` and the engine keeps working (risks R-7).
- [ ] **3.10 CI**. `rn_app.yml` keeps typecheck, lint, jest, the no-engine Android build and
  the iOS simulator build; the with-engine iOS job runs on the branch engine build (D-23).
- [ ] **3.11 Avatar preview fast path**. Today the entry-flow preview waits for the engine boot, then a
  random marketplace outfit downloaded and imported on the phone, imports running one at a time under
  the content lock, and a ready signal that fires only after every wearable and emote (risks R-18).
  First measure, with timestamps at engine start, engine ready, `backpack_open`, wearable definitions
  fetched, each wearable (optimized-server result, cache hit or miss, import time), all wearables
  loaded, avatar assembled and the `avatar` event to RN, on the Moto with a fresh install and a second
  launch. Then: the advance start from D-42 with the avatar loaded into a hidden preview, so
  `show_avatar_preview` only shows it; the local baked `wearable_{hash}.scn` and zip checked before the
  optimized-server HEAD, and its 404s remembered on disk; the avatar reported ready after body and
  wearables, with emotes and their audio loaded afterwards (the idle animation is bundled); the cache
  index scanned in the background at boot, last-used times kept between launches, the lookup that can
  run before the scan fixed, and one cache limit (today 1 GB from settings vs the 2 GB default); the
  Android shader baker on, as for iOS, if it measures better; avoid creating world-only services for
  the preview where safe, with the complete Idle/World ownership split in 5.7. The starter set and
  the placeholder follow D-open-17.

## Verification

- `cargo test -p dcl_core` covers the state machine and deep-link routing.
- Jest covers `rn/src/core/mock.ts` against the same interface.
- The exit criteria on both phones, from `cargo run -- run --target …`, and the no-engine build on
  a simulator and an emulator.

## Depends on / unblocks

Depends on phase 1 (core) and 2.1–2.3 (engine facade, scene requests). Unblocks phase 4 (every
screen rewires from the JSON `req` methods to the FFI).
