# Phase 5b: solo polish

Mateo + AI, after phase 5, before the full-migration gate. Phases 6 and 7 keep their numbers; this one is
inserted between 5 and 6 and takes no number from either.

## Goal

One person closes what phases 1–5 left open and what the 19 Oct audit found, so the team starts phase 6 on
a build that boots, is observable and has no known crash path. It is the "solo polish" of D-45: the
checks that need one person who knows the whole tree, not a team's device matrix. Nothing here adds a
feature. Items the team does better (device matrix, signed-account runs, Figma review) stay in phase 6.

## Exit criteria

- [ ] Every item below is ticked or moved to phase 6 or 8 with its reason written next to it.
- [ ] A release build produces readable JS and native stacks in Sentry for a deliberate crash on both
      platforms (5b.5, R-7).
- [ ] No known panic path reachable from scene input (5b.8).
- [ ] The open items still listed in `phase-1`–`phase-5` files and in
      [phase-5-verification.md](phase-5-verification.md) are each either done here or handed on by name.

## Work items

### A. Carried from phases 1–5 (not duplicated; the source file keeps the detail)

- [ ] **5b.1 Close the open boxes of phases 1–3 and 5.** Phase 1 regression on desktop + APK; 2.8 IME
  check on both phones (R-3); 2.9 release-build fps and memory, RN host vs Godot-exported app (R-1);
  3.1 check that the TurboModule and the GDExtension share one library handle (R-5); 3.4 native hosts;
  3.9 one native crash handler (D-16, R-4); 3.10 CI; 3.11 avatar preview fast path; the Idle/World service
  boundary and 5.10 clean-up in phase 5.
- [ ] **5b.2 Work the 4.16 parity checklist rows that need one person, not a matrix**: the "Not run" and
  "Left" cells in [phase-4-parity.md](phase-4-parity.md) that depend on no account and no second phone.
  Rows that need a real login, an iPhone or Figma go to phase 6.
- [ ] **5b.3 Pull in from `phase-5-verification.md`, only if time allows**: `begin_sign_in` while already
  `SignedIn` (RN crash), the 404 profile logged as an error (Sentry quota), deep-link routing outside host
  mode, `dcl_resource_tracker.rs` removal. The rest of that file's "handed to team polish" list stays in
  phase 6 (user decision, 17 Oct).

### B. From the 19 Oct audit

Findings below are the ones the plan did not cover. Decisions already taken with the owner are marked.

- [x] **5b.4 Permissions.** Keep the microphone permissions for the voice chat that will come back (user
  decision). Restore main's iOS `NSMicrophoneUsageDescription` text ("requires your microphone to
  communicate with other players") instead of "does not use the microphone", and add `RECORD_AUDIO` to the
  RN `AndroidManifest.xml`, so the declaration matches the app and the next voice release needs no manifest
  change. **Done 19 Oct:** `RECORD_AUDIO` is in the RN manifest and the iOS microphone text is main's.
- [ ] **5b.5 Readable crashes in Sentry (R-7, extends it to JS).** Wrap Metro with Sentry's config so
  bundles carry a debug ID; upload Hermes source maps from the Android build (`sentry.gradle`) and the iOS
  build phase, using the existing `SENTRY_AUTH_TOKEN`; keep the native symbol upload. Then turn R8 on with
  the Sentry mapping upload, after 5b.9 (the touch router must not depend on a class name). Exit: a
  deliberate JS error, Rust panic and scene crash each show readable frames. **19 Oct:** Metro runs through `getSentryExpoConfig` and the Android build copies the debug ID into the Hermes map (verified in a local release-style build: "Check generated source map for Debug ID"); `android_builds.yml` uploads bundle + map next to the native symbols. Not done: the iOS build phase, R8 (needs the mapping upload and the per-pointer touch routing below) and a deliberate-crash check in Sentry, which needs a `main` or `release` CI build.
- [x] **5b.6 16 KB alignment guard.** Verified on 19 Oct: all 22 `.so` files in the release APK have
  `LOAD` alignment `0x4000` (NDK 28 default). Add a CI step that runs `llvm-readelf -lW` over the APK's
  libs and fails on anything below `0x4000`. **Done 19 Oct:** `.github/scripts/check-16kb-alignment.py` runs in `android_builds.yml` after the export; it passes on the current release APK (22 libraries).
- [x] **5b.7 Licences.** `cargo deny` with an allow-list in `deny.toml` (586 crates today, 18 MPL-2.0) and
  `license-checker --production --failOn "GPL;AGPL"` for `rn/`; generate an acknowledgements screen from
  the output. Known on 19 Oct: npm tree is MIT/ISC/BSD/Apache plus MPL-2.0 build tools and the Sentry CLI
  (FSL-1.1-MIT, dev only). **Done 19 Oct:** `lib/deny.toml` (`cargo deny check licenses`, in `static_checks.yml`) and a `license-checker --onlyAllow` step in `rn_app.yml`. The generated acknowledgements screen is moved to phase 6 (needs a Figma design).
- [x] **5b.8 Panics from scene input.** `dcl_engine/src/dcl/js/fetch/mod.rs:136` parses scene-supplied
  header names and values with `.unwrap()`; an invalid header can panic the scene thread, and a panic
  that unwinds through a V8 callback may abort the process. Return a JS error instead; check whether main
  has the same code. Then review the other scene-reachable `unwrap`/`expect` in `dcl_engine`
  (`dcl/js/events.rs`, `fetch/mod.rs`, `dcl/crdt/`; 296 outside tests, most on internal channels) and wrap the
  scene-thread entry in `catch_unwind` so a scene panic ends that scene, reports to Sentry (the panic hook
  already does) and reaches RN through `scene_crashed` / `engine_notice` as a message, not a dead app. **Done 19 Oct:** the fetch header names and values return a JS error (`dcl/js/fetch/mod.rs`) and the scene thread is wrapped in `catch_unwind` (`dcl/mod.rs`), ending that scene through `RemoveGodotScene` with a system-error log line. The other scene-reachable unwraps were read: the rest sit on internal channels.
- [x] **5b.9 `dcl_core` unwrap review.** 65 `unwrap`/`expect` outside tests (the 237 first counted
  included `#[cfg(test)]` modules). Fix the ones that can fire at runtime: `env/mod.rs:267,273`
  (`RwLock.unwrap()`, no poison recovery unlike the other locks); `http/queue_requester.rs:115`
  (`receiver.await.unwrap()`); `analytics/frame.rs:39,60` (NaN in `partial_cmp`, `first()` on an empty vec);
  `session/ethereum_provider.rs:37`; `core.rs:63,88` (runtime build and `Core::get` before install: return a
  typed error to the shell). While in `MainActivity.kt`: replace the `SafeAreaProvider` class-name check
  with a tag or interface check, route touch per pointer rather than per gesture, drop the per-touch
  `Log.d`, and keep `--perf-log` out of release `BOOT_ARGS`. **Done 19 Oct:** `env` locks recover from poison, `queue_requester` returns an `HttpError` when the response sender is dropped, `frame.rs` sorts with `total_cmp`, `ethereum_provider` has no unwrap; `MainActivity` checks `is SafeAreaProvider`, logs nothing per touch and passes `--perf-log` in debug builds only. Left, with reason: `Core::get` keeps its documented panic (two engine call sites that run after attach), and touch routing stays per gesture: splitting a `MotionEvent` per pointer needs its own multitouch pass on a phone.
- [x] **5b.10 Core → shell event delivery.** The generated bindings call JS with `invokeBlocking`, so the
  delivery thread waits for every JS call and a slow JS thread delays `state_changed` behind old `hud`
  snapshots (4 per second). Keep only the latest `hud` in a slot and flush it before a `state_changed`; no
  other change. `Core::spawn` before `install`: in release, log an error and drop the future; the fresh-runtime
  fallback stays for `#[cfg(test)]` only. A failed delivery-thread spawn (B2) is not worth work. **Done 19 Oct:** only the newest `hud` waits in a slot and is flushed ahead of any `state_changed` (`ffi/mod.rs`, unit-tested); `Core::spawn` before install drops the future with an error in release.
- [ ] **5b.11 Hang reporting (R-7 follow-up).** After D-16 an engine-thread hang may not reach Sentry.
  Block the engine thread for six seconds on both platforms and confirm an ANR / app-hang event arrives. **19 Oct:** not run: nothing in the engine can block its thread on demand, and adding a debug command for one test was not worth it. Handed to phase 6 with the Sentry crash check.
- [x] **5b.12 Cold-start deep links.** A link that arrives before the core, the RN root or the engine is
  ready is queued in the core and applied once the state allows it. Test cold, warm and background starts on
  both platforms for `decentraland://`, `decentraland.org/jump|events|places` and `mobile.dclexplorer.com`. **19 Oct:** found and fixed a real gap: a link that arrived before the player was past the entry flow was dropped (`App.tsx` ignored it when not `entered`, `links.ts` dropped it with no subscriber), so cold-start `jump` links only opened Discover. Both now hold the latest link and open it once the shell is up (jest-covered). Verified on the Moto (19 Oct): a cold start from `https://decentraland.org/jump/?position=10,10` now lands in the world at 10,10 (before: Discover). Warm and background starts and the `decentraland://` and `mobile.dclexplorer.com` forms were not run.
- [x] **5b.13 Android lifecycle.** Kill-and-restore and memory trim on the Moto: `startEngine` must not
  create a second Godot after process death (guard on the Godot instance, not the fragment field);
  forward `onTrimMemory` to the engine if Godot does not. **19 Oct:** trim-memory (`RUNNING_CRITICAL`) with the world open: same pid, no crash. The activity-recreation guard on the Godot instance was not changed and not reproduced; process death restarts the process, so the existing check suffices there.
- [ ] **5b.14 Smoke tests that run without a person.** Add a Maestro suite (launch, guest sign-in, jump in,
  open backpack) and run it on PRs that touch `rn/`. It is the only automated guard of the screens once the
  GDScript tests are gone (D-20). Write the rollback plan for D-open-14 next to it: keep writing the files
  1.x reads for one release, and a staged rollout with halt criteria. **19 Oct:** `rn/.maestro/smoke.yaml` (fresh install, Terms, guest, avatar, Continue, tutorial Skip, in the world) passes end to end on the Moto. Not done: running it on PRs, which needs an emulator that runs the engine (phase 6). Rollback plan (D-open-14): keep writing every file 1.x reads for one release (session, settings, caches under the same data dir, R-6), ship 2.0.0 as a staged rollout (1%, 10%, 50%, 100%) and halt on a crash-free-sessions drop of more than 1 point or any new top-3 Sentry issue against the 1.16 baseline; the stores cannot downgrade, so rollback means a 1.x hotfix built from the permanent `1.x` branch (D-40) with a higher version code.
- [x] **5b.15 Supply chain.** Pin exact versions of `rn/` dependencies (many are `^`, `@ubjs/core` is a
  pre-release), add `npm audit` to `rn_app.yml`, and write down which core calls JS may make without a user
  action (signed requests, session export). **Done 19 Oct:** every `rn/package.json` range is an exact version (lockfile unchanged in substance), `npm audit --omit=dev --audit-level=critical` runs in `rn_app.yml` (34 high findings remain, all in build tooling with no upstream fix), and the list of core calls JS may make is in `rn/README.md`.

### Not scheduled (decided 19 Oct)

- Binary size budget: not a priority, 1.x is 230 MB and OTA can absorb it later. The release profile keeps
  `debug = 1`, `strip = false`, so `libdclgodot.so` is 154 MB; stripping it with symbols uploaded to
  Sentry is the easy win when size matters.
- Accessibility labelling and a font-scale pass: phase 6.
- Contributor tooling (`doctor` for the mobile toolchain), crash-rate comparison cohort: phase 6.

## Verification

Run on the Moto G60 and, where noted, an iPhone; log what was run in
[phase-5-verification.md](phase-5-verification.md) the same way phase 5 did. Unticked means not run.

## Depends on

Phase 5 closed (17 Oct 2026). 5b.5's R8 step depends on 5b.9's touch-router change. Ends where the
full-migration gate and the 1 Nov checkpoint (D-39) read the state.
