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
- [ ] **Avatar snapshots.** Client tests `avatar_no_outline` / `avatar_with_outline` score 64% / 63% against the April references (bloom and uibackground pass at 100%). Side by side: the current render is zoomed in more than the reference, and its eyes are white where the reference has green irises. Checked: the facial path (`apply_color_and_facial`, `get_wearable_facial_hashes`, `apply_texture_and_mask`) is the old GDScript line for line, the test sets the same colours, and the textures load; the eyes wearable currently resolves to one texture with no `_mask.png`, so there is nothing to tint. The references are probably stale against today's remote wearable content (the test fetches it from the network). Not verified. Fix: render the references from a local fixture so the test stops depending on remote content. **Deferred to phase 8 (user decision, 17 Oct: no offline wearable source yet).**
- [ ] **Lights (5.5).** The light-source port compiles and loads; rendering was never compared against 1.15. **Moved to team polish (user decision, 17 Oct).**
- [ ] **Video (5.5).** URL video, HLS and LiveKit streams, in background and foreground: not run. **URL video verified on the Moto; HLS and LiveKit moved to team polish (user decision, 17 Oct).**
- [ ] **Scene tests** (`--stest`), 17 Oct, desktop, local test realm (`coverage.yml`'s `@dcl/scene-explorer-tests` on :7666, `-- --scene-test [...13 parcels] --realm http://localhost:7666/scene-explorer-tests`; plain `run --stest` starts no test). Three fixes, none of them a port regression: (1) the old harness hung forever on a found scene that reports nothing (the live catalyst now has a testless scene on 52,-52): a found scene now fails after 180 s; (2) test mode loads the realm's own `scenesUrn` scenes only (no catalyst pointer walk), so the catalyst can no longer put another scene on the test parcels; (3) the player teleports to every parcel, since a loaded scene may only run its tests with the player there. Result: all 13 scenes report, 26 tests pass (raycast 12/12 among them), exit 1. The failures are the snapshot comparisons and tests that render avatars, models or UI (gltf-container, avatar-attach, material avatar portrait, ui-background, ui-text) against `tests/snapshots`, whose references are old (user's note) and some of which need remote models; they say nothing about a regression until the references are regenerated from this build (phase 8, with the avatar snapshots).
- [x] **Asset renderer** (`-- --asset-renderer --use-test-input`, 17 Oct, desktop): exit 0, `report.json` lists the three jobs (t-shirt front/34/back, sunglasses worn, wave emote k25/50/75) as `ok`, eight files written. Debug hub: verified on the Moto (`ping`, `state`, `teleport`). [x] **GP benchmark** (`-- --gp-benchmark`, desktop, 17 Oct): config loaded, load 12.6 s, 60 s settle timeout, 30 s warmup, 10 s sampling (3720 samples, mean 390 fps in a debug build), `baseline.json` and `baseline.png` written, result JSON printed, clean quit then the designed force-kill (exit 9). Two things look off and were not chased: `loaded_scene_count` reads 0 with a scene loaded (`scene_fetcher.loaded_scenes` in this realm), and settling never sees a loading container, so it always waits the full 60 s.
- [x] **CI benchmark** (`--benchmark-report`, the `benchmark.yml` run): it had been a no-op since phase 4 deleted `benchmark_flow_controller.gd` (nothing recorded metrics, the run idled until the timeout) and its feature set did not compile (gdext 0.5 `Dictionary::set`, `ffi` missing). Ported as `tools/benchmark_flow.rs` (six locations, Goerli/Genesis x3, load wait, 20 s settle, CSV, quit); run 17 Oct on desktop: exit 0, `benchmark_report.csv` with six rows. The terms/lobby/menu stages are gone with those screens, and the mesh/material counts are zero (the resource counter was removed).
- [ ] **Creator preview** (`-- --preview` against the SDK example scenes) after the 5.6 fetcher port. **Moved to team polish (user decision, 17 Oct).**
- [x] **Idle → World → Idle → World** on the Moto (17 Oct, build `c0ea5d3`, branch editor): Genesis (static memory 397 MB) → Back → Leave → Discover (220 MB, 0 scenes) → Jump in → Genesis again (403 MB); no growth across the cycle, no panic, "call failed" or "freed" in logcat.
- [x] **`--features use_memory_debugger`** build: it did not compile (28 errors from the gdext 0.5 move, in `benchmark_report.rs` and `memory_debugger.rs`); fixed, clippy `-D warnings` passes with the feature. Not run on a device.
- [x] Device, Moto G60, build of the final tree (16 Oct): avatar preview → name → Genesis (~30 fps) → emote wheel opens, joypad buttons render, camera swipe turns the view, chat panel opens; no panic, "already bound" or "Scene error" in logcat.
- [x] Moto, later builds (16 Oct, `7199c95` and the two-autoload build): joystick walks the avatar, Back → Leave dialog → Discover → Jump in again re-enters Genesis, the pointer tooltip ("Talk") shows on an NPC; no panic or "Method not found" in logcat. fps 17–30 in a crowded plaza.
- [x] Moto: selecting a slice of the emote wheel plays the emote (the avatar raises an arm, the wheel closes), no panic. fps was 10–13 in a crowded plaza at that moment.
- [x] Moto, build `21f9014` (after the typed-signal refactor and three fixes it exposed): enter Genesis, camera swipe, emote wheel selection, Back → Leave → Discover → Jump in → Genesis again; no panic, "call failed" or "after it has been freed" in logcat, 17–28 fps. The fixes: `LoadingSession::ready` connects one signal per `signals()` call (loading hung at 30% before), `Promise` settles through `defer_from_any_thread`, and a freed camera handle is never cloned after Leave. None of the three showed in `--itest` (25/25): wiring done in `ready` and teardown only run in the full app, so the device run is the check.
- [x] Moto, build `e522839` (`local_task` on gdext's `godot::task`): same cycle, enter → swipe → emote → Leave → re-enter, no panic and no bind-guard-across-await warning in logcat, 18 fps in the crowded plaza.
- [x] Debug hub on the Moto (`adb reverse 9231`, `cargo run -- debug-hub`): the typed commands `ping`, `state` (realm, fps, scenes, parcel, player position) and `teleport {x,y}` answered from the device; `eval` is gone by design.
- [x] Moto, build `d1ca7fb` (fresh install, guest, Genesis, teleport to 12,12 over the debug hub): the empty-parcel land around a scene renders its props (trees, rocks, plants, red grass), 52–60 fps, no panic, "call failed" or "freed" in logcat. A URL video (mp4) played through ExoPlayer in the same session (backend initialised, `READY`, GPU surface 854x480, released on leaving). HLS and LiveKit streams were not run.
- [x] On the name screen the avatar rendered small in a large hole: not an engine framing problem. `PreviewArea` always applied the create screen's `hole` margins (`marginTop u(130)`, `marginBottom u(250)`), which left an 87 dp window inside the naming screen's 506 dp box; the engine fitted the avatar to the 624x145 rect it was given. Fixed in RN (`PreviewArea inset={false}` on the naming screen); verified on the Moto (17 Oct) with a temporary local flow change that sent a guest to the naming screen: reported hole 376x545 dp, the avatar fills it. The earlier suspicion of the camera's minimum size was wrong. The same stale-framing symptom in the client snapshot test is a separate item (phase 8).
- [x] The `dcl_swift_lib.gdextension` boot error is gone on Android: the preset's `exclude_filter` removes it (0 hits in the logcat of build `d1ca7fb`).
- [x] fps / memory in Genesis against the recorded baseline (risk R-1: 11-15 fps on the debug template, 14-18 on release, RN host on the Moto). Phase 5 builds on the Moto, 17 Oct: Genesis 10-18 fps, 397-448 MB static memory: inside the recorded range, so no regression is visible. The R-1 comparison against the Godot-exported app from the same commit is not a phase-5 item (2.9 / 6.5).
- [x] Mali / low-RAM crash family, stress run (Moto, 17 Oct): eight teleports between Genesis, Goerli, (-9,-9), 52,-52 and 100,100, 25 s apart, no restart (same pid), no SIGSEGV / SIGABRT / fatal signal / ANR in logcat; Genesis static memory returned to 448 MB each time (no growth), the other parcels sat at 250-290 MB. fps stayed 10-11 in that session.
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
- [x] The name screen's avatar hole kept a small rect: fixed in RN (see above), verified on the Moto.
- [ ] RN crashes on `begin_sign_in` while already `SignedIn`. **Polish stage (RN, not phase 5).**
- [ ] Push notifications, reminders, error modals, settings relaunch on a phone. **Polish stage (RN, not phase 5).** (Joystick movement is verified, see the Moto lines above.)
- [x] es / pt_BR follow the device (Moto set to pt-BR, 17 Oct): the shell showed English because `getAll` dropped the empty locale ("follow the device") and filled the English default, so the engine's rule never reached RN; `coreApi.getAll` now falls back to the device locale (`localeFromDevice`, same mapping as the engine, every `pt` to `pt_BR`). Verified: Discover reads "Explorar" and the friends row "Amigos". **Translation gap, not code:** the entry-flow screens (terms, account, avatar, name, welcome) have no es / pt_BR strings at all (`i18n/pt_BR/flow.ts` is empty), so they stay English; a translator's task for the polish stage.
- [ ] OTP / Google / Gmail logins, native wallets and friends (moved to the polish stage).

iOS: **never built.** Every iOS-affecting change in phases 3–5 is unverified and needs the user's hands
(Xcode, Metal toolchain, Firebase iOS files, the `ios_host_release.a` template).

## Fork API pin (5.0)

- [x] `decentraland/godotengine` #29 (`spike/ios-host-embedding`, commit `333404e`): `process_templates.yml` dumps `extension_api.json` and `gdextension_interface.h` with the Linux editor and uploads them with the editors. Verified: the PR's CI run published `https://godot-engine-releases.dclexplorer.com/branches/spike-ios-host-embedding/editors/extension_api.json` (4.6.2.stable.gh, 6.7 MB), and it carries `Label3D::set_bbcode_enabled`, `set_bold_font`, `set_outline_size_float`.
- [x] Explorer side (user decision, 17 Oct: `GODOT_USE_BRANCH = Some("spike/ios-host-embedding")` for the whole RN refactor, back to the pinned SHA when #29 merges in phase 7): `install` downloads `extension_api.json` into `.bin/godot/`, the `godot` crate has `api-custom-json`, `lib/.cargo/config.toml` points `GDRUST_GODOT_API_JSON` at it, and `text_shape/node.rs` / `dcl_rich_ui_text.rs` call the fork methods typed (the `has_method` probes and four allowlist entries are gone). Verified: clippy, 511 unit tests. Also run with the branch editor (`4.6.2.stable.gh.333404e8e`): `--itest` 25 passed, 0 failed; Moto: boot, avatar, Genesis, a scene's 3D TextShape renders through the typed fork calls, teleport to 12,12 shows the props, no panic in logcat.
- Avatar snapshots: no offline wearable source for now, moved to phase 8 (user decision).
- Lights, video (HLS, LiveKit), creator preview: moved to team polish (user decision).
- iOS: tested by the user on another machine.

## Dev features removed (not ported, on purpose)

`--spawn-avatars`, `--emote-test`, the GDScript test scenes and `godot/examples`, the safe-margin overlay,
the scene and lighting renderers, the stress test, the FI / impostor benchmarks, the GP benchmark's
exploratory memory walkers and mesh-merge classification, GDScript `eval` (typed commands `state`, `teleport`,
`screenshot`, `log_filter` instead), the iOS dev localhost relay, and the editor addons `dcl_dev_tools`,
`dcl_mobile_preview`, `resource_tracker`, `dcl-ios-devtools`.

## Gate

The 30 Nov full-migration gate is judged after this checklist is run, not before. PRs #3072 and #3073 are
still unreviewed and unmerged; #3081 stacks on both.
