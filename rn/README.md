# DclRn — the mobile app

> ⚠️ Started as a vibe-coded spike; being migrated into the shipping iOS/Android app.

A React Native app that hosts the Decentraland Godot client as an embedded 3D view and draws the
UI in React Native. It ships under the existing store identity (`org.decentraland.godotexplorer`) as an
update of the Godot-exported app it replaces. The app boots an **ephemeral guest straight into Genesis Plaza** with the Godot UI
hidden (`--rn-host`). Scene (SDK) UI, the joystick, the on-screen gamepad and camera look stay native
in Godot; everything else is React.

```
Godot view (bottom)  <- Android: GodotFragment, iOS: GDTViewController as child VC of DclHostViewController
RN root view (top)   <- HUD: status chip, Wave / Plaza / Chat buttons, loading cover
GodotBridge          <- JSON commands/events <-> RnHostPlugin singleton <-> godot/src/rn_bridge/rn_bridge.gd
```

Commands (RN → Godot): `teleport{x,y}`, `send_chat{text}`, `emote{urn}`, `backpack_open|backpack_place{x,y,w,h}`, `backpack_close`, `equip{urn}`.
Events (Godot → RN): `ready`, `loading{state}`, `state{parcel,realm,fps,position}` (~4 Hz), `chat{address,message,timestamp}`.

## Backpack: a Godot 3D view inside an RN screen

The Backpack button opens a full-screen RN screen (`src/Backpack.tsx`) with a transparent hole on the
right. RN reports the hole's rectangle in device pixels; `godot/src/rn_bridge/rn_backpack.gd` places
the real `avatar_preview.tscn` there (converted to Godot's scaled viewport units), stops the main
viewport's 3D render, culls its 2D layers (nameplates included, via `canvas_cull_mask`) and hides the Godot UI. Touches inside the hole fall through to Godot, so the preview's own
drag-to-rotate works. Tapping a wearable sends `equip{urn}` and the avatar updates live.
The Godot surface is translucent (`override.cfg` in the host's assets sets
`display/window/per_pixel_transparency/allowed`) and `RnHostPlugin.set_surface_on_top` flips it above
the RN view while the backpack is open, so the avatar draws in front of RN content (the demo banner);
closing flips it back below. Touches still route by RN hit-test, so a drag over the avatar reaches Godot.
Spike scope: guest profile and a short list of base-avatar wearables, no ownership or saving.

## Iterate on the UI without Godot

The engine is optional at build time. Without the synced Godot artifacts (`rn/ios/Godot`,
`rn/android/app/godot`) the app builds for simulators and emulators and `src/godot/mockBridge.ts`
replays a session (ready → loading → state at 4 Hz, chat echo); `GodotPlaceholder` stands in for the
3D world and the avatar preview. The iOS simulator never links the engine.

```bash
cd rn && npm ci
npm start                      # Metro, fast refresh
npm run ios                    # simulator (cd ios && pod install first)
npm run android:no-godot       # emulator, even when app/godot is synced
npm run expo                   # Expo Go on a phone or simulator: no native build at all
# or, from the repo root, a bundled (Metro-less) build:
cargo run -- run --target ios --no-godot
cargo run -- run --target android --no-godot
```

React Native is pinned to the version Expo Go ships (SDK 57 → RN 0.86.3) so the JS runs there; `expo`
is excluded from native autolinking (`react-native.config.js`), so the native apps do not depend on
Expo modules. Upgrading RN means moving to the Expo SDK that ships it.

## The full app (with Godot)

`cargo run -- export --target android|ios` runs Godot's export and then builds the RN app from it;
`cargo run -- run --target android|ios` does build + export + install + launch as before.

- **Android**: Godot's APK goes to `exports/godot/`; `scripts/sync-godot-android.sh` copies the engine
  AAR, plugin AARs (`dcl-godot-android`, Sentry), native libs (unstripped, so the AAB carries their
  symbols) and project data into `android/app/godot`; Gradle (`app/godot.gradle`) builds the app to
  `exports/decentraland.godot.client.apk|aab` — the paths CI already signs and uploads. The AAB is left
  unsigned for the upload key. `versionCode` comes from `DCL_BUILD_NUMBER`, `versionName` from
  `lib/Cargo.toml`.
- **iOS**: needs the engine built with `embedded_host=yes` (no SwiftUI `@main`; the RN `AppDelegate`
  boots it with `godot_host_init`). `cargo run -- install --targets ios` fetches it as the
  `ios_host_release.a` template once the engine release publishes it; until then set
  `GODOT_HOST_LIB=<engine>/bin/libgodot.ios.template_release.arm64.a` (`BUILD_NAME=gh scons platform=ios
  target=template_release arch=arm64 production=yes embedded_host=yes`, with `editor/` removed as in CI).
  `scripts/sync-godot-ios.sh` fills `ios/Godot` and writes `Godot/godot.xcconfig` (link flags, device SDK
  only), which `DclRn/Project.xcconfig` includes optionally; the "Embed Godot" build phase
  (`scripts/embed-godot-ios.sh`) copies the pck and frameworks into the bundle. The export also stages
  `exports/rn` (no `node_modules`/Pods) for the TestFlight job, which runs `npm ci` + `pod install` and
  archives `DclRn.xcworkspace`.
- `--godot-only` stops `export` after Godot's own export, at its usual paths.

`DclHostViewController` stacks the RN view over Godot's and routes each touch: to RN when a real RN view is under
it, to Godot otherwise. Neither host passes `--force-mobile`: it marks the client as a *virtual* mobile, and the
landscape switch then resizes the window to 1280×720 like the desktop preview. The iOS `AppDelegate` forwards deep
links, universal links and APNs callbacks to the DclGodotiOS services (they listen on `GDTApplicationDelegate`,
which is not the app delegate here); on Android the `com.godot.game.GodotAppLauncher` alias keeps the deep-link
filters, home-screen shortcuts and the xtask launch command of the Godot-exported app.

## Known limitations

- The debug Godot template runs at about 11–15 fps on phones; `cargo run -- run -r` uses the release one.
- The Android back button is not forwarded to Godot.
- iOS: the backpack overlay is not wired yet (no `set_surface_on_top` in the iOS bridge, opaque Metal layer).
- Push, deep links, Sentry and IAP use today's Godot plugins inside the host; verified on the Android emulator
  (plugin loads, deep link teleports), not yet on iOS devices or with real push/TestFlight signing.
