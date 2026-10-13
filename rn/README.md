# DclRn — the mobile app

> ⚠️ Side project: being migrated into the shipping iOS/Android app.

A React Native app that draws all UI in React Native. It ships under the existing store identity
(`org.decentraland.godotexplorer`) as an update of the Godot-exported app it replaces. The Godot engine is
started lazily (`GodotBridge.startEngine()`) in **avatar-only mode** (`--rn-host --rn-avatar-only`): it
loads no world, creates a disposable account with a random profile, and only renders one avatar preview
that RN screens leave a transparent hole for. `enter_world` runs the explorer in the same session (the
engine cannot be restarted in-process, so it starts once and lives until the app dies). Scene (SDK) UI,
the joystick, the on-screen gamepad and camera look stay native in Godot; everything else is React.

```
Godot view (bottom)  <- Android: GodotFragment (created on startEngine), iOS: GDTViewController as child VC of DclHostViewController
RN root view (top)   <- all screens
GodotBridge          <- JSON commands/events <-> RnHostPlugin singleton <-> godot/src/rn_bridge/rn_bridge.gd
```

Commands (RN -> Godot), see `src/GodotBridge.ts`: `backpack_open|backpack_place{x,y,w,h}` (device px),
`backpack_close`, `equip{urn}`, `unequip{urn}`, `set_body_shape{urn}`, `set_color{target,color}`,
`play_emote{urn}`, `set_emote_slot{slot,urn}`, `randomize_avatar`, `set_name{name}`.
`enter_world{x,y,realm}` (realm is a full realm URL: `https://realm-provider-ea.decentraland.org/main` for Genesis City, `https://worlds-content-server.decentraland.org/world/<name>` for a World; x,y is the spawn parcel) and `exit_world`.
Events (Godot -> RN), see `src/events.ts`: `ready` (host scene is up) and `avatar{body_shape,wearables,
emotes,skin,hair,eyes}` after `backpack_open` and after every command that changes the avatar.
`enter_world` switches the same engine session from the avatar host to the explorer (same account and avatar) and emits `world{state:'entered'}`; `loading`/`state`/`chat` events and `teleport`, `send_chat`, `emote` work while in the world (`loading finished` = playable). `exit_world` tears the world down, returns to the avatar host and emits `world{state:'exited'}` then `ready`; a later `enter_world` works again.
In the world Godot never changes the device orientation (RN owns it); the layout follows the window shape.
Orientation is owned by RN: everything before a scene (entry flow, Discover, Backpack) is portrait-locked
(the manifest is portrait and the host's `override.cfg` starts the engine portrait), and the world screen
calls `GodotBridge.setOrientation('landscape')` (forced, either direction) until Leave brings it back to
`'portrait'`. Portrait scenes are not supported yet.

## UI structure (built from Figma)

- `src/ds/`: design-system module from the "Design System Mobile" file: tokens (colors, rarities, Inter
  fonts), `AppText`, `Pill`, `Tabs`, `ItemCard`, `SlotTile`, `BottomNav`, color sliders, buttons and the
  Figma icons. Android fonts live in `android/app/src/rnassets/`.
- `src/backpack/`: the "HI-FI" Backpack, portrait only (draggable bottom sheet).
  Wearables come from the catalyst base-avatars collection, emotes from the base-emotes entities; a failed
  load shows the "Connection lost" state.
- `src/flow/`: portrait-only entry flow (splash, terms, account, sign-in, avatar create/customize/naming,
  tour) mirroring `lobby.gd`. Sign-in is a stand-in (`flow/auth.ts`): the real auth is in the Rust client.
- `src/discover/`: basic Discover (Featured, Events, Last Played, Most Active) and the Detail Place sheet, from the
  public places/events APIs. Search filters the loaded lists; Jump In records the place in Last Played.
- `src/engine.ts`: lazy engine start, command queue until `ready`, latest avatar state.
- `src/world/WorldScreen.tsx`: loading cover, HUD and Leave while in a scene (landscape).

## Backpack: a Godot 3D view inside an RN screen

`godot/src/rn_bridge/rn_backpack.gd` owns the single `avatar_preview.tscn`: created on the first
`backpack_open`, then only shown, hidden and moved (`backpack_place`; px are converted with the current
window size, so it also works in landscape). `backpack_close` hides it and stops its rendering. Touches
inside the hole fall through to Godot, so the preview's own drag-to-rotate works.
The Godot surface is translucent (`override.cfg` in the host's assets sets
`display/window/per_pixel_transparency/allowed`) and `RnHostPlugin.set_surface_on_top` flips it above
the RN view while the preview is shown, so the avatar draws in front of RN content; closing flips it back.
Touches still route by RN hit-test. `unequip` of eyes/mouth/eyebrows is ignored (the engine forces defaults).

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
it, to Godot otherwise. Both hosts start the engine with `--rn-host --rn-avatar-only` (Idle: no lobby, no
world); the world is entered with the `enter_world` command. Android also passes `--force-mobile --no-pulse`
(`MainActivity.BOOT_ARGS`); iOS does not. `--force-mobile` marks the client as a *virtual* mobile, so on a desktop it
resizes the window to 1280×720 on the landscape switch and the virtual-keyboard height poll in `global.gd` is
skipped — check both on a device before aligning the two lists (plan 2.8). The iOS `AppDelegate` forwards deep
links, universal links and APNs callbacks to the DclGodotiOS services (they listen on `GDTApplicationDelegate`,
which is not the app delegate here); on Android the `com.godot.game.GodotAppLauncher` alias keeps the deep-link
filters, home-screen shortcuts and the xtask launch command of the Godot-exported app.

## Request/response bridge

Besides fire-and-forget commands, `rn/src/bridge/request.ts` gives a promise API over the same channel:
`request('settings.set', {key, value, commit}, {timeoutMs})` sends `{type:'req', id, method, params}`;
Godot (`godot/src/rn_bridge/rn_requests.gd`, routed before the avatar/explorer split so it works in both modes)
answers with a `res` event `{id, ok, result | error}`. Errors, rejected promises and timeouts (default 15 s)
reject with `BridgeError`. Method names and payloads are typed in `rn/src/bridge/methods.ts`:
`settings.{get_all,set,cache_size,clear_cache,version}`, `account.{sign_out,delete}`,
`friends.{list,requests,nearby,accept,reject,cancel,send,unblock,block}`, `notifications.{list,read}`,
`profile.{get,names,deploy}`, `fetch.signed` (https, DCL service hosts only), `places.{favorite,vote}`,
`clipboard.copy`, `screenshot.capture`. `settings.set` only accepts whitelisted keys, applies them live and
persists with `commit: true`.

Without the engine, `registerMockHandler(method, fn)` (`rn/src/bridge/mockHandlers.ts`) fakes a method;
unregistered methods reject with "no mock handler".

New commands (in-world only unless noted): `set_hud_insets {top,right,bottom,left}` and `set_chat_width {width}`
(device px), `chat_open {open}`, `hide_ui {hidden}`, `focus_camera {target}` (avatar preview; also in the
avatar host). New events: `res`, `loading_progress {loaded,total,mbps,percent}`, `hud_state {hidden,chat_open}`,
`unread_chat {count}`, `settings_changed {key,value}`, and `chat` now carries an optional resolved `profile`.

## Known limitations

- The debug Godot template runs at about 11–15 fps on phones; `cargo run -- run -r` uses the release one.
- The Android back button is not forwarded to Godot.
- iOS: the backpack overlay is not wired yet (no `set_surface_on_top` in the iOS bridge, opaque Metal layer).
- Push, deep links, Sentry and IAP use today's Godot plugins inside the host; verified on the Android emulator
  (plugin loads, deep link teleports), not yet on iOS devices or with real push/TestFlight signing.
