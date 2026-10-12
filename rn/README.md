# DclRn — VIBE-CODED SPIKE

> ⚠️ Exploratory spike. Not reviewed for merge, not production quality.

A React Native app that hosts the Decentraland Godot client as an embedded 3D view and draws the
UI in React Native. The app boots an **ephemeral guest straight into Genesis Plaza** with the Godot UI
hidden (`--rn-host`). Scene (SDK) UI, the joystick, the on-screen gamepad and camera look stay native
in Godot; everything else is React.

```
Godot view (bottom)  <- Android: GodotFragment, iOS: GDTViewController as child VC
RN root view (top)   <- HUD: status chip, Wave / Plaza / Chat buttons, loading cover
GodotBridge          <- JSON commands/events <-> RnHostPlugin singleton <-> godot/src/rn_bridge/rn_bridge.gd
```

Commands (RN → Godot): `teleport{x,y}`, `send_chat{text}`, `emote{urn}`, `backpack_open|backpack_place{x,y,w,h}`, `backpack_close`, `equip{urn}`.
Events (Godot → RN): `ready`, `loading{state}`, `state{parcel,realm,fps,position}` (~4 Hz), `chat{address,message,timestamp}`.

## Backpack: a Godot 3D view inside an RN screen

The Backpack button opens a full-screen RN screen (`src/Backpack.tsx`) with a transparent hole on the
right. RN reports the hole's rectangle in device pixels; `godot/src/rn_bridge/rn_backpack.gd` places
the real `avatar_preview.tscn` there (converted to Godot's scaled viewport units), pauses the plaza's
3D render and hides the Godot UI. Touches inside the hole fall through to Godot, so the preview's own
drag-to-rotate works. Tapping a wearable sends `equip{urn}` and the avatar updates live.
The Godot surface is translucent (`override.cfg` in the host's assets sets
`display/window/per_pixel_transparency/allowed`) and `RnHostPlugin.set_surface_on_top` flips it above
the RN view while the backpack is open, so the avatar draws in front of RN content (the demo banner);
closing flips it back below. Touches still route by RN hit-test, so a drag over the avatar reaches Godot.
Spike scope: guest profile and a short list of base-avatar wearables, no ownership or saving.

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
