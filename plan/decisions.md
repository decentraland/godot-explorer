# Decisions

Decided ones come from the proposal or from this plan's design and are not reopened without a new
fact. Open ones have a recommendation, an owner and a date; the date is the latest the work can
proceed without the decision.

## Decided

| # | Decision | Why |
|---|---|---|
| D-1 | Three layers, two languages: RN shell, Rust core, Rust+Godot engine. GDScript retired. | The language with no types and no tests holds the business logic; the four largest files sit at the lint cap. |
| D-2 | One app state machine, in the core. The shell renders it, the engine executes it. | Today's two copies (`engine.ts`, `rn_bridge.gd`) already disagree during transitions. |
| D-3 | The shell and the engine never talk to each other. The host owns the engine *view*, the core owns the engine *state*. | Keeps the engine commandable from desktop with no RN and from RN with no bridge. |
| D-4 | Desktop is a scene-only emulator. No product UI on desktop. | Product targets are Android and iOS; the emulator runs the exact shipped engine for creators and CI. |
| D-5 | Typed FFI generated from Rust (uniffi), callback interface for events. No JSON strings. | Schema and types at the boundary; keys never cross into JS. |
| D-6 | One core per process: the FFI symbols are exported from the same library as the GDExtension (`libdclgodot`). No separate core library. | Two shared libraries would statically link two cores. See `02-target.md`. |
| D-7 | Workspace layout: `lib/crates/dcl_core` (rlib) + `lib/crates/dcl_engine` (`[lib] name = "dclgodot"`). The artifact name, `.gdextension`, sync scripts and symbol upload do not change. | Smallest change to xtask, CI and the RN sync scripts. |
| D-8 | Two caches on purpose: content cache (core policy, engine consumers) and RN's image cache for UI thumbnails. | Different lifetimes and formats; unifying buys nothing. |
| D-9 | Analytics through the core only. No Segment RN SDK. | Two SDKs means two anonymous ids and a broken session chain. |
| D-10 | Public read-only GETs stay in the shell with hosts from `core.urls()`. Signed requests are typed core functions, never a generic signed fetch across the FFI. | RN's data layer is good at this; request shapes stay out of the UI; the `fetch.signed` allow-list goes away. |
| D-11 | The backpack is the only screen with a transparent engine surface above RN content. | Every other screen is RN over an opaque engine or no engine at all. |
| D-12 | The engine starts once and lives until the process dies. Everything that must survive leaving a world lives in the core. | Godot cannot restart in-process. |
| D-13 | **Sequencing, not a calendar.** Phases 1–4 are one developer plus AI sessions and end at the feature-complete gate (parity checklist green against 1.15). Then the full team does phase 5 (new features) and phase 6 (full testing and polish), cuts the release candidate and ships 2.0.0. Phase 7 after. Revised 14 Oct 2026: the dates and size estimates are dropped because the work is AI-assisted. LMT3 is a marketing campaign on 1.15–1.17 and is not touched. | |
| D-14 | The `.gdlintrc` caps are never raised on this branch. | The files at the cap are being deleted. |
| D-15 | The work lives on `feat/react-native`, draft PR #3059. PRs for work items target that branch; #3059's body points at this folder from 1.0 on and is retitled when it is ready for review. It is the React Native app, or the side project. | Decided 13 Oct 2026. |
| D-16 | **One crash reporter: `@sentry/react-native` owns native** (sentry-cocoa and sentry-android under it). Rust panics and engine errors are forwarded through the core as `report` events and captured in the shell with today's tags (`crash_context.rs`). The `project_main_loop.gd` policy (per-source keep rates, remote rate from flags) moves to `dcl_core::report` with tests. sentry-godot is dropped from the app; the emulator has no Sentry. Symbol upload for `libdclgodot` and the engine keeps its CI path, verified with a deliberate crash before the addon goes (risks R-7). Applied in 3.9. | Two native Sentry SDKs in one process fight over crash handlers. Decided 13 Oct 2026. |
| D-17 | **FFI glue: `uniffi-bindgen-react-native`.** The Rust side is uniffi either way (D-5); the generated C++/JSI TurboModule and TS types come straight from the uniffi metadata in `libdclgodot`. The five checks in phase 3 item 3.1 are its acceptance; a failed check is a blocker to fix inside the time box, and only if it cannot be fixed does the hand-written Kotlin/Swift TurboModule come back as a reopened decision. Host-only calls (attest, push token, store, surface order, engine view) stay in a small hand-written module. | Wins every criterion that scales with the size of the API (~70 calls, ~15 events, growing through phases 4 and 5) and lets RN devs consume new core calls without a native dev, which is what keeps the team parallel. Loses on maturity, debuggability and version coupling, which the acceptance checks and the fallback cover. See the comparison below. Decided 13 Oct 2026. |
| D-18 | **Session and settings files: the core's own format.** JSON files under the data dir; no Godot `ConfigFile` anywhere in the core. `settings.cfg` is imported once at first boot (key names kept from `rn/src/settings/keys.ts`) and left in place for one release. | Decided 13 Oct 2026. Applied in 1.4 and 4.5. |
| D-19 | **Core data dir = Godot's `user://`** on each platform, passed by the host; verified by upgrading a device from the store build (R-6). | Decided 13 Oct 2026. Applied in 3.4 and 6.3. |
| D-20 | **GDScript tests** are ported by subject when the subject is ported and deleted with it; no standalone effort. | Decided 13 Oct 2026. Phase 7. |
| D-21 | **No feature missing at 2.0.0.** Push and local OS notifications, in-app purchases and credits on iOS, wallet sign-in, share sheet and calendar all ship. For 2.0.0 the host module calls today's native plugin code (Firebase/APNs, StoreKit, Reown, share, calendar) so nothing is rewritten before the release; RN libraries replace that code in phase 7. Expo Go ends with the FFI regardless; the no-engine native builds with the mock core replace it. | Revised 13 Oct 2026: the 2.0.0 release has nothing a 1.15 user had missing. |
| D-22 | **Core math type: `glam`**, `From` impls at the engine edge. | Decided 13 Oct 2026. With 1.3, in phase 7. |
| D-23 | **Engine host template from the branch build.** xtask already pulls engine builds from a fork branch (`GODOT_USE_BRANCH` in `src/consts.rs`, published by engine CI under `/branches/<slug>/`); the engine branch `spike/ios-host-embedding` publishes `ios_host_release.a` and its CI is green at `927097bdd`. Set `GODOT_USE_BRANCH = Some("spike/ios-host-embedding")` on this branch so `install --targets ios` fetches it, and verify the install step takes the host template from the branch path. Reset the override and bump `GODOT_BUILD_SHA` at the RC cut (6.7); `main` keeps `None` until then. | Decided 13 Oct 2026. With 3.4. |
| D-24 | **The emote wheel stays in the engine**, with the joystick and gamepad; `organisms/emotes` ports to Rust in 7.3. The backpack still plays emotes through `play_emote`. | Decided 13 Oct 2026. |
| D-25 | **Every sign-in method ships in 2.0.0**: guest, email OTP, Google and Apple through the browser hand-off (already Rust), MetaMask on Android and WalletConnect on iOS through today's native code called from the host module. An RN wallet library (or a Magic Link RN SDK) replaces that code in phase 7. | Revised 13 Oct 2026. 3.6. |
| D-26 | **Three languages at 2.0.0**: en, es, pt_BR, generated from `godot/locale/*.csv`. | Revised 13 Oct 2026. 4.15. |
| D-27 | **Version 2.0.0.** `main` keeps releasing 1.16, 1.17… for LMT3 until 2.0.0 ships. Rollout, rollback and the asc-deploy switch are decided later (D-open-14). | Decided 13 Oct 2026. |
| D-28 | **Desktop after 2.0.0:** the emulator is published under its own name for creators and QA; desktop is no longer presented as a client; the last Godot-UI desktop build stays downloadable; product writes the message. | Decided 13 Oct 2026. 7.10. |
| D-29 | **Avatar preview is live only.** No low-end mode in 2.0.0; a snapshot fallback is a possible future feature, not a plan item. | Decided 13 Oct 2026. |
| D-30 | **No named owners until phase 5.** Owners per layer and per domain are assigned at onboarding (6.0). | Decided 13 Oct 2026. |
| D-31 | **Side project on this branch until the release candidate.** No early merge of phase 1 into `main`. A scheduled automated job (an AI session) merges `main` into PR #3059 every day, skipping UI-only changes and resolving deleted-`.gd` conflicts as deleted; a fix that matters is redone in RN or Rust. The branch merges to `main` at the RC cut (6.7). | Decided 13 Oct 2026. README "How work lands", R-9. |
| D-32 | **Reduced scope of the carve-out.** Comms transports (1.7), the content cache policy (1.8) and the Godot math types (1.3) stay in `dcl_engine` for 2.0.0 and move in phase 7 (7.0). They only matter in-world, where the engine exists; chat and presence reach the shell through the engine's event sink. | Decided 13 Oct 2026: one developer to 30 Nov; the user-facing goal does not need them. |
| D-33 | **The engine-side port is entirely after the release** (phase 7). 2.0.0 ships with the player, avatar, SDK components and fetcher still in GDScript behind the Rust `Engine` facade. | Decided 13 Oct 2026. Internal debt, not a user-facing gap. |
| D-34 | **New features by the team, after the gate, on the finished core**: Challenges, Gallery, Discover v2, Android IAP + Web Shop, Badges + Scrapbook. Each needs its spec (Figma, endpoints, metrics) before the gate so the core calls are in the FFI before phase 5. Built exactly like a phase-4 domain. | Decided 13 Oct 2026. Phase 5. |
| D-35 | **Phase 6 is test and polish first.** The team's implementation work is phase 5 only; everything else in phase 6 is the parity checklist, the device matrix, the upgrade test, performance, store readiness and the RC. The month doubles as onboarding to the architecture (6.0). | Decided 13 Oct 2026. Phase 6. |
| D-36 | **Quest is dropped and everything VR goes with it**, after the release (7.12): the `quest` xtask target, the Quest export preset and XR keys, the `[xr]` project settings, the OpenXR toggling in `dcl_build_plugin`, any XR code path in GDScript or Rust, and the CI and docs mentions. Not now: nothing VR is touched before 2.0.0. | Decided 13 Oct 2026. Nobody ships or tests the Quest build. |

## Open

| # | Question | Recommendation | Owner, decide by |
|---|---|---|---|
| D-open-5 | **Editor addons** (4,565 lines of GDScript, editor-only, never shipped). What each is: `dcl_build_plugin` (141) is an export plugin (XR mode for Quest, Android deps extraction, excludes editor-only addons from exports). `dcl-godot-android` (76) is the export plugin that registers the Android plugin (ExoPlayer stays). `dcl_mobile_preview` (779) previews GDScript UI scenes in the 2D editor with device frames; it is **not** the runtime phone emulation (that is `--emulate-ios/android` in `global.gd`, kept by the Rust root in 2.6) and is excluded from exports. `dcl-ios-devtools` (161) injects local-network Info.plist keys into Godot-exported debug iOS builds. `emote_converter` (267) is a tool-menu item converting GLB emotes to `.tres` libraries. `resource_tracker` (1,217) is an editor profiler. `dcl_dev_tools` (1,924) is the DCL Tools menu (Rust Log Filter and others) plus `resource_counter.gd`, loaded at runtime by `loading_screen.gd` and `benchmark_flow_controller.gd`. `sentry` is binaries. | Both export plugins exist only to drive Godot's own Android Gradle build, which the RN pipeline still runs to get the engine AAR, `libwebrtc.jar`, `libdclgodot.so` and the assets (`sync-godot-android.sh`); the RN app declares the Maven dependencies itself in `godot.gradle`. Keep them, exempt from the `.gd` rule, until xtask produces those four inputs without Godot's Gradle (`--export-pack` plus copies from the template, the deps zip and the cargo build), then delete both and `godot/android` with them (7.9). Drop `dcl_mobile_preview` (no GDScript UI left to preview), `resource_tracker`, `dcl-ios-devtools` (the RN app owns its Info.plist), `sentry` (D-16), and `dcl_dev_tools` once `resource_counter.gd` is ported for the benchmark (7.7). Keep `emote_converter` until the base emotes next change, then port it to xtask. | EM. 7.9 applies it. |
| D-open-14 | **Rollout and rollback for 2.0.0.** Staged percentages, the kill criteria, how long the 1.x line stays releasable, when `godot-asc-deploy` `main` switches to the RN workspace (the dispatch targets its `feat/react-native-app` today). | Play 10% → 50% → 100% over two weeks and an iOS phased release, each step gated by the daily crash KPI against the 1.x baseline; keep the 1.x pipeline releasable for one cycle; asc-deploy switches at the RC cut. | EM. By mid-December (6.8). |

## D-17: the comparison behind it

(a) `uniffi-bindgen-react-native` against (b) uniffi Kotlin/Swift bindings plus a hand-written
TurboModule. Both sit on the same Rust API.

| Criterion | (a) generated | (b) hand-written glue | Edge |
|---|---|---|---|
| Hops per call | JS → generated C++/JSI → Rust | JS → Kotlin/Swift TurboModule → uniffi binding → Rust (JNI on Android) | a |
| Code per core call | None beyond the Rust signature | TS spec + Kotlin + Swift + marshalling | a |
| Type safety | TS generated from Rust; a signature change fails `tsc` | TS spec by hand; nothing checks native against Rust | a |
| Async and events | Generated promises and JS-implemented callback interfaces | Hand-bridged promises; `NativeEventEmitter` with hand-typed payloads | a |
| Complex types | Records and enums become TS objects through JSI | Codegen types are limited; nested data tends to degrade into JSON strings | a |
| Adding a call in phases 4–5 | Rust, rebuild, regenerate | Four files, two platforms, test on both; about an hour each across ~70 calls | a |
| Who can add a call | Any Rust or RN dev | Needs a native dev per call, serializing the team | a |
| Runtime cost | Fewer copies, no JNI | Extra conversions | a, irrelevant by design: the hot path never crosses the FFI |
| First call working | Slower: C++ toolchain, RN 0.86 compat | Faster: known pieces | b |
| Maturity, community | 0.x, small maintainer base | Firefox-mobile-grade uniffi output; TurboModules are RN's own | b |
| Version coupling | Pinned uniffi; must track RN releases | Independent upgrades | b |
| Debugging a glue bug | Generated C++ and JSI | Kotlin/Swift breakpoints | b |
| Single library (D-6), events from tokio threads | Must be verified (checks 2 and 3) | Straightforward | b |
| Host-only calls | Small hand-written module | Same | tie |
| Exit cost | Fall back to (b), Rust unchanged | None | tie |
