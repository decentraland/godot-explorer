# RN host vs Godot-exported app: fps and memory (plan 2.9, risks R-1)

Same commit, same scene, release build, one phone at a time. Both apps share the package id
`org.decentraland.godotexplorer`, so installing one replaces the other (uninstall first when the
signature differs: `adb uninstall org.decentraland.godotexplorer`, which wipes data).

## What is measured

| Metric | How | Same on both hosts |
|---|---|---|
| fps, RSS | the engine logs `[Perf] fps=<n> rss_mb=<n>` every 5 s while a world is loaded, when started with `--perf-log` | yes, one code path (`DclEngine`, `memory_monitor::used_memory_mb`) |
| startup | Android: `adb shell am start -W -n <launcher>` (`TotalTime`); iOS: time from `devicectl process launch` to the first `[Startup] root._ready` line | roughly |
| sustained | the median of at least 12 `[Perf]` lines (60 s) after the scene settles, standing still at Genesis Plaza spawn | yes |

## Getting `--perf-log` to the engine

- Godot-exported app: `cargo run -- run --target android -- --perf-log` (the extras reach the launcher), or the
  deeplink path in `launch_devices.sh --param perf-log=true` is not wired: use the CLI form.
- Android RN host: add `"--perf-log"` to `BOOT_ARGS` in `rn/android/app/src/godot/java/com/dclrn/MainActivity.kt`
  for the measurement build (do not commit it).
- iOS RN host: put `--perf-log` on its own line in the export's `godot_cmdline.txt`, or pass it as a launch argument.

## Android

```bash
adb logcat -c
adb shell cmd package resolve-activity --brief org.decentraland.godotexplorer | tail -1   # launcher component
adb shell am start -W -n <component>                                                     # startup time
adb logcat -s godot | grep --line-buffered '\[Perf\]' | tee /tmp/perf-<host>.log
adb shell dumpsys meminfo org.decentraland.godotexplorer | grep -E 'TOTAL|Graphics'      # PSS cross-check
```

RN host: tap through to the avatar step and "Play as guest" (the engine only boots there), then enter Genesis Plaza.
Godot app: `scripts/bench/launch_devices.sh --android --realm https://realm-provider.decentraland.org/main --position 0,0`.
Check `com.dclrn` is the top resumed activity before any `adb input`; the phone is a daily device.

## iOS

```bash
xcrun devicectl device process launch --device <udid> --console org.decentraland.godotexplorer -- --perf-log \
  | tee /tmp/perf-<host>.log
```

## Record

Fill the table in `plan/risks.md` R-1 (one row per host and phone): median fps, median `rss_mb`, startup ms, build
commit, template (debug/release). The budget is no worse fps than the Godot-exported app and at most +100 MB RSS.

| Phone | Host | Commit | fps (median) | rss_mb (median) | startup ms |
|---|---|---|---|---|---|
| Moto Edge 60 Pro | Godot app | | | | |
| Moto Edge 60 Pro | RN host | | | | |
| iPhone | Godot app | | | | |
| iPhone | RN host | | | | |
