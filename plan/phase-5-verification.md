# Phase 5 verification: the solo-polish checklist

Phase 5 ported the engine-side GDScript to Rust (`dcl_engine`) in one branch (`feat/react-native-phase5`,
draft PR #3081, stacked on phase 4). The user's decision 3 (15 Oct 2026): before the team polishes, one
person checks that every phase's work does what it says. This file is that checklist. Nothing here is
ticked unless it was run; **unchecked means not verified**.

## What was verified (desktop, Linux)

- [x] `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p dclgodot --lib` (509 tests).
- [x] `cargo run -- run --itest`: 25 passed, 0 failed.
- [x] Boot into Genesis Plaza with the guest profile; the avatar renders.
- [x] `find godot/src -name '*.gd'` is empty; CI step added for it.

## Not verified, or known to regress

Engine (phase 5)
- [ ] **Avatar snapshots.** Client tests `avatar_no_outline` / `avatar_with_outline` score 65% / 63% against
  the April references; `cargo run -- test-tools` scores 82–95% (threshold 90%, two face images fail).
  Framing is smaller, eyes are untinted, skin is darker. No baseline was ever taken, so this is either a
  5.2 preview-port regression or a machine/renderer difference. Bloom and uibackground pass (100%).
- [ ] **Lights (5.5).** The light-source port compiles and loads; rendering was never compared against 1.15.
- [ ] **Video (5.5).** URL video, HLS and LiveKit streams, in background and foreground: not run.
- [ ] **Scene tests** (`--stest`): not completed. One run executed the raycast scene (12 tests passed, 0 failed); later runs found a streaming stage scene at (52,-52), so the remote `scene-explorer-tests` realm no longer matches the parcel list in `src/main.rs` and the run never finishes. The run did expose and fix four dead by-name calls to static funcs (scene teleport / change realm, raycast debug draw, SDK6 texture size, UI input focus). Needs the parcel list refreshed against the current content.
- [ ] **Asset renderer, GP benchmark, debug hub** were rebuilt in Rust and only exercised by unit tests and
  boot. Run each once: `--asset-renderer --use-test-input`, the GP benchmark deeplink, `debug-hub` +
  `unified.sh state|teleport|screenshot`.
- [ ] **Creator preview** (`-- --preview` against the SDK example scenes) after the 5.6 fetcher port.
- [ ] **Idle → World → Idle → World** on a phone, including nodes released on exit.
- [ ] **`--features use_memory_debugger`** build (bootstrap has cfg branches).
- [x] Device, Moto G60, build of the final tree (16 Oct): avatar preview → name → Genesis (~30 fps) → emote wheel opens, joypad buttons render, camera swipe turns the view, chat panel opens; no panic, "already bound" or "Scene error" in logcat.
- [ ] Still to run on the device: joystick walking, emote playback, Back / Leave → re-enter, tooltip on a pointer entity.
- [ ] On the name screen the avatar renders small in a large hole (the rect is right, the framing is not): same symptom as the client snapshot framing, so it is likely the engine-side preview framing.
- [ ] Android logs `ERROR: No GDExtension library found … dcl_swift_lib.gdextension` at boot (harmless, the Swift lib is iOS-only): remove the entry from the Android export.
- [ ] fps / memory in Genesis against the phase-2 baseline. The Moto crash family (Mali / low RAM, see
  memory `qa-android-teleport-crash`) was not re-measured.

Known engine bugs and leftovers
- `import-assets` exits with a crash on a warm `godot/.godot` (editor-import teardown). Output is complete.
- A 404 profile is logged as an error (Sentry quota risk).
- Deep-link routing is dropped from the engine outside host mode.
- ~30 by-name `.call("…")` / `call_deferred("…")` remain, all on Rust classes or the iOS/Android plugin
  singletons (`dcl_ios_plugin.rs`, `emote_wheel.rs`, `joypad.rs`, `hud_button.rs`, `raycast.rs` meta,
  `text_shape/node.rs`, `thread_safety.rs`). `comms/voice_chat.rs` and `communication_manager.rs` still hold `Gd<…>`.
- Four autoloads remain (`Global`, `GlobalBoot`, `EmptyParcelProps`, `EngineHost`), all native scenes.
- (done) the four legacy `Explorer` `#[func]`s were removed; the touch-controls tooltip and emote props are typed calls.
- `dcl_resource_tracker.rs` is dead now that the addon is gone.
- 5.10 not finished: `dcl-godot-android`, `dcl_build_plugin`, `godot/android`, `emote_converter` and the Swift
  plugin are kept on purpose (decision 1); removal needs the user's call.

RN (phase 4 and earlier, still open)
- [ ] The name screen's avatar hole keeps a small rect (RN layout report, not the engine).
- [ ] RN crashes on `begin_sign_in` while already `SignedIn`.
- [ ] Push notifications, reminders, error modals, settings relaunch, es / pt_BR, joystick movement on a phone.
- [ ] OTP / Google / Gmail logins, native wallets and friends (moved to the polish stage).

iOS: **never built.** Every iOS-affecting change in phases 3–5 is unverified and needs the user's hands
(Xcode, Metal toolchain, Firebase iOS files, the `ios_host_release.a` template).

## Dev features removed (not ported, on purpose)

`--spawn-avatars`, `--emote-test`, the GDScript test scenes and `godot/examples`, the safe-margin overlay,
the scene and lighting renderers, the stress test, the FI / impostor benchmarks, the GP benchmark's
exploratory memory walkers and mesh-merge classification, GDScript `eval` (typed commands `state`, `teleport`,
`screenshot`, `log_filter` instead), the iOS dev localhost relay, and the editor addons `dcl_dev_tools`,
`dcl_mobile_preview`, `resource_tracker`, `dcl-ios-devtools`.

## Gate

The 30 Nov full-migration gate is judged after this checklist is run, not before. PRs #3072 and #3073 are
still unreviewed and unmerged; #3081 stacks on both.
