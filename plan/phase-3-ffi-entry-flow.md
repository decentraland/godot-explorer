# Phase 3: FFI and the real entry flow

Side-project target, Nov 2026. First user-visible change on the branch.

## Goal

A typed FFI from `dcl_core` to the RN app, the core app state machine, and real sign-in in RN: guest,
email OTP, browser hand-off, wallet. The JSON bridge and `godot/src/rn_bridge/` are deleted.

## Exit criteria

- [ ] On an Android phone and an iPhone, the RN app from `cargo run -- run --target android|ios`
      signs in as guest and by email, recovers the session after a restart, enters Genesis Plaza
      through `core.enter_world`, leaves, and enters again.
- [ ] `godot/src/rn_bridge/` is gone; `rn/src/GodotBridge.ts`, `rn/src/bridge/`, `rn/src/engine.ts`
      and `rn/src/events.ts` are gone; `rn/src/core/` is the only way RN reaches native.
- [ ] One `Core` per process, verified: the TurboModule and the GDExtension share the library handle
      (an `installed()` counter asserts it in debug builds).
- [ ] `rn/src` has no hardcoded `decentraland.org|zone` or `dclexplorer.com` outside `*mock*` and
      `simulator.ts`; the eslint rule is on.
- [ ] Sentry decision (D-open-1) applied: one native crash handler in the app.

## Work items

- [ ] **3.1 Binding trial** (S, time-boxed to two days). uniffi proc-macros on a slice of the core
  (`state()`, `sign_in_guest()`, the event callback). Try `uniffi-bindgen-react-native` for a
  generated TurboModule on RN 0.86 with the new architecture; fallback is uniffi's Kotlin and Swift
  bindings plus a hand-written `DclCore` TurboModule. Record the choice in `decisions.md` D-open-2.
- [ ] **3.2 Core state machine** (L). The states of `04-core-api.md` as `dcl_core::app::State`;
  `Booting` recovers the persisted session; `enter_world` resolves the destination, raises
  `engine_required` if no engine is attached, then drives Idle → Loading → InWorld from engine
  events; `exit_world` keeps transports and session. Unit tests cover every transition with a fake
  engine sink.
- [ ] **3.3 FFI surface v1** (L). Session, boot gates, destination, URLs, state, avatar preview rect,
  analytics, lifecycle, deep links; the callback interface with every event in `04-core-api.md`.
  `rn/src/core/index.ts` is the typed surface; `rn/src/core/mock.ts` implements it for no-engine
  builds and replaces `mockBridge.ts` / `mockHandlers.ts`.
- [ ] **3.4 Native hosts** (M). Android: `DclCoreModule` loads `dclgodot`, installs the core with
  the app's files dir (risks R-6), registers the event callback; `MainActivity` creates
  `GodotFragment` on `engine_required` and flips surface order on `avatar_preview_mode`.
  `GodotBridgeModule.kt` and `RnHostPlugin.kt` are deleted. iOS: same in `GodotHost.mm` /
  `AppDelegate.swift`; the missing `set_surface_on_top` equivalent is written (CAMetalLayer order).
- [ ] **3.5 Attestation** (L). `attestation_service.gd` (869) → `dcl_core::attestation`: session
  token exchange with the mobile-bff (URL from `urls`, not hardcoded), JWT cache under the data dir
  (importing `user://attest_session.json` once), retry policy. The host provides the OS call as a
  callback: Play Integrity on Android, `DCAppAttestService` on iOS, ported from the Godot plugins.
- [ ] **3.6 Entry flow on the core** (M). `flow/auth.ts` stub → core calls. Guest; email OTP (the
  form and code modal are missing in RN today); browser hand-off (`open_url` event, app-return
  completes through `handle_deep_link`); welcome-back from the real profile; T&C version; version
  gate and device-support gate as core calls with RN modals; the FTUE "Jump in" through
  `resolve_destination`. Wallet sign-in (WalletConnect / MetaMask) through an RN library
  (decisions D-open-8); if it is not ready by the scope gate it moves to phase 4.
- [ ] **3.7 Deep links** (M). RN `Linking` and the iOS `AppDelegate` → `core.handle_deep_link`;
  `deep_link_router.gd` routing (jump, open, events, places, sign-in resume with the 5-minute cap,
  config params, push attribution) → `dcl_core::deep_link::route` on the state machine.
- [ ] **3.8 Delete the bridge, add the lint** (S). Remove `godot/src/rn_bridge/` (15 files), the
  `RnBridge` autoload, `--rn-host` / `--rn-avatar-only` in `dcl_cli.rs`, the RN JSON layer. Add the
  eslint `no-restricted-syntax` rule for hardcoded hosts; move the eight files in `01-today.md` to
  `core.urls()`.
- [ ] **3.9 Sentry** (M, after D-open-1). `@sentry/react-native` in the app with the native SDKs
  under it; the sentry-godot addon and `SentryObjC.framework` leave the RN build; the policy in
  `project_main_loop.gd` (per-source keep rates, remote rate from flags) → `dcl_core::report` with
  tests; Rust panics (`log_panics`) and engine errors reach the shell as `report` events and are
  captured there with the same tags `crash_context.rs` sets today. CI symbol upload for
  `libdclgodot` and the engine keeps working (risks R-7).
- [ ] **3.10 CI** (S). `rn_app.yml` keeps typecheck, lint, jest, the no-engine Android build and
  the iOS simulator build; the with-engine iOS job stays manual until the host template is published
  (`ios_host_release.a`, decisions D-open-10).

## Verification

- `cargo test -p dcl_core` covers the state machine and deep-link routing.
- Jest covers `rn/src/core/mock.ts` against the same interface.
- The exit criteria on both phones, from `cargo run -- run --target …`, and the no-engine build on
  a simulator and an emulator.

## Depends on / unblocks

Depends on phase 1 (core) and 2.1–2.3 (engine facade, scene requests). Unblocks phase 4 (every
screen rewires from the JSON `req` methods to the FFI) and the scope gate.
