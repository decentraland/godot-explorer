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
- [ ] **Avatar snapshots.** Client tests `avatar_no_outline` / `avatar_with_outline` score 64% / 63% against the April references (bloom and uibackground pass at 100%). Side by side: the current render is zoomed in more than the reference, and its eyes are white where the reference has green irises. Checked: the facial path (`apply_color_and_facial`, `get_wearable_facial_hashes`, `apply_texture_and_mask`) is the old GDScript line for line, the test sets the same colours, and the textures load; the eyes wearable currently resolves to one texture with no `_mask.png`, so there is nothing to tint. The references are probably stale against today's remote wearable content (the test fetches it from the network). Not verified. Fix: render the references from a local fixture so the test stops depending on remote content.
- [ ] **Lights (5.5).** The light-source port compiles and loads; rendering was never compared against 1.15.
- [ ] **Video (5.5).** URL video, HLS and LiveKit streams, in background and foreground: not run.
- [ ] **Scene tests** (`--stest`) do not complete. The parcel list in `src/main.rs` is still right (checked against the realm's `scenesUrn`: 52,-52 is Raycast, 54,-68 NFT Shape). Findings: the Raycast scene starts and passes its first test (one earlier run passed 12 of 12, 0 failed), then stops ticking; a real-catalyst scene ('Decentraland Wellness Week 2024 - Roads', 304 parcels, base 52,-71) loads alongside it because it covers 52,-52, and the scene ids swap between runs. Not fixed: whether scenes of the test realm should load without the catalyst's pointer scenes (the old fetcher also used radius 0). The run did expose and fix four dead by-name calls to static funcs (scene teleport / change realm, raycast debug draw, SDK6 texture size, UI input focus) and made `get_parcel_scene_id` deterministic.
- [ ] **Asset renderer, GP benchmark, debug hub** were rebuilt in Rust and only exercised by unit tests and
  boot. Run each once: `--asset-renderer --use-test-input`, the GP benchmark deeplink, `debug-hub` +
  `unified.sh state|teleport|screenshot`.
- [ ] **Creator preview** (`-- --preview` against the SDK example scenes) after the 5.6 fetcher port.
- [ ] **Idle → World → Idle → World** on a phone, including nodes released on exit.
- [ ] **`--features use_memory_debugger`** build (bootstrap has cfg branches).
- [x] Device, Moto G60, build of the final tree (16 Oct): avatar preview → name → Genesis (~30 fps) → emote wheel opens, joypad buttons render, camera swipe turns the view, chat panel opens; no panic, "already bound" or "Scene error" in logcat.
- [x] Moto, later builds (16 Oct, `7199c95` and the two-autoload build): joystick walks the avatar, Back → Leave dialog → Discover → Jump in again re-enters Genesis, the pointer tooltip ("Talk") shows on an NPC; no panic or "Method not found" in logcat. fps 17–30 in a crowded plaza.
- [x] Moto: selecting a slice of the emote wheel plays the emote (the avatar raises an arm, the wheel closes), no panic. fps was 10–13 in a crowded plaza at that moment.
- [x] Moto, build `21f9014` (after the typed-signal refactor and three fixes it exposed): enter Genesis, camera swipe, emote wheel selection, Back → Leave → Discover → Jump in → Genesis again; no panic, "call failed" or "after it has been freed" in logcat, 17–28 fps. The fixes: `LoadingSession::ready` connects one signal per `signals()` call (loading hung at 30% before), `Promise` settles through `defer_from_any_thread`, and a freed camera handle is never cloned after Leave. None of the three showed in `--itest` (25/25): wiring done in `ready` and teardown only run in the full app, so the device run is the check.
- [x] Moto, build `e522839` (`local_task` on gdext's `godot::task`): same cycle, enter → swipe → emote → Leave → re-enter, no panic and no bind-guard-across-await warning in logcat, 18 fps in the crowded plaza.
- [x] Debug hub on the Moto (`adb reverse 9231`, `cargo run -- debug-hub`): the typed commands `ping`, `state` (realm, fps, scenes, parcel, player position) and `teleport {x,y}` answered from the device; `eval` is gone by design.
- [x] Moto, build `d1ca7fb` (fresh install, guest, Genesis, teleport to 12,12 over the debug hub): the empty-parcel land around a scene renders its props (trees, rocks, plants, red grass), 52–60 fps, no panic, "call failed" or "freed" in logcat. A URL video (mp4) played through ExoPlayer in the same session (backend initialised, `READY`, GPU surface 854x480, released on leaving). HLS and LiveKit streams were not run.
- [ ] On the name screen the avatar renders small in a large hole (the rect is right, the framing is not): same symptom as the client snapshot framing, so it is likely the engine-side preview framing.
- [x] The `dcl_swift_lib.gdextension` boot error is gone on Android: the preset's `exclude_filter` removes it (0 hits in the logcat of build `d1ca7fb`).
- [ ] fps / memory in Genesis against the phase-2 baseline. The Moto crash family (Mali / low RAM, see
  memory `qa-android-teleport-crash`) was not re-measured.

Known engine bugs and leftovers
- `import-assets` exits with a crash on a warm `godot/.godot` (editor-import teardown). Output is complete.
- A 404 profile is logged as an error (Sentry quota risk).
- Deep-link routing is dropped from the engine outside host mode.
- By-name calls (`call`, `call_deferred`, `callable` with a string): about 90 sites. They are now guarded by the unit test `call_targets_test` (a missing target or a static `#[func]` called on an instance fails `cargo test`); the builtins stay as strings on purpose, the plugin singletons have no other way. Converting the own-method `call_deferred("x")` calls to typed `defer` closures is optional clean-up.
- Two autoloads remain (`Global`, `GlobalBoot`), both native, by design (see the exit criterion in `phase-5-engine-port.md`). `EngineHost` is created by the boot node and the empty-parcel props scene is instantiated on demand (unverified on device: floating-island props).
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
