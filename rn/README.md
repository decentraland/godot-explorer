# DclRn — VIBE-CODED SPIKE

> ⚠️ Exploratory spike. Not reviewed for merge, not production quality.

A React Native app that draws all UI in React Native. The Godot engine is started lazily
(`GodotBridge.startEngine()`) in **avatar-only mode** (`--rn-host --rn-avatar-only`): it loads no
world, creates a disposable account with a random profile, and only renders one avatar preview that
RN screens leave a transparent hole for. The engine cannot be restarted in-process, so it starts once
and lives until the app dies.

```
Godot view (bottom)  <- Android: GodotFragment (created on startEngine), iOS: GDTViewController
RN root view (top)   <- all screens
GodotBridge          <- JSON commands/events <-> RnHostPlugin singleton <-> godot/src/rn_bridge/rn_bridge.gd
```

Commands (RN -> Godot), see `src/GodotBridge.ts`: `backpack_open|backpack_place{x,y,w,h}` (device px),
`backpack_close`, `equip{urn}`, `unequip{urn}`, `set_body_shape{urn}`, `set_color{target,color}`,
`play_emote{urn}`, `set_emote_slot{slot,urn}`, `randomize_avatar`, `set_name{name}`.
Events (Godot -> RN), see `src/events.ts`: `ready` (host scene is up) and `avatar{body_shape,wearables,
emotes,skin,hair,eyes}` after `backpack_open` and after every command that changes the avatar.
The world commands (`teleport`, `send_chat`, `emote`) and `loading`/`state`/`chat` events only work
when the engine is started without `--rn-avatar-only` (old Genesis Plaza host mode).
Orientation is owned by RN: the manifest is portrait and `GodotBridge.setOrientation('portrait'|'sensor')`
switches it (the host's `override.cfg` makes the engine start portrait too).

## UI structure (built from Figma)

- `src/ds/`: design-system module from the "Design System Mobile" file: tokens (colors, rarities, Inter
  fonts), `AppText`, `Pill`, `Tabs`, `ItemCard`, `SlotTile`, `BottomNav`, color sliders, buttons and the
  Figma icons. Android fonts live in `android/app/src/rnassets/` (outside `src/main/assets`, which the
  sync script recreates).
- `src/backpack/`: the "HI-FI" Backpack, portrait (draggable bottom sheet) and landscape (split panel).
  Wearables come from the catalyst base-avatars collection, emotes from the base-emotes entities; a failed
  load shows the "Connection lost" state.
- `src/flow/`: portrait-only entry flow (splash, terms, account, sign-in, avatar create/customize/naming,
  tour) mirroring `lobby.gd`. Sign-in is a stand-in (`flow/auth.ts`): the real auth is in the Rust client.
- `src/discover/`: basic Discover (Featured, Events, Last Played, Most Active) and the Detail Place sheet, from the
  public places/events APIs. Search filters the loaded lists; Jump In only records the place in Last Played
  (entering the world isn't part of the avatar-only engine yet).
- `src/engine.ts`: lazy engine start, command queue until `ready`, latest avatar state.
- `src/WorldHud.tsx`: the old Genesis Plaza HUD; not mounted while the engine is avatar-only.

## Backpack: a Godot 3D view inside an RN screen

`godot/src/rn_bridge/rn_backpack.gd` owns the single `avatar_preview.tscn`: created on the first
`backpack_open`, then only shown, hidden and moved (`backpack_place`; px are converted with the current
window size, so it also works in landscape). `backpack_close` hides it and stops its rendering. Touches
inside the hole fall through to Godot, so the preview's own drag-to-rotate works.
The Godot surface is translucent (`override.cfg` in the host's assets sets
`display/window/per_pixel_transparency/allowed`) and `RnHostPlugin.set_surface_on_top` flips it above
the RN view while the preview is shown, so the avatar draws in front of RN content; closing flips it back.
Touches still route by RN hit-test. `unequip` of eyes/mouth/eyebrows is ignored (the engine forces defaults).

## Android (verified on a device)

```bash
cargo run -- build -r --target android
cargo run -- export --target android --format apk --release   # project data + stripped libs
rn/scripts/sync-godot-android.sh                    # engine AAR, libdclgodot.so, assets -> rn/android/app
#   (it refreshes asset mtimes: Gradle otherwise keeps a stale assets.sparsepck -> truncated scripts)
cd rn && npm install
cd android && ./gradlew :app:assembleRelease        # JS bundled in; no Metro needed
adb install -r app/build/outputs/apk/release/app-release.apk
```

## iOS (code only — never compiled by the author)

Needs the engine fork branch `spike/ios-host-embedding` (`decentraland/godotengine`), built with
`scons platform=ios target=template_release arch=arm64 embedded_host=yes`, plus a macOS machine:

```bash
cargo run -- build -r --target ios && cargo run -- export --target ios --release
GODOT_HOST_LIB=<engine>/bin/libgodot.ios.template_release.arm64.a rn/scripts/sync-godot-ios.sh
cd rn/ios && ruby configure_godot.rb && pod install
```

## Known limitations

- The first debug-template build ran at about 11–15 fps on the test phone; use the release template (`GODOT_BUILD=debug` selects the debug one).
- Android plugin features are not carried over: push, IAP, Firebase, Reown, ExoPlayer video.
- Sentry, deeplinks and the Android back button are not forwarded to Godot.
- iOS: engine init order, GDExtension dylib loading from the host bundle and touch pass-through are unverified.
