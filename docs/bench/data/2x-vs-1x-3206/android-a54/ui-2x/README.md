# 2.x React Native UI on the Galaxy A54 — what it costs to reach and paint a screen

Measured on the physical device attached to this machine, for [#3206](https://github.com/decentraland/godot-explorer/issues/3206)
(2.x vs 1.x benchmark). Measurement only: no product code was touched, nothing was committed.

| | |
|---|---|
| device | Samsung Galaxy A54 5G (`SM-A546E`), Android 16, serial `R5CW60624NE`, USB |
| package | `org.decentraland.godotexplorer`, build `2.0.0.2723-e3b7aa1-staging` |
| session | guest, signed in as `ricardito` |
| display | 1080x2340 portrait (React Native shell) / 2340x1080 landscape (in-world Godot surface) |
| run | 2026-11-05, 09:52–10:12 local (device clock agrees) |
| free memory at start | 515 MB free of 7436 MB; app resident PSS ≈ 2.0–2.1 GB |

Numbers below are the raw device output. Every screenshot in `explore/` is scaled to 760 px on the
long edge to fit the artifact size budget; nothing else was edited.

## 1. Is the React Native shell reachable from inside the world? Yes — the HUD's right-edge chevron opens it, and `BACK` opens an RN dialog over the world

Started in the world (Genesis Plaza), the app already in the foreground.

```bash
adb -s R5CW60624NE shell am start -n org.decentraland.godotexplorer/com.godot.game.GodotAppLauncher
adb -s R5CW60624NE exec-out screencap -p > explore/01-world.png
scripts/bench/2x-vs-1x/ui.sh texts
```

`am start` resumed the running instance (`Warning: Activity not started, intent has been delivered to
currently running top-most instance`). In the world the accessibility tree holds almost nothing —
`01-world.texts.txt` is the whole dump:

```
1 | 403 63 418 99
2.0.0.2723-e3b7aa1-staging - 30 FPS | 850 1010 1489 1062
```

The world HUD is drawn by Godot, so its buttons are not nodes: the only way to press one is by
coordinate, read off the screenshot. Coordinates below are in the **landscape 2340x1080** space.

| attempt | action | result | evidence |
|---|---|---|---|
| a) | `input keyevent KEYCODE_BACK` | **React Native dialog appears**: `Leave this world?` / `You will go back to the menus.` / `CANCEL` (331,649) / `LEAVE` (748,649) | `02-back.png`, `02-back.texts.txt` |
| b) | `input keyevent KEYCODE_ESCAPE` | nothing: same tree as `01-world`, still the world HUD | `03-escape.png`, `03-escape.texts.txt` |
| c) | `input tap 2283 240` (top right-edge chevron of the HUD) | **React Native shell**: Backpack screen, display flips to portrait 1080x2340, bottom tab bar present | `04-exit-affordance.png`, `04-exit-affordance.texts.txt` |
| d) | not run | c) already reached the shell | — |
| e) | not run | c) already reached the shell | — |

The dialog from a) is React Native, not Godot: its nodes are real text nodes and `ui.sh tap "CANCEL"`
works on it (`tapped 'CANCEL' at 331 649`). Pressing `LEAVE` there is the documented way back to the
menus; in this pass it was dismissed with `CANCEL` and the shell was entered with c) instead.

The exact working sequence and coordinates used for the rest of this file:

```
world (landscape)  --tap 2283 240-->  Backpack (portrait)
Backpack           --tap 240 2080-->  Discover
any tab            --tap <x> 2065-->   Discover 228 | Backpack 427 | Settings 638 | Profile 851
```

A prior, abandoned attempt at this same task left a python process (`/tmp/ui_frames.py`) running and
tapping tabs every ~1.5 s; it moved the UI under my measurements twice before it was found in `ps`
and killed. Everything in this file was re-run afterwards.

## 2. Time to content, per screen

Four screens are what the screenshots show; no other screen was reachable by tapping the shell: the
bottom bar has exactly these four entries, and `Settings` sub-pages (`Graphics`, `Audio`, …) were not
opened.

### 2.1 Probe, and why there are two of them

**pixel (primary).** Tap the tab, then poll `adb exec-out screencap -p` and stop at the first frame
whose mean-absolute difference from a settled reference of that screen is < 1.0 (8-bit grey, probe
area = everything above the bottom tab bar, because the tab highlight flips on the tap itself).
Screencap cadence measured on this phone: **473, 547, 538, 533 ms** — the resolution limit of this
probe is ≈ 0.5 s, so a value equal to one cadence means "already painted at the first frame".

**a11y (cross-check).** Poll `uiautomator dump` for a text node that only exists once the screen has
rendered (`UNEQUIP`, `Dev Tools`, `EDIT PROFILE`). One dump costs **0.9–3.4 s** here, so this only
brackets the paint: elapsed lies in `(last_miss − t_tap, hit − t_tap]`.

Both probes start from a neutral tab that alternates between two predecessors per screen.

References are full-resolution captures taken while settled on each screen (`explore/screen-*.png`);
their self-difference 1 s apart was 0.000 (Discover, Settings bottom sheet), 0.238 (Backpack) and
0.003 (Profile bottom sheet). The Profile **avatar** idles at 60 fps and never matches (band diff up
to 8.5), so its probe area is the bottom sheet, y 1050–1950.

### 2.2 Results — pixel probe, 5 reps per screen (`ttc-pixel.csv`)

| screen | signal (a11y) | n | median ms | min ms | max ms | reps ms |
|---|---|---|---|---|---|---|
| Discover | *(a11y unusable, see §5)* | 5 | 2268 | 1930 | 2430 | 2226, 2268, 1930, 2342, 2430 |
| Backpack | `UNEQUIP` | 5 | 1851 | 1220 | 1955 | 1955, 1952, 1271, 1851, 1220 |
| Settings | `Dev Tools` | 5 | 514 | 499 | 525 | 525, 514, 505, 499, 515 |
| Profile | `EDIT PROFILE` | 5 | 1160 | 1101 | 1182 | 1101, 1107, 1176, 1182, 1160 |

Settings is the only screen whose value sits on the probe resolution: every one of the five reps
matched at the **first** screencap (~0.5 s), so its true time-to-content is ≤ 500 ms and not
distinguishable from "instant" at this cadence. The other three are resolved across two to four
screencaps.

### 2.3 Results — a11y probe, 5 reps per screen (`ttc-a11y.csv`)

Brackets `(lower, upper]` in ms; `0` means the very first dump already saw the text.

| screen | reps `(lower, upper]` ms | miss dumps |
|---|---|---|
| Backpack | (0, 3393], (24528, 28224], (0, 3385], (2279, 4768], (0, 3323] | 0, 25, 0, 5, 0 |
| Settings | (0, 2444], (936, 3365], (1318, 3727], (892, 3281], (1294, 3683] | 0, 2, 3, 2, 3 |
| Profile | (1792, 4153], (0, 2978], (1648, 2123], (0, 2973], (1791, 4127] | 4, 0, 3, 0, 4 |

Read as a sanity check on §2.2, not as the headline: the upper bounds are 1.5–2× the pixel numbers
because they include one full dump (0.9–3.4 s). Backpack rep 2 is an outlier in the other direction —
25 misses over 24.5 s before the text appeared — consistent with the tab tap being lost rather than
the screen taking 28 s; it is left in the CSV unaltered. The a11y probe cannot resolve below ~0.9 s
on this phone, which is why the pixel probe carries the table.

## 3. Navigation leak: 20 Discover↔Profile cycles (`navleak.csv`)

```bash
pss() { adb -s R5CW60624NE shell dumpsys meminfo org.decentraland.godotexplorer \
        | sed -n 's/.*TOTAL PSS:[[:space:]]*\([0-9]*\).*/\1/p' | head -1; }
pss                                          # cycle 0
for c in $(seq 1 20); do
  adb -s R5CW60624NE shell input tap 228 2065   # Discover
  sleep 1
  adb -s R5CW60624NE shell input tap 851 2065   # Profile
  sleep 1
  [ $((c % 5)) -eq 0 ] && echo "$c,$(pss)"
done
```

| cycle | TOTAL PSS kB | MB |
|---|---|---|
| 0 | 2125134 | 2075 |
| 5 | 2113766 | 2064 |
| 10 | 2100471 | 2051 |
| 15 | 2069660 | 2021 |
| 20 | 2069276 | 2021 |

Delta 0 → 20: **−55.9 MB**. Monotonic **decreasing**, i.e. no navigation leak in 20 cycles; the
allocator gives back memory as the tab history grows. PSS is dominated by the in-world Godot surface
still resident behind the shell (~2.0 GB), so this test can only see a leak larger than that noise
floor; it cannot see a small JS-side leak. JS heap was not readable: release build, no Metro or
DevTools bridge.

## 4. Tap spam (`tapspam.csv`)

20 taps at the screen centre, 150 ms apart, from the Profile screen:

```bash
adb -s R5CW60624NE shell pidof org.decentraland.godotexplorer
adb -s R5CW60624NE exec-out screencap -p > explore/06-tapspam-before.png
adb -s R5CW60624NE logcat -c
for i in $(seq 1 20); do adb -s R5CW60624NE shell input tap 540 1170; sleep 0.15; done
adb -s R5CW60624NE shell pidof org.decentraland.godotexplorer
adb -s R5CW60624NE exec-out screencap -p > explore/07-tapspam-after.png
adb -s R5CW60624NE logcat -d | grep -E "FATAL|ANR|ReactNativeJS.*Error|AndroidRuntime.*E"
```

| | before | after |
|---|---|---|
| pid | 21219 | 21219 (survived) |
| TOTAL PSS | 2063505 kB | 2064897 kB (**+1 MB**) |
| logcat FATAL/ANR/ReactNativeJS Error | — | no matching lines |

`06-tapspam-before.png` and `07-tapspam-after.png` are indistinguishable: the centre taps landed on
the profile sheet, opened nothing, and left the screen on Profile.

## 5. What could not be measured, and why

- **Discover through the accessibility tree.** `uiautomator dump` is killed on that screen and never
  writes a file — 9 plain attempts across two runs plus 2/2 `--compressed`, while the same command on
  Settings returns 37592 bytes. Raw output in `a11y-discover-failure.log`. The screen's carousel
  keeps the window non-idle, so the dump waits and the process is OOM-killed (only ~515 MB free).
  Discover is therefore measured with the pixel probe only.
- **Sub-500 ms resolution.** Both probes are limited by the device: 0.5 s per screencap, 0.9–3.4 s
  per a11y dump. Screens that paint faster than one screencap (Settings) can only be given an upper
  bound. A frame-accurate number would need `screenrecord` frame timestamps, not used here.
- **JS heap / per-screen JS cost.** Release APK, no DevTools bridge reachable; `dumpsys meminfo`
  reports one process, so RN allocations cannot be separated from the Godot surface.
- **Screens below the tab bar** (`Settings` → `Graphics`, the `Emotes` tab, the wearables categories).
  No screenshot in this pass shows them, so they are not enumerated and not timed.
- **`LEAVE` on the "Leave this world?" dialog.** The dialog was captured and dismissed with `CANCEL`;
  the shell was entered with the world-HUD chevron instead, so the time-to-menu of the documented
  `BACK` → `LEAVE` path was not measured.
- **`BACK` inside the shell.** It is context-dependent and was not part of the protocol: in the world
  it opens the leave dialog, in the shell it opened the Genesis Plaza place sheet (a bottom sheet with
  `JUMP IN`). No screenshot of that state was kept, and while that sheet is open `uiautomator dump` is
  killed the same way as on Discover.
- **Attribution of world-HUD taps.** The HUD is Godot-drawn; the right-edge chevron at (2283, 240)
  opens Backpack, but no other HUD icon was mapped.

## 6. Files

```
explore/01-world.png                 world HUD, landscape, right-edge chevrons visible
explore/01-world.texts.txt           the entire a11y tree of the world (2 nodes)
explore/02-back.png                  KEYCODE_BACK -> "Leave this world?" RN dialog
explore/02-back.texts.txt            that dialog's nodes (provenance in the file header)
explore/03-escape.png                KEYCODE_ESCAPE -> unchanged world
explore/03-escape.texts.txt
explore/04-exit-affordance.png       tap 2283 240 -> Backpack (RN shell reached)
explore/04-exit-affordance.texts.txt
explore/05-shell-discover.png        Discover settled (entered from Backpack)
explore/06-tapspam-before.png        tap spam, before
explore/07-tapspam-after.png         tap spam, after
explore/screen-discover.png          settled reference used by the pixel probe
explore/screen-backpack.png
explore/screen-settings.png
explore/screen-profile.png
ttc-pixel.csv                        Phase B, pixel probe, 20 reps
ttc-a11y.csv                         Phase B, a11y probe, 15 reps
navleak.csv                          Phase C, PSS per 5 cycles
tapspam.csv                          Phase D
a11y-discover-failure.log            uiautomator dump killed on Discover, works on Settings
```

Device left on the `Discover` screen, portrait, app in the shell.

## 7. Reproducing

The probes were driven by a scratch script outside the repo (`/tmp/uiv/ttc.py`, 200 lines: tab
coordinates, per-screen probe area and threshold as in §2.1); it is not part of the tree. The
commands in §1–§4 plus those thresholds are enough to rebuild it. No Perfetto trace was taken and no
git state was changed.
