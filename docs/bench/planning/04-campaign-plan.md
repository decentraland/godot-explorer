# 04 — Re-measurement campaign plan for #3206 (2.x vs 1.x, A54 primary, 16e labelled)

Inputs: `00-constraints.md`, `01-data-inventory.md`, `03-tooling.md`, the code investigation
(`…/subagent-artifacts/outputs/a44ecafd…/context.md`), `docs/bench/2x-vs-1x-3206-protocol.md`, and the
raw runs under `docs/bench/data/2x-vs-1x-3206/`. Nothing here is a new measurement. Measurement-only:
no client change, no GDScript, no fix for the sky bug in this PR.

Device state read today (Galaxy A54, `R5CW60624NE`, `samsung/a54xnsxx/a54x:16/BP4A.251205.006/A546EXXSNFZI1:user/release-keys`):

| probe | value | consequence for the plan |
|---|---|---|
| `dumpsys thermalservice` → `Thermal Status` | `1` (LIGHT) at idle | "status == NONE before every run" is unreachable → gate is relative (§4.2) |
| `mName=AP` / `mName=SKIN` | `49.1 °C` / `41.8 °C` (`mStatus=1`) | a 34 °C gate (protocol §6) is not achievable today |
| battery | `level 13`, `status 3` (discharging) | **hard blocker** before F0 (§4.1) |
| `settings get system peak_refresh_rate` / `min_refresh_rate` | `null` / `null` | the panel is not pinned by anything today |
| `dumpsys display` → `mActiveModeId` | `1` | id ↔ Hz mapping is undocumented here; the trace is the decisive check (§2.5) |

---

## 1. Objective and the claims it will licence

**Objective.** Produce a defensible answer to one question: *at one pinned scene, one pinned settings
configuration and one pinned light, does the 2.x build (RN shell + Godot) deliver worse frame pacing
than the 1.x build on the Galaxy A54?* Frame pacing is measured as cadence quality, not mean fps.

### 1a. Publishable without controlling the sun

These are invariant to the sun because they are either light-insensitive or within-build.

| id | claim | source of the number |
|---|---|---|
| A1 | Cold launch → draw-ready time, per build, median [min–max] of 6 runs. | `[RenderReady]` `t50_ms/t90_ms/t100_ms` in `launch.log`, cross-checked with `am start -W TotalTime` in `launch.json` |
| A2 | Steady-state and peak PSS/RSS + swap of the app process at the pinned scene/pose, per build. | `mem.csv`, `meta.json.rss_steady_kb/rss_peak_kb` |
| A3 | Memory growth slope over a 10-min session, per build (leak probe, no light sensitivity). | `mem.csv` linear fit on `total_pss_kb` |
| A4 | Per-build cadence structure: the client is vsync-locked to integer multiples of the panel period; the modal cadence, in periods, is N. | `frames.csv` `window_ms` + `interval_ms` |
| A5 | The 2.x shell's own navigation cost (2.x only, no 1.x counterpart): time-to-content and jank for 20 navigations. | `ui-2x` methodology (protocol §9.4); reported as a 2.x-only statement |
| A6 | Categorical light state, not a performance number: with `Dynamic Skybox` off and `Custom Skybox = Midday`, 2.x renders midnight and 1.x does not (2.x luma 33.1 vs 1.x 110.6–111.9 on parcel 100,100). | `android-a54/sky-fixed-midday/RESULT.json`, `1x-skyfix-r2/r3/state.json`; cause at `lib/crates/dcl_engine/src/sky/skybox_time.rs:131-134` |

### 1b. Claims that need a matched-light pair

| id | claim | why the sun matters |
|---|---|---|
| B1 | Primary: fraction of the window spent in ≥2-period stalls (cadence quality), 2.x vs 1.x. | The sun changes render cost: within one build and scenario fps spans 26.87 at luma 101.6 → 15.83 at luma 117.6 → 14.06 at luma 86.0 (`gp-hi-v3/1x-v3-r1..r3/meta.json` + `state.json`), i.e. a 1.9× swing from light alone |
| B2 | Mean fps, frame-time p50/p99, 1 % low. | same |
| B3 | CPU % of the process during the window. | fragment/shadow cost tracks irradiance |
| B4 | The "budget" verdict (2.x fps ≥ 1.x fps). | same as B1 |

### 1c. Claims this campaign explicitly refuses to make

1. **Any single "2.x is X % slower" number whose two arms are not light-matched** (cropped luma
   difference > 5 points, §2.6). If the campaign cannot produce such a pair, it publishes two arm
   bands and no delta, and says so in the first paragraph of the results.
2. **Nothing from the void scenarios.** `gp-hi-pair` (profile drift: 0/0/2 in `INVALID.md`),
   `gp-empty` (connection-lost modal), `gp-hi`/`gp`/`gp-sun-drift`/`smoke` (no `state.json`, no
   per-run profile evidence — `01-data-inventory.md` §3–§4), `soak` (`frames=0`).
3. **No GPU-time claim.** No GPU devfreq node on this device (`00-constraints.md`), no
   `gpu_counter`/`gpu_work_period` evidence in any existing trace → "the engine's GPU work is X" is
   refused. Frame Timeline measures presentation, not GPU work (`03-tooling.md` §1.1).
4. **No p99.9 and no sub-second-hitch claim from a 30 s window** (~900 frames: `03-tooling.md` §1.2.5).
5. **No pooled Android+iOS number**, no pooled 120/60 Hz number.
6. **No causal attribution to React Native.** The two build artefacts differ in engine version
   (1.16.1 vs 2.0.0), shell (engine-drawn HUD vs RN), and settings plumbing. The campaign can say
   "the 2.x artefact is slower here"; it cannot say "because of the RN shell". A claim of that shape
   needs an engine-only 2.x control that does not exist (`rn/android/app/src/godot/java/com/dclrn/MainActivity.kt:104-110`
   passes at most `--force-mobile --no-pulse`).
7. **No thermal-cliff claim.** No long positive-control run is budgeted; the campaign measures a
   gated steady-ish state and does not describe where throttling begins.
8. **No significance language at n < 4** (exact two-sided Mann–Whitney floor at 3 vs 3 is p = 0.10;
   `03-tooling.md` §6.1).
9. **No extrapolation** to another scene, another device, or a later plaza deployment.
10. **No "regression since 1.16.1"** — no bisection is in scope.

---

## 2. The condition

### 2.1 Primary condition P1 (`gp-hi-p1`)

| field | value |
|---|---|
| scene | Genesis Plaza, `decentraland://open?position=0,0` |
| scene version (pinned) | `bafkreihugp3rlcjryzkenaby4fb73gjdhvdxdkvu7uihhfqyxjqnhkkgxy` — must equal the `scene_id` of that run's own `[RenderReady]` |
| spawn | client-reported `position` `"0,-2"`, identical string in every run |
| profile | `graphic_profile = 3` (HIGH), `dynamic_graphics_enabled = false`, both builds |
| sky policy | `dynamic_skybox = false`, `skybox_time = 43200` (`Custom Skybox = Midday`) on both builds. In Genesis Plaza this switch is **inert**: the scene's `SkyboxTime` has higher precedence (`lib/crates/dcl_engine/src/sky/skybox_time.rs:81-96`) than the player setting (`:121-135`), so the light is the scene's own time on both builds. It is still set, because (a) it is the only state the 2.x bug can corrupt into view, and (b) a run where the scene override does not arrive falls to `settings_target()` → 2.x midnight, which the luma band detects |
| pose | no input during the run; camera mode untouched (no external control exists — `engine/core_link.rs` has no camera command) |
| refresh | `min_refresh_rate = 120.0`, `peak_refresh_rate = 120.0`; Samsung "Motion smoothness" set to **Standard/120 Hz, not Adaptive** by hand, screenshot as evidence |
| window | 30 s after a 25 s settle, app in foreground, charger detached |
| avatars | uncontrolled (no hide mechanism anywhere — code investigation §6); visible count recorded and required to be within ±2 of the pilot's count |

### 2.2 Secondary condition S1 (`gp-shipped`)

Same scene/pose/refresh, **shipped configuration as found**: `dynamic_graphics_enabled = true`,
`graphic_profile` as the build ships (A54 ships `Low` + dynamic ON, protocol §4) — i.e. what a player
gets. 45 s windows, one block of 3 per build. Reported **descriptively only** (n=3, no significance
claim). This is the arm that answers "does the shipped experience differ", not the arm that separates
the builds.

### 2.3 Secondary condition S2 (`gp-sun-sensitivity`, control arm)

Same scene/pose/profile as P1 but `dynamic_skybox = true` on both builds, 20 s windows, 3 runs per
build spread over ≥ 40 min so the world clock has advanced (world clock = UTC × 12, `120`-minute
cycle — `lib/crates/dcl_engine/src/godot_classes/dcl_global_time.rs:10-27`). Purpose: measure **in our
own harness** how much fps moves per luma point in this scene, so the final report can state how large
a light mismatch would be needed to fake the P1 delta. It is a calibration arm, never a comparison arm.

### 2.4 How each field is verified from the device (not from the panel)

| field | device evidence required | pass value |
|---|---|---|
| build identity | `dumpsys package org.decentraland.godotexplorer` `versionName`/`versionCode` + `shasum -a 256 .bench-dl/apk-*.apk` | `1.16.1`/`2688` + 1x hash, or `2.0.0`/`2723` + 2x hash, as recorded at campaign init |
| profile | `[RenderReady] {"…","profile":3}` from the pid returned by `pidof` **at that launch**, and zero `Profile (downgrade|upgrade)` lines for that pid | `3`, zero changes |
| dynamic graphics off | zero `[DynamicGraphics] FPS limit changed` lines for that pid during `[t0, t1]`; on 1.x, zero `[DynamicGraphics]` lines of any kind | zero. **Direct verification does not exist** (the `initialized: enabled=…` line is 1.x-only, per `run_paired.sh`): this is a surrogate, and the report must say so |
| dynamic skybox state | 2.x: `decentraland://open?position=0,0&rust-log=dclgodot::sky=debug` in the **pilot only**, requiring `SkyboxTime SDK control changed: active=true` (`scene_runner/components/skybox_time.rs:77-90`). Measured windows run **without** debug logging (1.x has no equivalent, so enabling it on one arm only would be an asymmetry). Otherwise: the luma band | `active=true`, then luma in band |
| scene id | `scene_id` in `[RenderReady]` | the pinned hash above |
| pose | `position` in `[RenderReady]` + screenshot shows no modal | `"0,-2"`, no modal |
| refresh mode | `settings get system peak_refresh_rate` = `120.0`, `min` = `120.0`, **and the trace's own modal `window_ms`** (= `af.dur/1e6`, the expected frame window = one vsync) | `8.333 ms ± 0.05`; `16.667` means the panel ran at 60 Hz → invalid unless *both* arms are 60 Hz |
| battery / charging | `dumpsys battery` `level` and `status` at gate time | level ≥ 20, `status` discharging (`ac:false usb:false`) |

### 2.5 The `mActiveModeId` problem

`min_refresh_rate`/`peak_refresh_rate` are `null` today and the active mode is `1`, with no documented
map from id to Hz on this build. So the *settings* read-back is necessary but not sufficient; the
decisive per-run check is the modal `window_ms` in `frames.csv`. A run whose modal `window_ms` is not
`8.333` is invalid for every cadence claim (§8).

### 2.6 The luma acceptance band

- **Metric:** `luma_crop` — mean of PIL `Image.convert("L")` (Rec.601 weights, sRGB-encoded values) over
  the rectangle `x ∈ [0.15W, 0.85W]`, `y ∈ [0.10H, 0.80H]` of the screenshot, i.e. the world viewport
  minus the top HUD strip and the bottom 20 % where both shells draw their controls. The historical
  `luma_mean` in `state.json` is the full-frame Rec.601 mean and is **not interchangeable** with it;
  both are written to every manifest. Rationale for cropping: the two shells have different UI
  surfaces, so full-frame luma carries a build-correlated UI bias (`03-tooling.md` §4.1.3).
- **Band definition:** `band_build = [L* − 5, L* + 5]`, where `L*` is the median `luma_crop` of the two
  pilot runs of that build (§4.1 F4). Per build, because cross-build luma also carries the UI/residual
  light bias; the cross-build check is a *separate* rule (below).
- **Why ±5:** within a session the light is stable to 0.6 points over 90 s (98.0 → 98.6, measured
  today) and 0.6 points over 12 windows 111 s apart (56.4 → 61.0); a relaunch moves it by ~1 point
  within a build (102.3 vs 103.1) and by ~4 points against the in-session value (98 → 102/103). The
  condition changes we must exclude are ≥ 20 points (`gp-hi-fixed-sky`: 1.x 104.6–108.8 vs 2.x
  72.6–77.6). ±5 is ≈5× the observed relaunch noise and ≤ ¼ of the smallest condition effect on
  record.
- **Cross-build match rule:** a *pair* is light-matched when both members are in their build's band
  **and** `|luma_crop(2x) − luma_crop(1x)| ≤ 5`.
- **What happens outside the band:** the run is not deleted and not silently averaged. It is written
  with `flags: ["luma_out_of_band"]`, excluded from the primary comparison, and shown in the
  luma-stratified table (§7.3). A pair outside the cross-build rule is `pair_luma_mismatch` and is
  likewise excluded from the delta, never "adjusted". **If ≥ 2 of the 6 runs of either arm are out of
  band, the arm aborts** and the campaign re-runs the preflight (F3/F4) before continuing; if the
  re-preflight fails again, the campaign falls to the fallback design (§2.7) or to claim class 1c.1.

### 2.7 Fallback design if F3 (light stability in GP) or F4 (cross-build match) fails

Parcel `100,100` (scene-free, so no `SkyboxTime` override and the setting governs), `dynamic_skybox =
true` on both builds. Then the sun is a pure function of UTC: `(utc_unix_s × 12) mod 86400` in-game
seconds, i.e. it repeats exactly every **7200 s of UTC** (86400/12; `dcl_global_time.rs:10-27`). The
window is started when `int(time.time()) % 7200 ≤ 2`, which matches the two arms' sun to within
`12 × 2 = 24` in-game seconds (0.03 % of a day) without any client change.

Conditional gates for the fallback: (i) F4b — 1.x obeys the same 7200 s period, verified by three
1.x windows at the same phase (+0, +1200, +2400 s) whose `luma_crop` differ by ≤ 3; if 1.x does not
repeat with that period, the fallback is rejected and the campaign publishes claim class 1c.1.
(ii) A run whose `(start_unix % 7200) > 5` is invalid (schedule slip).

---

## 3. Arms and sample size

### 3.1 Arms

| arm | condition | runs/build | windows | use |
|---|---|---|---|---|
| P1 | `gp-hi-p1` (§2.1) | 6 | 30 s | the comparison |
| S1 | `gp-shipped` (§2.2) | 3 | 45 s | descriptive, shipped experience |
| S2 | `gp-sun-sensitivity` (§2.3) | 3 | 20 s | calibration only |
| F | preflight ledgers (§4.1) | 1–3 | 15–30 s | pass/fail, not results |

24 measured windows + 8 preflight windows.

### 3.2 Why n = 6 per arm

Two-sample normal approximation, α = 0.05 two-sided, power 0.80 (`03-tooling.md` §6.1):
`n = 2·(1.96+0.84)²·(CV/Δ)² = 15.7·(CV/Δ)²`, so the smallest detectable relative difference at n runs
is `MDR(n) = CV·√(15.7/n)`.

| n/arm | `√(15.7/n)` | MDR at CV 10 % | CV 15 % | CV 20 % | exact two-sided Mann–Whitney floor |
|---|---|---|---|---|---|
| 3 | 2.29 | 22.9 % | 34.3 % | 45.8 % | 2/20 = 0.100 |
| 4 | 1.98 | 19.8 % | 29.7 % | 39.6 % | 2/70 = 0.029 |
| **6** | **1.62** | **16.2 %** | **24.3 %** | **32.4 %** | **2/924 = 0.0022** |

Justification for 6:

- The planning figure is the observed 10–20 % spread → n = 6 keeps MDR at 16–32 %.
- The best existing matched-ish evidence suggests the effect is larger than that: `gp-hi-fixed-sky`
  gives 1.x 17.48/17.93/18.82 fps (CV 3.7 %) vs 2.x 15.18/14.42/13.47 (CV 5.8 %), ratio 0.81 (−19 %).
  At those CVs MDR(6) is 6–9 %, comfortably enough; at the pessimistic CV 15 % the campaign declares
  itself "descriptive, MDR 24 %" and does **not** claim the 19 % effect as detected — the bootstrap CI
  carries the uncertainty instead.
- n = 6 is also the smallest n that makes an exact two-sided p < 0.05 arithmetically possible with a
  comfortable margin (0.0022), and it splits into two balanced blocks of 3 for the ABBA order (§4.3).

### 3.3 Time budget

Per-run cost, P1 (gate-dominated):

| step | minutes |
|---|---|
| thermal gate / cooldown (§4.2), floor 8, expected 12 today | 12.0 |
| launch + `logcat` to `[RenderReady]` | 0.7 |
| settle | 0.4 |
| 30 s window + trace pull + luma capture + post snapshot | 0.8 |
| `manifest.py` assembly + verdict | 0.2 |
| **per P1 run** | **≈ 14.1** |

| item | minutes |
|---|---|
| P1: 12 runs × 14.1 | 169 |
| S1: 6 runs × 13 (45 s windows) | 78 |
| S2: 6 runs × 9 (20 s windows, 180 s floor) | 54 |
| preflight F0–F5 (§4.1) | 82 |
| 8 installs (P1 2, S1 2, S2 2, preflight 2) × 4 min incl. taps | 32 |
| soak before the first P1 block | 20 |
| **total device time** | **≈ 435 min (7.3 h)** |

Split over two days at block boundaries, never inside a block: day 1 = F0–F5 + P1 blocks 1–2
(1x r1–r3, 2x r1–r3); day 2 = P1 blocks 3–4 (2x r4–r6, 1x r4–r6) + S1 + S2. Each build appears on
both days, so a day effect does not land on one arm; `manifest.py` records `day` so it can be checked.

### 3.4 Cut order if time runs short

1. Cut **S2** (−54 min). The sun-sensitivity statement is then cited from existing data instead:
   `gp-hi-v3/1x-v3-r1..r3` (26.87 fps at luma 101.6, 15.83 at 117.6, 14.06 at 86.0).
2. Cut **S1 to n = 2** (−26 min). It is already descriptive; the loss is one run per build.
3. Cut **P1 to n = 4** (−56 min). Consequences, stated in the report: exact MWU floor 0.029, MDR
   19.8/29.7/39.6 % at CV 10/15/20 %.
4. **Floor:** never below 4 + 4. Below that, the campaign publishes bands and refuses the delta
   (claim 1c.1) rather than report an underpowered comparison.

---

## 4. Run order, pairing, preflight

### 4.1 Preflight block — runs before any measured arm, can abort the campaign

| id | on which build | what it measures | pass | fail |
|---|---|---|---|---|
| **F0** controls | both | battery ≥ 80 % and discharging; auto-brightness off + fixed `screen_brightness`; `svc power stayon true`; DND on; auto-rotate off; Wi-Fi on the same AP; "Motion smoothness" not Adaptive (screenshot); `cmd package compile -m speed-profile -f org.decentraland.godotexplorer` after every install; `adb version`, `trace_processor_shell` sha256 recorded | all set, recorded | **abort** the campaign (battery is at 13 % today) |
| **F1** display pin | 2.x | `display_mode.sh pin 120` → read-back + a 15 s trace → modal `window_ms` | read-back `120.0`/`120.0` **and** modal `window_ms` = 8.333 ± 0.05 | try `pin 60`; if that holds, run the whole campaign at 60 Hz and label it; if neither holds, **abort all cadence claims** (fps-only campaign) |
| **F2** harness self-test | none (offline) | `cadence.py --check` on `android-a54/gp-hi-fixed-sky/2x-sky-r1/frames.csv` vs that run's `meta.json` | `p50` within 0.5 ms of `66.69`, `p99` within 1.0 ms of `83.13`, modal `window_ms` 8.333 ± 0.05 | **abort**: tooling is not trustworthy |
| **F3** GP light stability | 2.x | 5 `luma_crop` captures 60 s apart, one session, no relaunch | `max − min ≤ 2.0` **and** `|slope| ≤ 0.5 luma/min` | GP light is time-varying → fall back to §2.7 (or F4 must succeed with a wider band, decided then, in writing, before the runs) |
| **F4** light ledger / match | both (1 run each, 20 s, `rust-log=dclgodot::sky=debug` on 2.x) | `luma_crop`, `luma_full`, the `SkyboxTime SDK control changed` line, `[RenderReady]` profile/scene/position | 2.x shows `active=true`; both profiles 3; `|L*(2x) − L*(1x)| ≤ 5` | matched pair not established → §2.7 fallback; if that also fails, **abort the delta** and publish 1c.1 |
| **F5** thermal reproducibility | 2.x | two consecutive gates (§4.2) reaching the target | both reach target, `|ΔAP| ≤ 1.0 °C` | switch to the **throttled steady state** protocol: 20 min soak, no cooling, gate = soak-end AP ≤ +1.0 °C, and label every number "throttled steady state" |
| **F6** settings reachability | both | 2.1 applied by the harness, then two consecutive launches must log `"profile":3` | 2/2 | **abort**: the settings arm is uncontrolled |

Any **abort** stops the campaign before a single measured window. Any **design change** (F3/F5) is
written into the campaign file (`campaign.json`) before continuing, so the report can state which
design was actually executed.

### 4.2 Cooldown rule between runs (conjunctive, measured)

A run may start only when **all** hold, polled every 20 s after `am force-stop`:

1. `dumpsys thermalservice` `Thermal Status` ≤ **1** — not 0: today's idle baseline is already 1
   (`AP 49.1 °C`), so requiring NONE would stall the campaign indefinitely;
2. `mName=AP` ≤ **40.0 °C**;
3. `mName=SKIN` ≤ session-baseline + **1.0 °C**;
4. ≥ **480 s** since the previous window's end;
5. battery level ≥ 20 %, `status 3` (discharging).

Abort the block (not the campaign) if the gate is not met in **1500 s**; write `gate.json` with the
elapsed wait, the final values, and `"gate_ok": false`. Arm-level balance is then checked, not
assumed: the median `start_ap_c` of the two arms must differ by ≤ 1.0 °C, else the block is re-run.

### 4.3 Exact run order (builds alternate at block level, ABBA, balanced for linear drift)

```
[install 1x if needed]   block A1 : p1-1x-r1  p1-1x-r2  p1-1x-r3
[install 2x]             block B1 : p1-2x-r1  p1-2x-r2  p1-2x-r3
                         block B2 : p1-2x-r4  p1-2x-r5  p1-2x-r6     (no install between B1/B2)
[install 1x]             block A2 : p1-1x-r4  p1-1x-r5  p1-1x-r6
```

Positions 1–12; 1.x occupies 1–3 and 10–12 (mean position 6.5), 2.x occupies 4–9 (mean 6.5): a linear
thermal/ambient drift hits both arms equally. Only **two** installs (1x→2x at position 4, 2x→1x at
position 10), because installs cost 2–4 min and wipe settings. Settings are re-applied by
`run_paired.sh` after every install and verified by F6's rule on every launch.

Why not run-by-run alternation: it costs an install (uninstall + install + re-tap settings ≈ 6–8 min)
per alternation — 12 alternations instead of 2 installs would add ~70 min and 12 settings-application
opportunities, which is the exact failure that voided `gp-hi-pair`.

S1 runs after day-2 block A2 (one install per build: 1x S1 runs, install 2x, 2x S1 runs). S2 runs
last with the 2x install still in place if possible; otherwise 2 more installs are counted in §3.3.

**Pair definition.** A pair is a `(p1-1x-rN, p1-2x-rM)` couple *inside the same ABBA half* (positions
1–6 or 7–12), formed post hoc by matching on `start_ap_c` (closest), and accepted only if
`|Δ start_ap_c| ≤ 1.0 °C` **and** the §2.6 cross-build luma rule holds. Pairs are a secondary analysis
(n ≤ 6); the primary analysis is the unpaired arm comparison, because the build switch separates the
two members of a pair by ≥ 5 runs in time.

---

## 5. Metrics per run

All cadence numbers are computed from `frames.csv` (durable, one row per presented frame:
`ts, interval_ms, jank_type, on_time_finish, present_type, display_frame_token`, plus the new
`window_ms` column, §6.2), **not** from a re-run SQL query, so a trace-processor upgrade cannot move a
published number.

Definitions: `P` = modal `window_ms` of that run (one vsync); `periods_i = interval_ms_i / P`;
`modal_periods` = mode of `round(periods_i)`; on-cadence = `|periods_i − modal_periods| ≤ 0.25`;
**hiccup** = `modal_periods + 1 ≤ periods_i < modal_periods + 2`; **stall** = `periods_i ≥
modal_periods + 2` (1-skip / ≥2-skip, `03-tooling.md` §1.2.4).

| metric | how computed | from which file | why it is in the report | class |
|---|---|---|---|---|
| `pct_window_in_stalls` | `Σ interval_ms over stalls / Σ interval_ms` | `frames.csv` → `cadence.json` | **primary**: what a player feels, scale-free across differing modal cadences | **primary** |
| `modal_periods` | mode of `round(interval_ms/P)` | `frames.csv` → `cadence.json` | tells the reader *which* cadence each arm sits on; mean fps hides it (measured p50 land on 4, 6, 7, 8, 9 periods) | secondary |
| `on_modal_frac` | fraction of intervals on-cadence | `frames.csv` | cadence quality within an arm | secondary |
| `stalls_per_min`, `hiccups_per_min` | counts / `span_min` | `frames.csv` | rate view of the same tail | secondary |
| `stall_periods_hist` | histogram of `round(periods_i)` | `frames.csv` | shows whether the tail is 8-period or 20-period | diagnostic |
| `jank_type_counts` | `count by jank_type` (platform's own taxonomy) | `frames.csv` | must agree with our re-derivation; disagreement is a tooling bug | diagnostic |
| `fps_avg`, `interval_p50/p95/p99`, `fps_p1_low`, `fps_p5_low`, `fps_median`, `interval_max_ms` | existing SQL in `bench_android.py:140-179` (`frames_query`), per-frame rows written at `:226-240` | `meta.json.frames` | the numbers the previous report quoted; kept for continuity and for the fps budget verdict | secondary |
| `cpu_mean_pct` | 1 Hz `top -b -n 1 -o %CPU,ARGS` | `cpu.csv`, `meta.json.cpu_mean_pct` | shows whether a pacing difference is CPU-bound; light-sensitive, hence only inside a matched pair | secondary |
| `pss_steady_mb`, `pss_peak_mb`, `swap_pss_mb` | `dumpsys meminfo` 1 Hz | `mem.csv`, `meta.json` | the ≤ 1.x + 100 MB budget; light-insensitive (claim A2) | secondary |
| `luma_crop`, `luma_full`, `band`, `in_band` | §2.6 | `screen.png` → `state.json` | the condition record; without it no delta may be published | condition (gate) |
| `start_ap_c`, `end_ap_c`, `start_skin_c`, `start_status`, `gate_wait_s` | `dumpsys thermalservice` | `thermal.csv`, `gate.json` | decides whether the pair is thermal-matched | condition (gate) |
| `refresh`: `min`/`peak` read-back, `active_mode_id`, `modal_window_ms` | `settings get`, `dumpsys display`, `frames.csv` | `display.json`, `cadence.json` | the silent confounder; 16.667 ms means the run is void | condition (gate) |
| `version_name`, `version_code`, `apk_sha256` | `dumpsys package`, `shasum` | `manifest.json` | build identity, per run not per campaign | condition (gate) |
| `profile`, `scene_id`, `position`, `profile_changes` | `[RenderReady]` + `Profile (downgrade|upgrade)` count, pid-filtered | `launch.log` | the exact check that voided 3 of 6 runs last time | condition (gate) |
| `avatars_present` | counted from `screen.png` by the operator, recorded | `manifest.json` | uncontrolled factor; blocks pooling if arms differ by > 2 | diagnostic |
| iOS: `fps_samples[]`, `fps_median`, `pct_seconds_below_target` | 1 Hz HUD samples | iOS `meta.json` | §10 only | separate dataset |

**Direction-of-bias note to print with the primary metric:** an arm on a *coarser* modal cadence needs
a larger absolute miss before a frame counts as a stall. So if 2.x (coarser) still shows a higher
`pct_window_in_stalls`, the conclusion is conservative; if 2.x shows a *lower* value while sitting on
a coarser cadence, the result is ambiguous and must be reported as such, together with `on_modal_frac`
and `modal_periods`.

---

## 6. Harness changes required

Rule: additive wherever possible; `01-data-inventory.md` §6 shows the existing artefacts already have
three schema families, so nothing reads `frames.csv` other than `bench_android.py` and the new
`cadence.py` (verified: `summarize.py` and `art_3206.py` read `meta.json`/`state.json`/`launch.log`).

### 6.1 New file `scripts/bench/2x-vs-1x/display_mode.sh`

Refresh-rate pinning and verification. Interface:

```bash
display_mode.sh pin 120        # settings put system min_refresh_rate 120.0 ; peak_refresh_rate 120.0
display_mode.sh read          # JSON: {"min":"120.0","peak":"120.0","active_mode_id":1}
display_mode.sh verify <trace-dir>   # reads <trace-dir>/frames.csv, requires modal window_ms 8.333±0.05
```

- `pin` writes both settings, re-reads them, and exits non-zero if the read-back is not the requested
  value (today both are `null`, so the first thing the campaign does is set them).
- `verify` is the decisive check: modal `window_ms` from the run's own `frames.csv`; `8.333` → 120 Hz,
  `16.667` → 60 Hz, anything else → `mode_unknown`. Writes `display.json`:
  `{"pinned":true,"requested_hz":120,"read_back":{"min":"120.0","peak":"120.0"},"active_mode_id":1,"modal_window_ms":8.333,"mode_ok":true}`.
- Why not `mActiveModeId`: undocumented id ↔ Hz map on this build (`mActiveModeId=1` today).

### 6.2 Edit `scripts/bench/2x-vs-1x/bench_android.py` (three additive changes)

1. `frames_query()`'s per-frame SQL (`bench_android.py:229-232`, currently `select af.ts, … interval_ms,
   af.jank_type, …`) gains `round(af.dur / 1e6, 3) as window_ms` — one extra CSV column, first column
   after `ts` so the existing `DictReader` consumers keep working. This is the vsync period, taken from
   the device, which is what makes the refresh check trace-derived.
2. `cmd_window()` writes `display.json` (via the logic of 6.1) and copies `gate.json` into the run dir.
3. `meta.json` gains three keys: `"refresh"` (the `display.json` body), `"apk_sha256"`, and
   `"platform": "android"`. Additive; both existing consumers ignore unknown keys.

Breakage: none found. The `dur_ms`-only `frames.csv` in
`android-a54/soak/2x-soak-60m` is already an outlier and stays whatever it is; new runs get 7 columns.

### 6.3 New file `scripts/bench/2x-vs-1x/thermal_gate.sh`

```bash
thermal_gate.sh --ap 40.0 --status 1 --skin-max <baseline+1> --floor-s 480 --timeout-s 1500 \
                [--probe-only] --out <run-dir>/gate.json
```

`--probe-only` reads the sensors once and writes the JSON without waiting (used at F0 to record the
session baseline). Otherwise it polls every 20 s after `am force-stop`, evaluates the five conditions
of §4.2, writes
`gate.json` = `{"polled_at":…, "wait_s":…, "ap_c":…, "skin_c":…, "status":…, "battery_level":…,
"charging":false, "floor_s":480, "gate_ok":true|false}` and exits non-zero when `gate_ok` is false.
Replaces `run_paired.sh`'s `cool_down()` (`:172-180`), which waits on AP temperature alone
(`TARGET_TEMP` defaults to 40 at `:20`, the protocol documents 34) with no thermal-status, skin or
minimum-floor condition — and 40 °C is not reachable on this device today (idle AP 49.1 °C).

### 6.4 New file `scripts/bench/2x-vs-1x/cadence.py`

```bash
cadence.py <run-dir>            # writes <run-dir>/cadence.json, prints a one-line summary
cadence.py --check <frames.csv> --meta <meta.json>   # F2 self-test, exit 0 on pass
```

Core computation (one pass over `frames.csv`, no SQL):

```python
P = modal(round3(window_ms))                      # 8.333 at 120 Hz
periods = [interval_ms / P for interval_ms in intervals if interval_ms]
modal_periods = Counter(round(p) for p in periods).most_common(1)[0][0]
on_modal = sum(1 for p in periods if abs(p - modal_periods) <= 0.25) / len(periods)
hiccup = sum(1 for p in periods if modal_periods + 1 <= p < modal_periods + 2)
stall = [p for p in periods if p >= modal_periods + 2]
stall_time_frac = sum(stall) * P / sum(periods) * P     # share of window time in stalls
```

Writes `cadence.json`: `{"vsync_ms":8.333,"modal_periods":7,"on_modal_frac":0.882,
"hiccups_per_min":0.4,"stalls_per_min":2.4,"pct_window_in_stalls":0.041,
"stall_periods_hist":{"6":3,"7":640,"9":11},"jank_type_counts":{…},"frames":872,"span_s":29.9}`.
`--check` is F2: it must reproduce the p50/p99 of an existing run (within 0.5 / 1.0 ms).

### 6.5 New file `scripts/bench/2x-vs-1x/manifest.py`

Assembles and *validates* one run. `manifest.py run <run-dir>` reads `meta.json`, `cadence.json`,
`state.json`, `gate.json`, `display.json`, `launch.log`, `launch.json`, the APK hash and the tool
versions, and writes `run-manifest.json` plus a `verdict`. Schema (schema id `3206-run/1`):

```json
{"schema":"3206-run/1","label":"p1-2x-r3","arm":"P1","block":"B1","position":5,"day":2,
 "build":{"name":"2x","version_name":"2.0.0","version_code":2723,"apk":"apk-2x-e3b7aa102.apk","apk_sha256":"…","debuggable":false},
 "scene":{"id":"bafkrei…kxy","position":"0,-2","profile":3,"profile_changes":0},
 "settings":{"dynamic_graphics":"off","dynamic_skybox":"off","custom_skybox":"midday","evidence":["launch.log","settings.png"]},
 "display":{"pinned":true,"read_back":{"min":"120.0","peak":"120.0"},"active_mode_id":1,"modal_window_ms":8.333,"mode_ok":true},
 "thermal":{"start_ap_c":44.0,"start_skin_c":38.1,"start_status":1,"end_ap_c":47.9,"gate_wait_s":842,"gate_ok":true},
 "light":{"luma_crop":104.9,"luma_full":105.0,"band":[99.9,109.9],"in_band":true},
 "metrics":{"pct_window_in_stalls":0.041,"modal_periods":7,"on_modal_frac":0.882,"fps_avg":17.5,
            "interval_p99_ms":75.1,"cpu_mean_pct":144.2,"pss_steady_mb":1967},
 "tooling":{"adb":"…","trace_processor_sha256":"…","harness_commit":"…"},
 "flags":[],"verdict":"valid"}
```

`manifest.py init --out <campaign.json>` records the campaign-level facts once: device fingerprint,
APK sha256s, tool versions, the design decisions taken at F3/F5, the luma bands, the pinned scene id.
`manifest.py check <run-dir>` re-applies §8 and exits non-zero on `invalid` — this is the same code the
reviewer runs, so the checklist and the harness cannot disagree.

### 6.6 New file `scripts/bench/2x-vs-1x/campaign.sh`

Orchestrator. Subcommands `preflight F0..F6`, `p1`, `s1`, `s2`, `resume`. Behaviour:

- writes `campaign.pid` and refuses to start if another campaign pid is alive (rule: never edit
  scripts while they run; kill by explicit pid);
- enforces the ABBA sequence of §4.3 and refuses a run whose position does not match the plan;
- per run: `display_mode.sh verify` → `thermal_gate.sh` → `run_paired.sh <build> <label>` →
  `cadence.py` → `manifest.py run` → append a row to `campaign.csv`;
- stops the campaign when `manifest.py` marks a run invalid twice in a row for the same reason.

### 6.7 Edits to existing files

| file | change | reverse-compatibility |
|---|---|---|
| `run_paired.sh` | `cool_down()` (currently `run_paired.sh:172-180`) → `thermal_gate.sh`; call `display_mode.sh verify` before the window; call `manifest.py run` at the end; new env `ARM=P1|S1|S2`, `DAY`, `POSITION` unchanged | **behaviour change**: `TARGET_TEMP` (default 34) stops being the gate. An old invocation still runs but now passes through the three-condition gate; guards `[ -x thermal_gate.sh ]` keep it working if the new scripts are absent |
| `summarize.py` | `METRICS` gains the cadence keys (`pct_window_in_stalls`, `on_modal_frac`, `stalls_per_min`, `modal_periods`) read from `cadence.json` when present; per-run table gains `luma_crop`, `start_ap_c`, `gate_ok`, `verdict`; rows with `verdict != "valid"` are excluded and counted | additive, keys are optional |
| `bench_android.py` | §6.2 | additive |
| `frame_trace.cfg` | **no change.** 128 MB already covers a 30 s (and 45 s) window; `power/gpu_frequency` is not exposed on this device (no devfreq node), adding it would only produce a data-source failure | n/a |
| `art_3206.py` | report page reads `run-manifest.json` first and falls back to the current per-file reads | additive |

Nothing in this list touches client code, GDScript, or the engine.

---

## 7. Analysis plan

### 7.1 Unit and statistic

- **Unit of analysis = one run.** Never pool frames across runs (`summarize.py` already takes the run
  as the unit; keep that).
- Per arm, per metric: all raw values, `median [min–max]`, IQR. **Bootstrap CI of the difference in
  medians**, 10 000 resamples, seed `3206`, percentile 2.5/97.5. No mean ± SD, no bare p-value.
- Exact two-sided **Mann–Whitney U** only as a secondary, at n ≥ 4, with the exact distribution stated.
- Post-hoc **MDR** from the measured CV, always printed: `MDR = CV·√(15.7/n)`.

### 7.2 How the luma band enters the analysis

1. Runs with `verdict != "valid"` are out (they are also counted and listed).
2. Runs with `luma_out_of_band` are out of the primary comparison but stay in the stratified table.
3. Pairs must satisfy the §2.6 cross-build rule; unmatched pairs go to the stratified table.
4. **When the bands do not overlap** (the 2.x band and the 1.x band are disjoint intervals): the primary
   delta is **refused** and the report states, with both bands and their widths, that the two builds
   rendered different light and that no fps/cadence delta is publishable — that is claim 1c.1, and it
   is a legitimate, publishable outcome. The stratified table then shows fps vs `luma_crop` for every
   valid run of both builds with the S2 calibration slope, so a reader can see what the delta *would*
   have been at matched light, explicitly labelled as not a measurement of the build effect.

### 7.3 Table shapes for the report

**T1 — condition ledger (one row per run, appendix).**
`label | arm | block | position | build | versionName | profile | profile_changes | scene_id (8 chars) | position | luma_crop | band | in_band | start_ap_c | start_skin_c | start_status | gate_wait_s | min/peak refresh | modal_window_ms | avatars | verdict | flags`

**T2 — primary metric (the headline table).**
`arm | n valid / n run | 2.x median [min–max] | 1.x median [min–max] | ratio 2.x/1.x | bootstrap 95 % CI | pairs matched (n) | CV 2.x | CV 1.x | MDR @80 % | luma bands (2.x / 1.x) | light matched?`
Rows: `pct_window_in_stalls` (**primary**, bold), `on_modal_frac`, `stalls_per_min`, `modal_periods`.

**T3 — fps set and resources (secondary).**
One row per metric: `metric | 2.x median [min–max] | 1.x median [min–max] | ratio | CI | note`.
Metrics: `fps_avg`, `interval_p50_ms`, `interval_p95_ms`, `interval_p99_ms`, `fps_p1_low`,
`interval_max_ms`, `cpu_mean_pct`, `pss_steady_mb`, `pss_peak_mb`, `swap_pss_mb`,
`cold_launch_to_render_ready_ms`, `render_ready_t100_ms`.

**T4 — cadence distribution.**
`arm | P (ms) | modal_periods | % on cadence | hiccups/min | stalls/min | % window in stalls | 8-10-period frames | ≥ 11-period frames | jank_type top-3`
Plus the `stall_periods_hist` as a small bar table per arm.

**T5 — the refused list** (verbatim from §1c, each with the reason and the file that makes it
refusable).

**T6 — iOS appendix, separate section, separate header:**
`platform=iOS · instrument = 1 Hz HUD samples · no present-time evidence · descriptive only`.
`run | build | fps_median | fps_min–max | % seconds below target | luma_full | installed version`

### 7.4 Result sentence template

> In **{condition}** at pinned scene **{scene8}**, profile **{profile}**, light matched — `luma_crop`
> {L2x} vs {L1x} (bands ±5, matched = **{yes|no}**) and starting AP {T2x} vs {T1x} °C —
> **{build}** {metric} was **{x} {unit}** (median of {n} valid runs, min–max {a}–{b}, CV {cv} %)
> against **{other}** {y} {unit} ({a'}–{b'}, CV {cv'} %): ratio **{r}×**, bootstrap 95 % CI
> **{lo}–{hi}**, pairs matched {k}/{n}. Mann–Whitney exact two-sided p = {p}. At 80 % power this
> sample could have detected {mdr} %. {If the CI spans 1: "No difference was detected at this sample
> size; this is not evidence of equality (MDR {mdr} %)."}

---

## 8. Invalidation rules — a checklist a reviewer applies without re-running

Run `manifest.py check <run-dir>`; it must agree with this list. A run fails if **any** line fails.

1. **Files present:** `meta.json`, `frames.csv`, `cadence.json`, `state.json`, `gate.json`,
   `display.json`, `launch.log`, `screen.png`, `run-manifest.json`. Any missing → invalid.
2. **Build identity:** `version_name`/`version_code` and `apk_sha256` equal the arm's recorded values
   (1.x `1.16.1`/2688, 2.x `2.0.0`/2723); `debuggable` false.
3. **Profile:** exactly one `[RenderReady]` for the run's pid with `"profile":3`, and
   `Profile (downgrade|upgrade)` count = 0 for that pid inside `[t0,t1]`. (This alone voids
   `1x-hi-p1`, `1x-hi-p4`, `2x-hi-p1` in `gp-hi-pair`.)
4. **Scene:** `scene_id` = `bafkreihugp3rlcjryzkenaby4fb73gjdhvdxdkvu7uihhfqyxjqnhkkgxy`;
   `position` = the pilot's string.
5. **Frame count:** `frames ≥ 100` and `span_s ≥ 0.90 × window seconds` (the app was animating and the
   trace is not a ring-buffer remnant; the soak failure that produced `frames: "0"` is caught here).
6. **Refresh:** read-back `min`/`peak` = `120.0` and modal `window_ms` = 8.333 ± 0.05. A `16.667`
   fails unless the campaign file says the whole campaign ran at 60 Hz.
7. **Thermal gate:** `gate_ok` true, `start_ap_c ≤ 40.0`, `start_status ≤ 1`, `gate_wait_s ≥ 480`,
   `battery_level ≥ 20`, not charging.
8. **Light:** `luma_crop` inside the build's band; `luma_full` recorded (for continuity with older
   `luma_mean`). A run at `luma_crop < 55` is not the plain world (overlay: the tutorial measured
   luma 48) → invalid.
9. **Modal / foreground:** screenshot shows the world at the pinned pose, no dialog, no "Connection
   lost / RETRY" (the `gp-empty` failure).
10. **Contamination:** no `am start -W` `Status: timeout`; no other adb job over the window
    (`manifest.json.tooling` records the pre/post `adb devices` line); `pid_before` = 0 (the app was
    not already running when the trace started).
11. **Schedule (fallback design only):** `(start_unix % 7200) ≤ 5`.
12. **Verdict consistency:** `manifest.py check` reproduces the stored `verdict`; a run whose stored
    verdict is `valid` but which fails any line above is a tooling bug and blocks publication.

---

## 9. Risks and mitigations

| # | risk | detection signal | fallback |
|---|---|---|---|
| R1 | **The device is already hot**: today `Thermal Status 1`, AP `49.1 °C`, SKIN `41.8 °C` at idle, so "cold" does not exist and a 34 °C gate is fiction | `gate.json.wait_s` > 900 on the first two gates, or `start_ap_c` ≥ 42 despite 15 min of cooling | switch to the **throttled steady state** protocol (F5 fail path): 20 min soak, gate = soak-end AP + 1 °C, every number labelled "throttled steady state" |
| R2 | **Panel cannot be pinned to 120 Hz** (both settings `null`, `mActiveModeId` undocumented) | `display_mode.sh pin` read-back mismatch, or modal `window_ms` = 16.667 | pin to 60 Hz for the whole campaign and label it; if neither holds, drop all cadence claims and ship fps-only (F1) |
| R3 | **The two builds do not render the same light even in GP.** Existing evidence says they did not on 2026-11-03: `gp-hi-fixed-sky` 1.x 104.6–108.8 vs 2.x 72.6–77.6 | F4: `|L*(2x)−L*(1x)| > 5`, or disjoint bands in the measured arms | §2.7 phase-matched fallback at parcel 100,100; if that fails, refuse the delta (1c.1) and publish bands |
| R4 | **Settings taps silently fail** — 3 of 6 runs in `gp-hi-pair` loaded a profile other than the one on screen | `[RenderReady]` profile ≠ 3, or `Profile (downgrade|upgrade)` > 0 | `SKIP_SETTINGS=1` with the settings applied by hand and screenshots attached; re-run the run; abort the block after two failures |
| R5 | **Spawn position is randomised** inside the spawn-point bbox (`realm/scene_definition.rs:104-107`) | `position` differs from the pilot | keep the run only if `position` matches, else re-run; if it never matches, treat pose as a recorded factor and report the positions |
| R6 | **Remote avatars cannot be excluded** (no hide mechanism in either build — code investigation §6) | `avatars_present` from the screenshot; also `cpu_mean_pct` outlier | require arm counts within ±2 of the pilot; otherwise report the counts and refuse pooling across arms |
| R7 | **Tooling drift**: `trace_processor_shell` in `.bench-tmp` is a 10.7 KB wrapper, not a pinned binary; a newer processor can move a percentile with no code change | `manifest.json.tooling.trace_processor_sha256` differs between runs; F2 `--check` fails | compute all cadence metrics from `frames.csv` (raw per-frame rows) instead of SQL; re-derive the fps set from `frames.csv` too when the hash differs |
| R8 | **Install wipes AOT/compile state** (`cmd package compile`), so the first run after an install is systematically different from a later one | run 1 of a block deviates by more than 2 × the arm CV on the primary metric | compile after every install (F0) so the AOT state is identical across installs; if run 1 is still an outlier, keep the block at n=3 and record position as a covariate in `manifest.json` (positions are already balanced, so the effect cancels across the ABBA halves) |
| R9 | **Ambient/air-conditioning drift between the two days**, hitting one arm harder | arm `start_ap_c` medians differ by > 1.0 °C; day-1 vs day-2 `luma_crop` medians differ | re-run the unequal block (positions are known); report day as a covariate |
| R10 | **A run starts late** and, in the fallback design, lands on the wrong sun phase | `start_unix % 7200 > 5` | discard, wait for the next 7200 s cycle (2 h) — this is why the fallback is the fallback and not the primary |
| R11 | **Trace buffer / ring overflow** on a long window | `span_s` ≪ window seconds, `frames` small | chunk into 30 s windows (already the design); never a 10-min trace |
| R12 | **`screencap` inside the window perturbs the pipeline** | luma capture is taken *after* the trace stops (`bench_android.py` already does this) | keep it after; if a run's `luma` is required mid-window, take a second `screencap` 5 s after the window and record both |
| R13 | **iOS has no frame timeline** → someone pools it with Android | any table row without `platform=ios` label | §10 labelling rule; the acceptance review checks the label |
| R14 | **Battery/charging**: today level 13 %, discharging; plugging in during a run changes the thermal budget | `dumpsys battery` `ac/usb true`, or level < 20 at gate | hard blocker at F0; the run is invalid (rule 7) |

---

## 10. What the iPhone 16e contributes, and how it is labelled

**Contributes (three things, nothing more):**

1. **Functional coverage** (claim A-class, light-insensitive): the 2.x build launches, loads the pinned
   scene and renders it without crashing on `iPhone17,5` / iOS 26, with the same
   `[RenderReady]`/milestone lines the Android build emits on stdout (`xcrun devicectl device process
   launch --console`).
2. **A coarse draw-rate comparison, per build, in the same shape for both** — 1 Hz HUD/per-second
   samples, n = 3 runs per build, 30 s each: `fps_median`, `fps_min–max`, `% of seconds below target`.
   This is legitimate because the *instrument* is identical on both builds
   (`03-tooling.md` §5.2/§5.3), even though it is coarse.
3. **Memory footprint at load** (`[MemMonitor] footprint=` line, where present).

**Cannot contribute:** cadence, stalls, p99, present-time evidence, GPU time. The existing sample
(`ios-16e/gp-hi-ios/2x-ios-r1/meta.json`: `fps_median` 38.0, `fps_min` 37, `fps_max` 40, `luma_mean`
61.9, no `frames` block) shows exactly the ceiling.

**Labelling rules (all mandatory, checked in review):**

- Its own section, header literally: `iOS — 1 Hz draw-rate samples, no present-time evidence,
  descriptive only`. Never a row inside T2/T3/T4.
- Every row carries `platform=ios`, `instrument=HUD-1Hz`, `n=<runs>`, the installed `CFBundleVersion`,
  and the word *descriptive*.
- No bootstrap CI over pooled Android+iOS data, no ratio computed against an Android number, no
  verdict word (`PASS`/`FAIL`) in an iOS row.
- The build provenance difference must be printed next to it: 1.x is the published `1.16.0`; 2.x is a
  locally signed CI export (`ios-build.sh` + `devicectl install`), not an App Store build.
- The same sky caveat applies (`skybox_time` int → `try_to::<f64>()` → midnight on 2.x,
  `sky/skybox_time.rs:131-134`), so an iOS luma difference is not a light match.
- Cost/time is budgeted separately and is the first thing dropped after S2 (§3.4): 3 runs × 2 builds ×
  ~5 min + 2 installs ≈ 35 min.

---

## 11. Execution checklist (one screen, in the order a human follows it)

```bash
cd ~/Projects/decentraland/godot-explorer/sprint28-bench-2x-3206
export ANDROID_SERIAL=R5CW60624NE
export TRACE_PROCESSOR=$PWD/.bench-tmp/trace_processor_shell
S=scripts/bench/2x-vs-1x
CAMP=docs/bench/data/2x-vs-1x-3206/campaign-3206

# 0) physical, once — battery ≥ 80 %, UNPLUGGED, same Wi-Fi, DND, "Motion smoothness" ≠ Adaptive
adb -s $ANDROID_SERIAL shell 'svc power stayon true; settings put system screen_brightness_mode 0; settings put system screen_brightness 120'
mkdir -p $CAMP && python3 $S/manifest.py init --out $CAMP/campaign.json   # device fp, apk sha256, tool versions

# 1) F0 controls + F1 display pin (abort here if it fails)
$S/thermal_gate.sh --probe-only --out $CAMP/gate-baseline.json
$S/display_mode.sh pin 120 && $S/display_mode.sh read
# 2) F2 harness self-test on a known run — must pass before any measurement
python3 $S/cadence.py --check docs/bench/data/2x-vs-1x-3206/android-a54/gp-hi-fixed-sky/2x-sky-r1/frames.csv \
        --meta  docs/bench/data/2x-vs-1x-3206/android-a54/gp-hi-fixed-sky/2x-sky-r1/meta.json

# 3) preflight design gates (installs 2x, then 1x; each install needs the settings taps)
$S/campaign.sh preflight F3           # GP light stability, 2.x
$S/campaign.sh preflight F4           # light ledger, both builds -> decides §2.1 vs §2.7
$S/campaign.sh preflight F5 F6        # thermal reproducibility + settings reachability
$S/campaign.sh preflight --soak 20    # 20 min soak before the first measured block

# 4) P1, ABBA: 1x r1-r3 | install 2x | 2x r1-r6 | install 1x | 1x r4-r6
$S/campaign.sh p1 --day 1 --blocks A1,B1
$S/campaign.sh p1 --day 2 --blocks B2,A2 --resume

# 5) descriptives
$S/campaign.sh s1 --day 2            # shipped config, 45 s windows, 3 runs/build
$S/campaign.sh s2 --day 2            # sun-sensitivity control, 20 s windows, 3 runs/build

# 6) analysis (no device needed)
python3 $S/summarize.py --root docs/bench/data/2x-vs-1x-3206/android-a54 --manifest-only \
        --out $CAMP/summary.md
python3 $S/art_3206.py                      # page from run-manifest.json
```

Per-run, the orchestrator does exactly this and nothing else:

```bash
$S/display_mode.sh verify "$RUN"          # writes display.json, non-zero if modal window_ms != 8.333
$S/thermal_gate.sh --ap 40.0 --status 1 --skin-max "$SKIN_MAX" --floor-s 480 --timeout-s 1500 \
                   --out "$RUN/gate.json" || { echo "[abort] gate"; exit 2; }
WINDOW=30 SETTLE=25 ARM=P1 $S/run_paired.sh 2x p1-2x-r3          # existing script, gated
python3 $S/cadence.py "$RUN" >/dev/null && python3 $S/manifest.py run "$RUN"
python3 $S/manifest.py check "$RUN" || echo "$RUN invalid: do not leave it in the table"
```

Two hard rules while this runs: **do not edit a script that is executing** (kill by explicit pid from
`$CAMP/campaign.pid`), and **do not leave an invalid run directory in place** — `manifest.py check`
must exit 0 for every run that reaches a table.
