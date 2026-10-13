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

## Open

| # | Question | Recommendation | Owner, decide by |
|---|---|---|---|
| D-open-3 | **Session and settings files.** `settings.cfg` (Godot `ConfigFile`, INI-like) holds `session_account`, `guest_profile` and every setting; existing users have it on disk. | The core writes its own files (`session.json`, `settings.json`) under the data dir and imports `settings.cfg` once at first boot with a small INI reader (keep the key names from `rn/src/settings/keys.ts`); the old file is left in place until the following release. Covered by a test with a real `settings.cfg` fixture. | Core owner. With 1.4. |
| D-open-4 | **Core data dir on the RN host.** Must equal the Godot-exported app's `user://` so the store update keeps sessions and caches (risks R-6). | Pass `OS.get_user_data_dir()`'s equivalent from the host: on Android the path Godot's `OS_Android` resolves (verify whether it is `getFilesDir()` or external), on iOS the same `Documents`/`Library` dir. Verify on a device by upgrading from a store build. | Host owner. With 3.4. |
| D-open-5 | **Editor addons in GDScript** (4,565 lines, editor-only, never shipped). | Out of the zero-`.gd` rule for the January release: the lint says "no `.gd` under `godot/src`". After the release: port `dcl_build_plugin` and `dcl_mobile_preview` to a Rust `EditorPlugin` if still used, drop the rest. | EM. By the scope gate. |
| D-open-6 | **GDScript tests** (`godot/src/test`, 5,711 lines). | Port by subject when the subject is ported; delete with the subject. No standalone test-porting effort. | With each phase-5 item. |
| D-open-8 | **RN libraries for OS intake:** IAP, local and push notifications, wallet connect, share and calendar. | Pick per domain in phase 4 with the usual criteria (maintained, new-architecture support, no Expo-only dependency). Consequence to accept now: once the app has a custom TurboModule, Expo Go cannot run it; the UI-without-engine workflow continues through the no-engine native builds (`--no-godot`, `npm run android:no-godot`) with the mock core. | Shell owner. With 3.6 (wallet) and 4.6/4.8. |
| D-open-9 | **Math type in the core** (comms, movement, pulse decoder). | `glam` (already in the dependency tree through gdext); `From` impls at the engine edge. | Core owner. With 1.3. |
| D-open-10 | **Engine host template.** The iOS `embedded_host=yes` library is built locally today; CI's process-templates job deletes the raw `.a`. | Publish `ios_host_release.a` from the engine release (merge the engine branch `spike/ios-host-embedding` into `decentraland/godotengine` main) and bump `GODOT_BUILD_SHA`; until then `GODOT_HOST_LIB`. | Engine owner. Before the December team ramp-up. |
| D-open-11 | **Emote wheel: RN or engine?** `rn_ui_filter.gd` leaves `EmoteWheel` visible in Godot today. | RN (it is HUD); the engine plays the emote through `play_emote`. | Shell owner. With 4.10. |
| D-open-12 | **Sign-in methods at the January release.** Today on mobile: guest, email OTP, Google and Apple through the browser hand-off to the auth server, MetaMask native on Android (Reown in `dcl-godot-android`), WalletConnect native on iOS (`DclWalletConnect` GDExtension); Discord and X hidden. The RN entry flow has none of the wallet paths. | Guest, email OTP and the browser flow in phase 3: all three are already Rust (`thirdweb_guest.rs`, `decentraland_auth_server.rs`, `remote_wallet.rs`). Wallet sign-in through an RN WalletConnect library in phase 4 if a maintained one works on the new architecture, otherwise after January. Pull the share of `AUTH_SUCCESS` by `login_type` from Segment before deciding whether a January gap is acceptable. | Product + EM. By the scope gate. |
| D-open-13 | **Localization.** Godot ships en, es and pt_BR from `godot/locale/*.csv`; RN has `i18n/en` only, with keys that mirror `en.csv`. | Ship the same three languages: generate `i18n/es` and `i18n/pt_BR` from the CSVs with a script (the keys already match), keep a CI check for missing keys as `static_checks.yml` does for the CSVs today. Never fewer languages than the app has now. | Shell owner. Before the first phase-4 domain closes. |
| D-open-14 | **Release mechanics.** Version number (`lib/Cargo.toml` drives `versionName`, the iOS short version and `bump_version.yml`): 1.16 or 2.0. Staged rollout and the rollback path: the Godot-exported 1.15.x line stays releasable from `main`/`release` or not. When `godot-asc-deploy` `main` switches to the RN workspace (the dispatch targets its `feat/react-native-app` today). | 2.0.0. Play staged rollout 10% → 50% → 100% over two weeks and an iOS phased release, each step gated by the daily crash KPI against the 1.15 baseline. Keep the 1.15 pipeline releasable for one cycle. asc-deploy merges the same day as the explorer branch. | EM. By mid-December. |
| D-open-15 | **Desktop artifacts after January.** `linux_builds.yml`, `windows_builds.yml` and `macos_builds.yml` publish desktop clients today; after phase 4 the desktop export is the emulator, with no sign-in and no product UI (D-4). | Publish the emulator under its own name for creators and QA; stop presenting desktop as a client; the last Godot-UI desktop build stays downloadable; product writes the message to desktop users. | Product. By the scope gate. |
| D-open-16 | **Avatar preview on low-end Android: live engine or snapshot fallback.** Idle mode renders the avatar live in a transparent surface (D-11). Below some device tier that may be too expensive; the catalyst's profile snapshots are the alternative. | Live by default. Decide a snapshot fallback for devices under the lowest dynamic-graphics preset with the numbers from 2.9 (R-1), not before. | Engine owner + product. After 2.9, by the scope gate. |
| D-open-17 | **Owners per layer and per domain.** This plan says "core owner", "shell owner", "engine owner" without names; December needs a named owner per layer, per phase-4 domain and for the device matrix. | Name them at the scope gate; add an owner column to the work items in `phase-4-screens.md` and `phase-5-engine-port.md`. | EM. Scope gate. |
| D-open-18 | **Phase 1 into `main` before December?** Phase 1 has no user-visible change and merging it early would cut the three-month divergence (R-9). D-13 says the branch ships nothing before December. | No. The workspace split and the runtime move touch most Rust files and would ride into an LMT3 release with no team attention on it. Keep the weekly `main` → branch merges. Reopen only if a weekly merge takes more than a day. | EM. Now. |

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
