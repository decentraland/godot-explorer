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
| D-13 | Oct–Nov is a side-project on this branch; nothing ships before December; December is the full team; January is the release. Scope gate the week of 23 Nov 2026. | LMT3 is untouched. |
| D-14 | The `.gdlintrc` caps are never raised on this branch. | The files at the cap are being deleted. |
| D-15 | The work lives on `feat/react-native`, draft PR #3059. PRs for work items target that branch; #3059's body points at this folder from 1.0 on and is retitled when it is ready for review. It is the React Native app, or the side project. | Decided 13 Oct 2026. |
| D-16 | **One crash reporter: `@sentry/react-native` owns native** (sentry-cocoa and sentry-android under it). Rust panics and engine errors are forwarded through the core as `report` events and captured in the shell with today's tags (`crash_context.rs`). The `project_main_loop.gd` policy (per-source keep rates, remote rate from flags) moves to `dcl_core::report` with tests. sentry-godot is dropped from the app; the emulator has no Sentry. Symbol upload for `libdclgodot` and the engine keeps its CI path, verified with a deliberate crash before the addon goes (risks R-7). Applied in 3.9. | Two native Sentry SDKs in one process fight over crash handlers. Decided 13 Oct 2026. |
| D-17 | **FFI glue: `uniffi-bindgen-react-native`.** The Rust side is uniffi either way (D-5); the generated C++/JSI TurboModule and TS types come straight from the uniffi metadata in `libdclgodot`. The five checks in phase 3 item 3.1 are its acceptance; a failed check is a blocker to fix inside the time box, and only if it cannot be fixed does the hand-written Kotlin/Swift TurboModule come back as a reopened decision. Host-only calls (attest, push token, store, surface order, engine view) stay in a small hand-written module. | Wins every criterion that scales with the size of the API (~70 calls, ~15 events, growing through phase 4) and lets RN devs consume new core calls without a native dev, which is what keeps December parallel. Loses on maturity, debuggability and version coupling, which the acceptance checks and the fallback cover. See the comparison below. Decided 13 Oct 2026. |
| D-18 | **Session and settings files: the core's own format.** JSON files under the data dir; no Godot `ConfigFile` anywhere in the core. `settings.cfg` is imported once at first boot (key names kept from `rn/src/settings/keys.ts`) and left in place for one release. | Decided 13 Oct 2026. Applied in 1.4 and 4.5. |
| D-19 | **Core data dir = Godot's `user://`** on each platform, passed by the host; verified by upgrading a device from the store build (R-6). | Decided 13 Oct 2026. Applied in 3.4. |
| D-20 | **GDScript tests** are ported by subject when the subject is ported and deleted with it; no standalone effort. | Decided 13 Oct 2026. Phase 5. |
| D-21 | **OS-intake features are removed for 2.0.0**: push and local OS notifications, in-app purchases and credits, wallet connect, share sheet, calendar, and the Godot plugins that provided them. They return in phase 6, in RN and the core, after the refactor. Expo Go ends with the FFI regardless; the no-engine native builds with the mock core replace it. | Decided 13 Oct 2026. Phase 4 deletes, phase 6 rebuilds. |
| D-22 | **Core math type: `glam`**, `From` impls at the engine edge. | Decided 13 Oct 2026. 1.3. |
| D-23 | **Engine host template from the branch build.** xtask already pulls engine builds from a fork branch (`GODOT_USE_BRANCH` in `src/consts.rs`, published by engine CI under `/branches/<slug>/`); the engine branch `spike/ios-host-embedding` publishes `ios_host_release.a` and its CI is green at `927097bdd`. Set `GODOT_USE_BRANCH = Some("spike/ios-host-embedding")` on this branch so `install --targets ios` fetches it, and verify the install step takes the host template from the branch path. Reset the override and bump `GODOT_BUILD_SHA` when the engine branch merges in December; `main` keeps `None`. | Decided 13 Oct 2026. With 3.4. |
| D-24 | **The emote wheel stays in the engine**, with the joystick and gamepad; `organisms/emotes` ports to Rust in 5.3. The backpack still plays emotes through `play_emote`. | Decided 13 Oct 2026. |
| D-25 | **Sign-in at 2.0.0: guest and email OTP only.** Google, Apple, MetaMask and WalletConnect buttons show "not available in this version"; the browser hand-off and the wallet flows are not wired. Providers return in phase 6, purely in RN (a Magic Link RN SDK is a candidate). | Decided 13 Oct 2026. 3.6 and 6.1. |
| D-26 | **2.0.0 ships English only.** es and pt_BR come back in phase 6, regenerated from `godot/locale/*.csv`. | Decided 13 Oct 2026. 6.5. |
| D-27 | **Version 2.0.0.** `main` keeps releasing 1.16, 1.17… until 2.0.0 ships. Rollout, rollback and the asc-deploy switch are decided later (D-open-14). | Decided 13 Oct 2026. |
| D-28 | **Desktop after January:** the emulator is published under its own name for creators and QA; desktop is no longer presented as a client; the last Godot-UI desktop build stays downloadable; product writes the message. | Decided 13 Oct 2026. 5.10. |
| D-29 | **Avatar preview is live only.** No low-end mode in 2.0.0; a snapshot fallback is a possible future feature, not a plan item. | Decided 13 Oct 2026. |
| D-30 | **No named owners now.** Owners per layer and per domain are decided in December. | Decided 13 Oct 2026. |
| D-31 | **Side project on this branch until December.** No early merge of phase 1 into `main`. A scheduled automated job (an AI session) merges `main` into PR #3059 every day, skipping UI-only changes and resolving deleted-`.gd` conflicts as deleted; a fix that matters is redone in RN or Rust. In December the branch merges with as much as possible. | Decided 13 Oct 2026. README "How work lands", R-9. |

## Open

| # | Question | Recommendation | Owner, decide by |
|---|---|---|---|
| D-open-5 | **Editor addons** (4,565 lines of GDScript, editor-only, never shipped). What each is: `dcl_build_plugin` (141) is an export plugin (XR mode for Quest, Android deps extraction, excludes editor-only addons from exports). `dcl-godot-android` (76) is the export plugin that registers the Android plugin (ExoPlayer stays). `dcl_mobile_preview` (779) previews GDScript UI scenes in the 2D editor with device frames; it is **not** the runtime phone emulation (that is `--emulate-ios/android` in `global.gd`, kept by the Rust root in 2.6) and is excluded from exports. `dcl-ios-devtools` (161) injects local-network Info.plist keys into Godot-exported debug iOS builds. `emote_converter` (267) is a tool-menu item converting GLB emotes to `.tres` libraries. `resource_tracker` (1,217) is an editor profiler. `dcl_dev_tools` (1,924) is the DCL Tools menu (Rust Log Filter and others) plus `resource_counter.gd`, loaded at runtime by `loading_screen.gd` and `benchmark_flow_controller.gd`. `sentry` is binaries. | Keep `dcl_build_plugin` and `dcl-godot-android` as export tooling, exempt from the `.gd` rule, ported to xtask when next touched. Drop `dcl_mobile_preview` (no GDScript UI left to preview), `resource_tracker`, `dcl-ios-devtools` (the RN app owns its Info.plist), `sentry` (D-16), and `dcl_dev_tools` once `resource_counter.gd` is ported for the benchmark (5.7). Keep `emote_converter` until the base emotes next change, then port it to xtask. | EM. 5.9 applies it. |
| D-open-14 | **Rollout and rollback for 2.0.0.** Staged percentages, the kill criteria, how long the 1.x line stays releasable, when `godot-asc-deploy` `main` switches to the RN workspace (the dispatch targets its `feat/react-native-app` today). | Play 10% → 50% → 100% over two weeks and an iOS phased release, each step gated by the daily crash KPI against the 1.x baseline; keep the 1.x pipeline releasable for one cycle; asc-deploy switches the day the branch merges. | EM. By mid-December. |

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
| Adding a call in phase 4 | Rust, rebuild, regenerate | Four files, two platforms, test on both; about an hour each across ~70 calls | a |
| Who can add a call | Any Rust or RN dev | Needs a native dev per call, serializing December | a |
| Runtime cost | Fewer copies, no JNI | Extra conversions | a, irrelevant by design: the hot path never crosses the FFI |
| First call working | Slower: C++ toolchain, RN 0.86 compat | Faster: known pieces | b |
| Maturity, community | 0.x, small maintainer base | Firefox-mobile-grade uniffi output; TurboModules are RN's own | b |
| Version coupling | Pinned uniffi; must track RN releases | Independent upgrades | b |
| Debugging a glue bug | Generated C++ and JSI | Kotlin/Swift breakpoints | b |
| Single library (D-6), events from tokio threads | Must be verified (checks 2 and 3) | Straightforward | b |
| Host-only calls | Small hand-written module | Same | tie |
| Exit cost | Fall back to (b), Rust unchanged | None | tie |
