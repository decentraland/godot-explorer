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

## Open

| # | Question | Recommendation | Owner, decide by |
|---|---|---|---|
| D-open-2 | **FFI binding generator.** The Rust side is uniffi either way (D-5); the question is the glue between Hermes and the uniffi C ABI. (a) `uniffi-bindgen-react-native`: generates a C++/JSI TurboModule and the TS types straight from the uniffi metadata; one boundary, no Kotlin or Swift per call, async calls and callback interfaces generated; a smaller community project, version-coupled to uniffi and to RN. (b) uniffi's Kotlin and Swift bindings plus a hand-written `DclCore` TurboModule: proven pieces, debuggable in Kotlin/Swift, but every call is written three times (TS spec, Kotlin, Swift) and events and async are hand-plumbed, a standing tax on the ~70 calls in `04-core-api.md` and on every phase-4 domain. | (a) if it passes the five checks in 3.1, else (b). Either way the host-only calls (attest, push token, store, surface order, engine view) stay in a small hand-written module. Low regret: switching later changes the glue, not the core. | Core owner. **End of 3.1**, mid-Nov. |
| D-open-3 | **Session and settings files.** `settings.cfg` (Godot `ConfigFile`, INI-like) holds `session_account`, `guest_profile` and every setting; existing users have it on disk. | The core writes its own files (`session.json`, `settings.json`) under the data dir and imports `settings.cfg` once at first boot with a small INI reader (keep the key names from `rn/src/settings/keys.ts`); the old file is left in place until the following release. Covered by a test with a real `settings.cfg` fixture. | Core owner. With 1.4. |
| D-open-4 | **Core data dir on the RN host.** Must equal the Godot-exported app's `user://` so the store update keeps sessions and caches (risks R-6). | Pass `OS.get_user_data_dir()`'s equivalent from the host: on Android the path Godot's `OS_Android` resolves (verify whether it is `getFilesDir()` or external), on iOS the same `Documents`/`Library` dir. Verify on a device by upgrading from a store build. | Host owner. With 3.4. |
| D-open-5 | **Editor addons in GDScript** (4,565 lines, editor-only, never shipped). | Out of the zero-`.gd` rule for the January release: the lint says "no `.gd` under `godot/src`". After the release: port `dcl_build_plugin` and `dcl_mobile_preview` to a Rust `EditorPlugin` if still used, drop the rest. | EM. By the scope gate. |
| D-open-6 | **GDScript tests** (`godot/src/test`, 5,711 lines). | Port by subject when the subject is ported; delete with the subject. No standalone test-porting effort. | With each phase-5 item. |
| D-open-8 | **RN libraries for OS intake:** IAP, local and push notifications, wallet connect, share and calendar. | Pick per domain in phase 4 with the usual criteria (maintained, new-architecture support, no Expo-only dependency). Consequence to accept now: once the app has a custom TurboModule, Expo Go cannot run it; the UI-without-engine workflow continues through the no-engine native builds (`--no-godot`, `npm run android:no-godot`) with the mock core. | Shell owner. With 3.6 (wallet) and 4.6/4.8. |
| D-open-9 | **Math type in the core** (comms, movement, pulse decoder). | `glam` (already in the dependency tree through gdext); `From` impls at the engine edge. | Core owner. With 1.3. |
| D-open-10 | **Engine host template.** The iOS `embedded_host=yes` library is built locally today; CI's process-templates job deletes the raw `.a`. | Publish `ios_host_release.a` from the engine release (merge the engine branch `spike/ios-host-embedding` into `decentraland/godotengine` main) and bump `GODOT_BUILD_SHA`; until then `GODOT_HOST_LIB`. | Engine owner. Before the December team ramp-up. |
| D-open-11 | **Emote wheel: RN or engine?** `rn_ui_filter.gd` leaves `EmoteWheel` visible in Godot today. | RN (it is HUD); the engine plays the emote through `play_emote`. | Shell owner. With 4.10. |
