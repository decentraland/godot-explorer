# 2.x vs 1.x — run protocol (#3206)

Copy-paste protocol for the benchmark in [`2x-vs-1x-3206-report.md`](./2x-vs-1x-3206-report.md).
Every number in that report came from these commands on these builds; anything below that was not
executed for lack of time is marked **pending** with the command that would produce it.

## 1. Builds under test (pinned)

| build | version | artifact | commit |
|---|---|---|---|
| 1.x | `1.16.1` (versionCode 2688) | `https://mobile-artifacts.dclregenesislabs.xyz/android/release/latest/decentraland.godot.client.apk` | `cfc24b4e5a1f0bc9f2d30f01f88bfa21a7aae279` (`release`, build number 2688) |
| 2.x | `2.0.0` (versionCode 2723) | `https://mobile-artifacts.dclregenesislabs.xyz/android/main/e3b7aa102bd0fbfe3a51cc2204248a59662b3fde/decentraland.godot.client.apk` | `e3b7aa102bd0fbfe3a51cc2204248a59662b3fde` (`main`, = the merge of PR #3099) |

Both are CI-signed release APKs (`export --release`): `aapt2 dump badging` shows no
`application-debuggable`, and both carry the same `CN=Decentraland, OU=Explorer` signer
(SHA-256 `8e75d23a86d4b511c7cf2220b9830574178d28517fc9a730b6a7f4eaca4c4784`). Only release builds
are installed for any measurement: no debug/profiler build takes part.

```bash
mkdir -p .bench-dl
curl -o .bench-dl/apk-1x-release-latest.apk \
  https://mobile-artifacts.dclregenesislabs.xyz/android/release/latest/decentraland.godot.client.apk
curl -o .bench-dl/apk-2x-e3b7aa102.apk \
  https://mobile-artifacts.dclregenesislabs.xyz/android/main/e3b7aa102bd0fbfe3a51cc2204248a59662b3fde/decentraland.godot.client.apk
AAPT=$HOME/Library/Android/sdk/build-tools/36.0.0/aapt2
$AAPT dump badging .bench-dl/apk-1x-release-latest.apk | head -1   # versionName='1.16.1'
$AAPT dump badging .bench-dl/apk-2x-e3b7aa102.apk        | head -1   # versionName='2.0.0'
```

**Content pin.** Both builds must report the same `scene_id` in `[RenderReady]` and the same spawn
`position`; the report quotes both. A run where `scene_id` differs from the recorded value is void
(the Genesis Plaza deployment changed under the benchmark).

## 2. Devices

| device | id | state | app version seen |
|---|---|---|---|
| Samsung Galaxy A54 5G (`SM-A546E`, Android 16, arm64-v8a, 120 Hz) | `R5CW60624NE` | usb | see report |
| iPhone 16e (`iPhone17,5`, iOS 26) | `1DD7B8A8-BECD-512A-93F5-11A7ABBD2CBC` | paired | see report |

```bash
adb devices -l
xcrun devicectl list devices
adb -s R5CW60624NE shell 'getprop ro.product.model; getprop ro.build.version.release'
```

Rules that apply to every run:

- **Release only**, installed from the pinned artifacts above.
- The same physical device runs 1.x and 2.x: both builds share the package id
  `org.decentraland.godotexplorer`, so installing one replaces the other and data is wiped when
  Android refuses a version-code downgrade.
- Phone unplugged from the charger, no other app in the foreground, screen brightness untouched,
  airplane mode off, Wi-Fi on the same network.
- Do not touch the phone during a window, do not run other adb work while the window is sampling.

## 3. Installing and the entry flow

```bash
# fresh install (needed when going 2.x -> 1.x: Android refuses the version-code downgrade)
adb -s $SERIAL uninstall org.decentraland.godotexplorer
adb -s $SERIAL install .bench-dl/apk-1x-release-latest.apk     # or the 2.x apk

# first launch only: the deep link waits behind the entry flow
adb -s $SERIAL shell am start -a android.intent.action.VIEW \
  -d 'decentraland://open?position=0,0' org.decentraland.godotexplorer
```

Entry flow, per build (once per install; every later launch enters straight from the link):

- **1.x** (Godot-drawn UI, no accessibility nodes) — portrait taps at 1080×2340:
  `PLAY AS GUEST` (540,1592) → `Don't allow` on the notifications prompt (541,1985) →
  `LET'S GO` (541,1897). Scripted in `scripts/bench/2x-vs-1x/run_scenario.sh:prepare_build`.
- **2.x** (React Native, exposes its views to `uiautomator`): `Play as guest` →
  `Continue` (avatar step) → `SKIP` (tutorial). Use `scripts/bench/2x-vs-1x/ui.sh tap "<text>"`.

`ui.sh texts` dumps every visible text with its bounds; that is how the 2.x driver finds nodes and
how the report's screen table was navigated.

## 4. Fixed pose and graphics profile

- **Pose:** `decentraland://open?position=0,0`. The client spawns at `position "0,-2"` (both builds
  log the same value) and nothing moves the camera: no touch input during the window.
- **Graphics profile: `HIGH`, pinned in both builds, with dynamic graphics OFF.** Not the shipped
  default — an A54 ships at `Low` with dynamic graphics ON, which caps the client at ~18 fps
  (`[DynamicGraphics] FPS limit changed, target=55.6ms`) and hides any difference between the builds
  behind that cap. With dynamic graphics off the loop still reads 16.6 ms but the real limiter
  becomes the render cost, so the number belongs to the build.

  Settings are not scriptable: they live in the app's internal storage (not reachable without root)
  and no launch flag overrides them, so each build is set once by hand after installing, and the
  install wipes it. The path, both of them taps:

  | build | path | evidence |
  |---|---|---|
  | 2.x | Settings tab `645 2094` → Graphics `400 292` → Dynamic Graphics `901 465` → profiles `538 860` → High `260 1193` | logcat `[DynamicGraphics] initialized: enabled=false, profile=3` |
  | 1.x | in world: avatar `224 105` (twice, the first tap also dismisses the tutorial) → gear `226 723` → Dynamic Graphics `2075 328` → profiles `1568 645` → scroll → High `1035 511` | screenshot of the panel (1.x logs no profile) |

  A run where the two builds report a different `profile`, or where 2.x does not log
  `enabled=false`, is void.

## 5. What is measured, and how

One "window" = one run. Nothing here relies on the client's own counters, because the two builds do
not expose the same ones (1.x has no Scene Inspector, 2.x's React Native shell does not forward
benchmark flags to the engine):

| metric | source | exact command |
|---|---|---|
| frames presented, fps avg, fps p1/p5 low, frame-time p50/p95/p99/max | Perfetto `android.surfaceflinger.frametimeline` | `bench_android.py window` |
| RSS / PSS (whole app process, React Native + engine) | `dumpsys meminfo` | same |
| CPU | `top -b -n 1 -o %CPU,ARGS` | same |
| die temperature | `dumpsys thermalservice` | same |
| cold launch | `am start -W` `TotalTime` + wall clock to `[RenderReady]` | `run_scenario.sh` |
| scene load to draw-ready | `[RenderReady]` `t50_ms` / `t90_ms` / `t100_ms` (async pipeline compiles done) | parsed from logcat |
| in-engine startup | `[Startup] global._ready end: Nms` | parsed from logcat |
| engine memory footprint | `[MemMonitor] footprint=NMB` (RSS + swap, cross-check for PSS) | parsed from logcat |
| avatars on screen | counted from the window screenshot | `screen.png` |

**fps definitions** (they must be quoted with the numbers, the two conventions are not
interchangeable):

- `fps avg` = presented frames ÷ window seconds.
- `fps p1 (1% low)` = 1000 ÷ p99 of the *inter-frame interval*; `fps p5 (5% low)` = 1000 ÷ p95.
  These are the slowest 1 % / 5 % of the run, the number a player feels during a hitch.
- Because `dur` on a frame-timeline slice is the frame's expected window (one vsync), not its
  duration, intervals are rebuilt from consecutive presentation timestamps. The method was checked
  against the in-client FPS readout (2.x HUD showed `18 FPS` while the trace gave 17.8–18.2 fps).

```bash
python3 scripts/bench/2x-vs-1x/bench_android.py --serial $SERIAL window \
  --label 2x-gp-r1 --build 2x --scenario gp --seconds 60 \
  --out docs/bench/data/2x-vs-1x-3206/android-a54/gp/2x-gp-r1
```

## 6. Run order, count and settle

- **Count:** 5 runs per build per scenario (the issue asks for ≥ 5). With `HIGH` + dynamic OFF the
  phone heats enough to throttle mid-block (16.9 → 13.6 fps inside one build), so the thermal-matched
  design below is used instead, with 3 runs per build.
- **Order:** strictly alternating `2x 1x 2x 1x …` so a thermal or network drift hits both builds
  equally. `run_scenario.sh` implements it; `run_paired.sh` alternates one run at a time because it
  has to re-apply the graphics settings on every install.
- **Thermal matching:** each run starts from the same die temperature. The app is force-stopped and
  the AP sensor (`dumpsys thermalservice`, `mName=AP`) polled until it is ≤ 34 °C; launch + settle
  then lands the window at the same temperature in both builds, and `thermal.csv` row `t=0` records
  the value actually measured (a run outside the band is re-run). Without this, block order decides
  the result.
- **Settle:** 25 s standing still after `[RenderReady]`, then the window.
- **Window length:** 30 s in the thermal-matched design (the perfetto ring buffer holds a 60 s window
  too, but 60 s of `HIGH` heats the phone by ~7 °C inside a single run). The 60 s default stays for
  scenarios measured at the shipped profile.

```bash
# shipped profile, 5 runs per build, alternating
ORDER="2x 1x 2x 1x 2x 1x 2x 1x 2x 1x" SETTLE=25 \
  scripts/bench/2x-vs-1x/run_scenario.sh gp "" 5 60

# HIGH + dynamic OFF, thermal-matched: one run at a time, in this order
for pair in 2x 1x 2x 1x 2x 1x; do
  WINDOW=30 SETTLE=25 TARGET_TEMP=34 scripts/bench/2x-vs-1x/run_paired.sh "$pair" "${pair}-hi-p$n"
done
```

Aggregation (median of the per-run medians, plus Δ % and the budget verdict):

```bash
python3 scripts/bench/2x-vs-1x/summarize.py \
  --root docs/bench/data/2x-vs-1x-3206/android-a54 \
  --out docs/bench/data/2x-vs-1x-3206/android-a54/summary.md
```

**Budget:** ≥ 1.x fps and ≤ 1.x + 100 MB RSS. Both are computed from the medians in `summary.md`.

## 7. Scenario definitions

| scenario | how it is put on screen |
|---|---|
| `gp` | `decentraland://open?position=0,0`, nothing else; other players present as found (count them in `screen.png`); measured at the shipped default (dynamic ON, profile as detected) |
| `gp-hi` | `gp` at `HIGH` + dynamic graphics OFF, 60 s windows, no thermal matching: kept as the run that shows why the cooldown is needed, not as a result |
| `gp-hi-pair` | `gp-hi` with thermal matching per run (§6): 30 s windows, 3 runs per build, alternating. This is the build-vs-build comparison |
| `gp-empty` | same, then cut the network once the world is up so no remote avatar is in view: `adb shell cmd wifi set-wifi-enabled disabled` (see §8) |
| `gp-avatars` | the worst case available without a bot farm: `gp` while the plaza is populated; report the avatar count |

## 8. Invalidation criteria

A window is void, and is re-run, when any of these holds:

1. `[RenderReady]` never appeared, or `scene_id` differs from the pinned value, or `profile`
   differs between the two builds.
2. The screenshot is not Genesis Plaza, shows a modal/dialog, or the client is not the resumed
   activity during the window.
3. `pct_missed_deadline` > 50 % **or** the trace holds fewer than 100 frames (the app was not
   animating; typically backgrounded).
4. The phone left the charger mid-run, `am start -W` reported `Status: timeout`, or a window
   sampled while another adb job was writing to the device.
5. `os` reports `level=1` low-memory in `[MemMonitor]` (memory pressure from something else).
6. Battery < 20 % (Android throttles) or the device reported a thermal status ≥ `SEVERE`
   (`dumpsys thermalservice`) before the window started.

Each run's `meta.json` keeps the raw fields needed to apply these rules after the fact.

## 9. Stress tests

All of them keep the same build, pose and profile rules as §4, and each writes its own directory.

### 9.1 Enter/exit the same scene 20 times (leak check)

```bash
for i in $(seq 1 20); do
  adb -s $SERIAL shell am start -W -a android.intent.action.VIEW \
    -d 'decentraland://open?position=0,0' org.decentraland.godotexplorer >/dev/null
  until adb -s $SERIAL shell logcat -d | grep -q '\[RenderReady\]'; do sleep 1; done
  adb -s $SERIAL logcat -c
  hold_ms=15000; sleep $((hold_ms/1000))
  adb -s $SERIAL shell am force-stop org.decentraland.godotexplorer
  adb -s $SERIAL shell dumpsys meminfo org.decentraland.godotexplorer | sed -n 's/.*TOTAL PSS:\s*\([0-9]*\).*/'"$i"',\1/p' \
    >> /tmp/enter-exit.csv
done
```

Report: PSS after cycle 1 vs cycle 20 (MB), fps in cycle 1 vs cycle 20, and the slope of the fit.
Measured on 2026-11-03 with this exact command on both builds (20 cycles each); the numbers and the
per-cycle CSV live in `data/2x-vs-1x-3206/android-a54/enter-exit/<build>/`.

### 9.2 Soak (60–90 min) on Android

```bash
# one long window; the sampler writes mem/thermal/cpu once per second
python3 scripts/bench/2x-vs-1x/bench_android.py --serial $SERIAL window \
  --label 2x-soak-90m --build 2x --scenario soak --seconds 5400 \
  --out docs/bench/data/2x-vs-1x-3206/android-a54/soak/2x-soak-90m
```

The trace buffer is a ring (128 MB). A 71-min run measured on 2026-11-03 kept only the last ~6
min of frame timeline **and dropped the process table**, so the fps query joined against nothing and
returned zero rows while the mem/thermal/CPU samplers covered the whole run. Chunk the soak into
10-min windows (`--seconds 600`) and concatenate the CSVs; do not run it as one trace.

### 9.3 Avatar push until fps drops under budget

```bash
# per step: note the visible avatar count (screenshot), then sample 30 s
python3 scripts/bench/2x-vs-1x/bench_android.py --serial $SERIAL window \
  --label 2x-avatars-n<N> --build 2x --scenario avatars --seconds 30 \
  --out docs/bench/data/2x-vs-1x-3206/android-a54/avatars/2x-avatars-n<N>
```

Avatars are other players, so N is not scriptable from the phone: the run needs either a populated
place or N bots (a desktop client per bot, each with its own profile, joining `position=0,0`).
Both builds get the same N. **Pending:** the bot farm was not provisioned for this pass; the report
quotes the as-found population instead.

### 9.4 UI stress (React Native screens)

```bash
# 20 navigations through the same screen (leak): PSS before/after + JS heap if reachable
for i in $(seq 1 20); do
  scripts/bench/2x-vs-1x/ui.sh tap "Discover"; scripts/bench/2x-vs-1x/ui.sh tap "Profile"
  adb -s $SERIAL shell dumpsys meminfo org.decentraland.godotexplorer | grep 'TOTAL PSS'
done

# tap spam: open/close a bottom sheet and a dialog in a burst, look for jank or a crash
for i in $(seq 1 20); do
  scripts/bench/2x-vs-1x/ui.sh tap "Settings"; adb -s $SERIAL shell input keyevent KEYCODE_BACK
done
adb -s $SERIAL logcat -d | grep -E "FATAL|ANR|ReactNativeJS.*Error"
```

Measured on 2026-11-05 on the 2.x build only (in 1.x these screens are drawn by the engine and have
no accessibility tree). Two corrections to the commands above, both needed to run it at all:

- `ui.sh tap "Discover"` does not work from inside the world: the HUD's buttons are engine-drawn and
  absent from the tree. The scriptable way back to the shell is `KEYCODE_BACK` (opens the RN dialog
  `Leave this world?`, whose `LEAVE` node is tappable) or a tap at `2283 240` in landscape 2340x1080
  (HUD chevron), which lands on Backpack in portrait; from there the tabs are at `y=2065` and
  `x` = Discover 228, Backpack 427, Settings 638, Profile 851.
- Time-to-content cannot be polled with `uiautomator dump` (one dump costs 0.9–3.4 s and it is
  OOM-killed on Discover): use `screencap` in a loop (~0.5 s per capture) and stop when the area
  above the tab bar matches a settled reference of the target screen. Resolution is one capture, so
  a screen that paints faster than that (Settings) can only be reported as an upper bound.

Results, CSVs and screenshots: `data/2x-vs-1x-3206/android-a54/ui-2x/`.

### 9.5 Degraded network

```bash
adb -s $SERIAL shell cmd wifi set-wifi-enabled disabled     # or
adb -s $SERIAL shell settings put global captive_portal_mode 0   # no throttler on this phone
# emulator-only alternative: adb emu network speed gsm
```

This phone has no userspace throttler and the client has no bandwidth knob, so degraded-network
runs are limited to on/off. Turning Wi-Fi off after the world is loaded is also how `gp-empty`
removes remote avatars; document which of the two a run used.

## 10. iOS

1.x is already installed on the iPhone (`1.16.0`). The 2.x build is not published as an installable
app: the R2 bucket holds the unsigned Godot export only, so it has to be signed here:

```bash
# the CI export for main@e3b7aa102 (pck + xcodeproj + frameworks + rn/ios/Godot glue)
curl -o .bench-dl/ios-2x-export.tar.gz \
  https://mobile-artifacts.dclregenesislabs.xyz/ios/main/e3b7aa102bd0fbfe3a51cc2204248a59662b3fde/export.tar.gz
# build DclRn in Release against the export's embedded-host libgodot.a
scripts/bench/2x-vs-1x/ios-build.sh .bench-dl/ios-2x-export.tar.gz \
  "$HOME/Library/Application Support/Godot/export_templates/4.6.2.stable/ios.zip"
xcrun devicectl device install app --device $IOS_UDID \
  .bench-tmp/ios-build/Build/Products/Release-iphoneos/DclRn.app
```

Measurements on iOS, with the same pose and settle:

```bash
# launch + engine milestones + memory footprint on stdout
xcrun devicectl device process launch --device $IOS_UDID --console \
  org.decentraland.godotexplorer | tee /tmp/ios-1x.log
# RSS and CPU over time (needs the pid from the line above)
xcrun xctrace record --template 'Activity Monitor' --device $IOS_UDID \
  --attach <pid> --time-limit 60s --output /tmp/ios-1x.trace
xcrun xctrace export --input /tmp/ios-1x.trace --toc
```

fps on iOS: this phone cannot be reached over `adb`, and the engine only logs `[Perf]` with
`--perf-log`, which a release RN build does not pass. Options, in order of preference:
`xctrace --template 'Animation Hitches'` (hitch counts) or `'Metal System Trace'` (frame rate from
the Metal command buffers); the fallback used when neither could be scripted is to read the FPS
counter the 2.x HUD already draws at the bottom of the screen from screenshots taken every 2 s
while standing still, and quote the median of those samples.
