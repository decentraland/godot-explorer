# DclRn — the mobile app

The shipping iOS/Android app. Every screen and modal is React Native; the Rust core (`dcl_core`) holds the
session, the networking and the app state machine; the Godot engine (`dcl_engine`) only runs scenes. It ships
under the existing store identity (`org.decentraland.godotexplorer`) as an update of the Godot-exported app it
replaces.

```
Godot view (bottom)  Android: GodotFragment, created on `engine_required`; iOS: GDTViewController as a child of DclHostViewController
RN root view (top)   every screen, the HUD, the modals
dcl-core module      typed FFI (uniffi-bindgen-react-native): calls in, one event callback out
```

The shell and the engine never talk to each other: both talk to the core. The core decides when the engine
starts (`engine_required`, then the engine attaches itself) and what it shows (the avatar preview, a loading
world, a world), and the shell renders from `core.state()` alone. Scene (SDK) UI, the joystick, the gamepad and
camera look stay in Godot. Orientation belongs to RN: portrait before a scene, landscape in it.

## Source layout

- `src/core/`: the FFI surface. `native.ts` re-exports the generated bindings, `api/` wraps them per domain,
  `mock/` implements the same interface for builds without an engine and for jest, `useCoreState.ts` and
  `events.ts` subscribe to the core's events.
- `src/app/`: the shell (`Shell.tsx`), navigation, deep-link intake, startup, session and analytics lifecycle.
- `src/flow/`: the entry flow (splash, terms, account, sign-in, avatar, name, tour) and the FTUE steps.
- `src/discover/`, `src/backpack/`, `src/profile/`, `src/menu/`, `src/settings/`, `src/hud/`, `src/world/`,
  `src/modals/`, `src/notifications/`: the screens, built from Figma (`docs/rn-ui-spec/`). Public GETs (places,
  events, catalog) live here with hosts from `core.urls()`; anything signed is a typed core call.
- `src/ds/`: the design system: tokens, fonts, `AppText`, `Pill`, `Tabs`, `ItemCard`, buttons, icons.
- `src/i18n/`: the only translation source: `en`, `es`, `pt_BR`, one file per area. The `engine` area is
  what Godot draws itself; `node scripts/i18n-engine.mjs` exports it to `godot/locale/*.csv` (checked in CI
  with `--check`). `__tests__/i18n-coverage.test.ts` fails on hardcoded UI text.
- `src/gallery/`: a component gallery (every screen and state with fixtures), also exercised by jest.
- `modules/dcl-core/`: the generated TurboModule. Regenerate it after any change to a Rust signature in
  `dcl_core` or `dcl_host`: `cd lib && cargo build -p dcl_bindings && cd ../rn && node scripts/ubrn-generate.mjs`
  (CI fails on a diff).
- `android/`, `ios/`: the native host, the only native code we write. It creates the engine view on
  `engine_required`, routes touches (RN when a real RN view is under the finger, Godot otherwise), and
  implements what JS must not or cannot do: attestation, the device id, video.

## What JS may ask the core to do

Keys never reach JS: it cannot read the ephemeral key, the wallet, the attestation token or a signed header, and
there is no generic "signed fetch". The calls that act on the account are typed and each follows a user action:

- Session: `sign_in_guest`, `sign_in_email_start|verify`, `sign_in_browser` + `complete_browser`,
  `sign_in_wallet_begin|complete`, `sign_out`, `delete_account`.
- Profile and places: `deploy_profile`, `set_favorite`, `vote`, `set_event_reminder`.
- Support and telemetry: `submit_bug_report`, `capture_screenshot`, `track`, `track_screen`,
  `set_analytics_consent`.

Calls with no user action (state, settings, URLs, version and device gates, notification polling, session
lifecycle ticks) read or report state. A new core call that signs, spends or deletes needs a user gesture in the
screen that makes it and a line in this list.

## Iterate on the UI without Godot

The engine is optional at build time. Without the synced Godot artifacts (`rn/ios/Godot`,
`rn/android/app/godot`) the app builds for simulators and emulators and `src/core/mock` stands in for the core;
`GodotPlaceholder` stands in for the 3D world and the avatar preview. The iOS simulator never links the engine.

```bash
cd rn && npm ci
npm start                      # Metro, fast refresh
npm run ios                    # simulator (cd ios && pod install first)
npm run android:no-godot       # emulator, even when app/godot is synced
# or, from the repo root, a bundled (Metro-less) build:
cargo run -- run --target ios --no-godot
cargo run -- run --target android --no-godot
```

The Expo Go workflow ended with the FFI: a custom native module cannot load in Expo Go.
React Native is pinned to the version of Expo SDK 57 (RN 0.86.3); upgrading RN means moving to the Expo SDK that
ships it. `expo` is excluded from native autolinking (`react-native.config.js`).

Checks (all run in CI): `npx tsc --noEmit`, `npx eslint App.tsx src __tests__`, `npx jest`,
`npm audit --omit=dev --audit-level=critical`. Dependencies are pinned to exact versions in `package.json`;
update them on purpose, not through a `^` range.

## Visual snapshots

Every screen of the gallery (`src/gallery/`, one id per Figma frame or state) is also a Storybook story: the
`Screens/<group>` stories in `src/gallery/stories/` are generated from the registry, so a screen registered once
shows up in both. `.github/workflows/rn_visual.yml` runs the no-Godot debug app on an x86_64 Android emulator with
Storybook enabled, opens each story by deep link and takes a screenshot with Maestro. A PR's screenshots are compared
by reg-cli with the **official** screenshots of its base branch and sent, with the reg-cli report, to the mobile board
(Cloudflare R2). The sticky PR comment links to the run there.

- **Review**: in the board's *Visual snapshots* tab, approve or reject the changed and new screens. The decision is
  mirrored on the PR as the commit status `RN visual snapshots` (pending while screens await review, success when
  approved or when nothing changed, failure when rejected, error when a screen was not captured). Add it as a required
  check in branch protection to gate merges on it.
- **Official screenshots**: the accepted set of `2.x`, `main` and `release*`, kept for good. A push to one of those
  branches publishes its capture as the new official set. PR runs and their images are deleted after 14 days.
  Until the first push to a base branch has published one, PRs compare with `rn/visual-baselines` in git.
- **Which screens**: CI snapshots only the main views listed in `src/gallery/snapshot-set.json` (36 of the gallery's 322
  frames: one per screen, no per-detail Figma copies). The rest stay in the gallery and Storybook for development.
  To snapshot another screen, add its id to that file.
- **A new screen**: register it in the gallery (`src/gallery/`) and run `npm run gallery:ids`.
- **A screen that is not stable** (animation, timer): give its registry entry a `settleMs`, or list its id in
  `tools/ui-spec/visual-ignore.json` (`{"<id>": "reason"}`).
- **Locally**, on an emulator or phone (`ANDROID_SERIAL` picks the device; needs [Maestro](https://maestro.mobile.dev)):
  install the debug build (`DCL_NO_GODOT=1`, then `cd android && ./gradlew :app:assembleDebug -PdclNoGodot=true`) and
  run `tools/ui-spec/ci_capture.sh --ids /tmp/shots <id>[,<id>]` (or `ci_capture.sh <source> /tmp/shots`). It starts
  Metro with `EXPO_PUBLIC_STORYBOOK_ENABLED=true` (without it Storybook is not in the bundle), pins the device (720x1600
  at 320 dpi, animations off, fixed status bar clock; `tools/ui-spec/device_normalize.sh`) and writes one PNG per id.
  `tools/ui-spec/rn_capture.sh` is the same capture through adb alone, with no Storybook or Maestro.
- **Compare locally**: put the expected screenshots in a folder (`tools/ui-spec/board_visual.py fetch-official --ref
  2.x --out /tmp/expected` with `BUILD_REPORTER_TOKEN` and `BOARD_EVENTS_URL` set), then `python3
  tools/ui-spec/visual_report.py prepare --new /tmp/shots --baseline /tmp/expected`, `cd rn && npx reg-cli
  ../visual-report/actual ../visual-report/expected ../visual-report/diff -R ../visual-report/index.html -J
  ../visual-report/reg.json -M 0.06 -T 0.001 -I`, then `visual_report.py summary`.

Storybook's own on-device browser needs `react-native-reanimated` and `react-native-gesture-handler`, which the app
does not ship; the stories render alone (`onDeviceUI: false`), and `GalleryList` is the in-app index. The runs are
debug builds, and debug builds never report to Sentry (`src/sentry.ts`), so captures leave no events behind.

## The full app (with Godot)

`cargo run -- export --target android|ios` runs Godot's export and then builds the RN app from it;
`cargo run -- run --target android|ios` does build + export + install + launch.

- **Android**: Godot's APK goes to `exports/godot/`; `scripts/sync-godot-android.sh` copies the engine AAR, the
  native libs (unstripped, so the AAB carries their symbols) and the project data into `android/app/godot`;
  Gradle (`app/godot.gradle`) builds the app to `exports/decentraland.godot.client.apk|aab`, the paths CI signs and
  uploads. `versionCode` comes from `DCL_BUILD_NUMBER`, `versionName` from `lib/Cargo.toml`. The
  `com.godot.game.GodotAppLauncher` activity alias keeps the deep-link filters and the xtask launch command of the
  Godot-exported app. `MainActivity.BOOT_ARGS` passes `--force-mobile --no-pulse` (and `--perf-log` in debug builds).
- **iOS**: needs the engine built with `embedded_host=yes` (no SwiftUI `@main`; the RN `AppDelegate` boots it with
  `godot_host_init`). `cargo run -- install --targets ios` fetches it as the `ios_host_release.a` template, or set
  `GODOT_HOST_LIB=<engine>/bin/libgodot.ios.template_release.arm64.a` (`BUILD_NAME=gh scons platform=ios
  target=template_release arch=arm64 production=yes embedded_host=yes`, with `editor/` removed as in CI).
  `scripts/sync-godot-ios.sh` fills `ios/Godot` and writes `Godot/godot.xcconfig`; the "Embed Godot" build phase
  (`scripts/embed-godot-ios.sh`) copies the pck and frameworks into the bundle. The export also stages
  `exports/rn` for the TestFlight job, which runs `npm ci` + `pod install` and archives `DclRn.xcworkspace`.
  The `AppDelegate` forwards deep links, universal links and APNs callbacks to the core and the notification
  libraries.
- `--godot-only` stops `export` after Godot's own export, at its usual paths.

## Crash reports

Metro runs through `@sentry/react-native/metro`, so every bundle carries a debug ID. CI uploads the Hermes source
maps and the native symbols for `main` and `release` builds (`android_builds.yml`); without that upload a JS crash
shows minified frames.

## Known limitations

- The debug Godot template runs at about 11–15 fps on phones; `cargo run -- run -r` uses the release one.
- Portrait scenes are not supported: the world screen forces landscape.
