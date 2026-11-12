# 03 — Tooling & methodology reference: benchmarking a mobile 3D client (A54 5G / iPhone 16e)

Scope: how to make a 1.x-vs-2.x comparison of the Decentraland Godot client in a heavy 3D scene
repeatable and defensible on (a) Samsung Galaxy A54 5G — Exynos 1380, Mali-G68, Android 16 (API 36),
120 Hz panel — and (b) iPhone 16e, iOS 26. Current method: Perfetto frame timeline over 30 s,
`meminfo`, `dumpsys thermalservice`, mean screenshot luma as a "time of day" covariate, 3 runs per
condition with thermal cooldowns.

---

## PROVENANCE / VALIDATION LIMITATION (read before citing anything below)

This run had **no web search or source-fetch tool available** (only file read/write). Therefore:

- **No URL below was fetched or opened in this run.** URLs are stable official documentation entry
  points recalled from model memory; they are the right *places to look*, not verified page
  content. **Verify every URL before it goes into an external report.** Any URL that 404s should be
  found by name (doc title given) rather than copied.
- Claims are marked:
  - **[DOC]** — documented behaviour of the named platform/tool, as standard in official docs
    (unfetched; re-confirm before publication).
  - **[DERIVED]** — arithmetic or logic derived here from documented definitions; checkable by
    hand, no source needed beyond the definition used.
  - **[INFERENCE]** — my engineering judgement, not stated by any source.
- Statistics in §6 are standard results (no web source needed; math shown inline).
- Sysfs paths, vendor counter names and Samsung-specific thermal zone names are **device-dependent
  and unverified**. §7 requires you to *enumerate* them on the actual device and paste the output
  into the report — never assume.

Net effect: treat this document as a **methodology spec + command catalogue**, and spend one hour
verifying the marked URLs and dumping the device state before the campaign starts.

---

## 1. Frame timing on Android: what to measure and why "fps" is the wrong headline

### 1.1 The four instruments and what each actually sees

| Instrument | Granularity | Window | Sees | Blind to |
|---|---|---|---|---|
| Perfetto **Frame Timeline** (`android.surfaceflinger.frametimeline` data source) | per frame, `SurfaceFrameToken` + `DisplayFrameToken`, expected vs actual present, `jank_type` | arbitrary (seconds–minutes) | full pipeline: app submit → SF composite → present; jank cause taxonomy | GPU time inside the app, unless GPU counters also recorded |
| `dumpsys SurfaceFlinger --latency <layer>` | per frame, 3 timestamps (desired present, actual present, frame ready) | **fixed 127-frame ring buffer** [DOC] | present timestamps for one layer | anything older than ~1 s at 120 Hz |
| `dumpsys gfxinfo <pkg> framestats` | per frame, 20+ phase timestamps (input→animation→measure/layout/draw→sync→issue-draw→swap) plus `Flags` | **ring buffer (~120 frames)** [DOC] | app-side UI-thread phase breakdown, `Flags` bit 0 = frame missed deadline | native/render-thread games largely bypass this path; short history |
| `Choreographer` (in-app callback, or `atrace` `gfx` category) | per vsync callback | arbitrary if you log it | "did I get a vsync callback, when" | whether the frame reached the display |

Key structural fact: **all four Android-side tools measure the *submission/present* pipeline. None of
them measures GPU execution time.** On a Mali-G68 in a heavy 3D scene the interesting number
(GPU busy time per frame) is invisible to Frame Timeline. [DOC for the split; INFERENCE for
"interesting number"]

- Android developer, *Slow rendering / frame rendering* (the "16 ms" budget, jank definition): https://developer.android.com/topic/performance/vitals/render
- Android developer, *Inspect GPU rendering speed and overdraw* (`dumpsys gfxinfo`, `Profile HWUI rendering`): https://developer.android.com/topic/performance/rendering/inspect-gpu-rendering
- Perfetto docs home: https://perfetto.dev/docs/
- Perfetto *Frame Timeline* data source: https://perfetto.dev/docs/data-sources/frametimeline
- Perfetto trace processor / SQL: https://perfetto.dev/docs/analysis/trace-processor
- Android developer, *Overview of system tracing*: https://developer.android.com/topic/performance/tracing
- Android `Choreographer` reference: https://developer.android.com/reference/android/view/Choreographer
- Godot performance monitors (`Performance.TIME_FPS`, `RENDER_TOTAL_DRAW_CALLS_IN_FRAME`): https://docs.godotengine.org/en/stable/classes/class_performance.html

### 1.2 The vsync-cap trap (this is the central measurement problem of the campaign)

At 120 Hz the vsync period is 8.333 ms. If the app cannot hit 120 fps and lands at ~30 fps, frame
intervals are **quantised to multiples of 8.333 ms** (4 periods = 33.33 ms). [DERIVED from the
definition of vsync-locked presentation]

Consequences, all decision-relevant:

1. **Mean fps is a coarse, lossy statistic.** 30 fps == 33.33 ms only if every frame is exactly 4
   periods. A run with 10 % of frames at 3 periods and 90 % at 4 periods reports ~31.6 fps and looks
   "fine", while the p99 of frame intervals is 25 ms in one run and 41.7 ms in another (a doubled
   interval = 8 periods) — the *distribution shape*, which is what a player feels, is invisible in
   the mean. [DERIVED]
2. **p95/p99 of frame intervals become near-degenerate under a hard cap.** With ≥95 % of frames at
   exactly 4 periods, p95 is 33.33 ms *by construction* and carries no information; only p99 (or
   higher) starts to separate "occasional 8-period stall" from "clean 4-period cadence". Reporting
   "p95 = 33.3 ms, p99 = 33.3 ms" for two builds that feel different is the failure mode to avoid.
   [DERIVED]
3. **Therefore the reportable metric is not an fps percentile but a *cadence-quality* metric:**
   fraction of display frames delivered on the expected cadence, plus the *number and size of the
   missed-vsync tail*. [DERIVED / recommendation]
4. **Missed-vsync tail, detected properly:** count frames whose interval exceeds the modal interval
   (the mode, e.g. 33.33 ms) by ≥1 vsync period, and classify by how many periods were skipped
   (1-skip = "hiccup", ≥2-skip = "stall"). This is exactly what Frame Timeline's `jank_type`
   (`App deadline missed`, `SurfaceFlinger deadline missed`, `Buffer stuffing`, `Dropped Frame`,
   …) encodes — prefer the platform's own classification over your own re-derivation, and *report
   both* so the two agree. [DOC for the jank type names; INFERENCE for "report both"]
5. **A 30 s window at 30 fps = ~900 frames.** That is a *small* sample for p99: the 99th percentile
   of 900 samples rests on ~9 observations, and p99.9 would rest on <1. Do not report p99.9 from
   30 s windows; and state the sample size next to every percentile. [DERIVED]

### 1.3 Minimal Perfetto recipe for this campaign

Frame Timeline needs the named data source, which the `-t`/category shorthand does **not** include;
use a config file.

```bash
# /tmp/frames.cfg  (Perfetto trace config, textproto)
buffers { size_kb: 131072  fill_policy: RING_BUFFER }
data_sources { config { name: "android.surfaceflinger.frametimeline" } }
data_sources { config { name: "linux.ftrace"
  ftrace_config {
    ftrace_events: "sched/sched_switch"
    ftrace_events: "sched/sched_wakeup"
    ftrace_events: "power/cpu_frequency"
    ftrace_events: "power/cpu_idle"
    ftrace_events: "power/gpu_frequency"     # vendor-dependent availability
    ftrace_events: "cpufreq_interactive/cpufreq_interactive"  # if scaling governor uses it
    atrace_categories: "gfx"
    atrace_categories: "view"
    atrace_categories: "sched"
    atrace_categories: "freq"
    atrace_apps: "<your.package>"
    buffer_size_kb: 32768
  } } }
data_sources { config { name: "linux.process_stats" } }
data_sources { config { name: "android.power"  android_power_config { battery_poll_ms: 1000 } } }
duration_ms: 35000          # > measurement window: 5 s of run-up/wind-down context
flush_period_ms: 5000
```

```bash
adb push /tmp/frames.cfg /data/misc/perfetto-configs/frames.cfg
adb shell perfetto --txt -c /data/misc/perfetto-configs/frames.cfg \
  -o /data/misc/perfetto-traces/frames.pftrace
adb pull /data/misc/perfetto-traces/frames.pftrace
# or, from the Perfetto repo: tools/record_android_trace -c /tmp/frames.cfg -o frames.pftrace
```

Analysis (trace_processor; table names vary by trace-processor version — check
`SELECT name FROM sqlite_master WHERE type='table' AND name LIKE '%frame%'` first):

```sql
-- modern name (v4x+):
SELECT upid, ts, dur, present_type, jank_type, jank_tag, on_time_finish, gpu_composition
FROM android_frames
WHERE ts BETWEEN :t0 AND :t1;
-- older names: actual_frame_timeline_slice / expected_frame_timeline_slice

-- modal interval + skipped-period histogram
WITH f AS (
  SELECT upid, ts, ts - LAG(ts) OVER (PARTITION BY upid ORDER BY ts) AS d
  FROM android_frames WHERE ts BETWEEN :t0 AND :t1)
SELECT round(d/1e6, 2) AS interval_ms, count(*) n FROM f GROUP BY 1 ORDER BY n DESC;
```

Report from this: `n_frames`, modal interval, `% frames == modal`, histogram of intervals in
vsync-period units, count of ≥2-period intervals, jank_type counts from Frame Timeline, plus the
**share of the wall-clock window spent in ≥2-period stalls**.

### 1.4 Recommendations

1. **Headline metric = cadence quality, not fps.** Per condition report: modal frame interval in
   vsync periods, `% frames on modal cadence`, `# stalls ≥2 periods per 30 s`, `% window in stalls`,
   and Frame Timeline `jank_type` breakdown. Secondary: mean fps. Never report fps alone. [DERIVED]
2. **Always pair Frame Timeline with a GPU-busy signal** — Perfetto `power/gpu_frequency` +
   `gpu_counter`/`gpu_work_period` if the Exynos kernel exposes them, otherwise in-app GPU timestamp
   logging. Without it you are measuring the compositor, not the renderer. [INFERENCE]
3. **Do not use `dumpsys gfxinfo` or `SurfaceFlinger --latency` as the primary source** for 30 s
   windows: their ring buffers hold ~1 s at 120 Hz. Use them only for cheap smoke checks and for the
   `Flags`/`framestats` phase split when a jank spike needs attributing. [DOC for buffer size]
4. **Log the sample size and the display mode next to every percentile** (see §3 for the mode), and
   cap the reported percentiles at p99 for a 30 s window.

---

## 2. Thermal control: how to gate cooldowns and what to record

### 2.1 Three unrelated signals, three different meanings

- **`dumpsys thermalservice`** — the ThermalService HAL view: current reported temperatures per
  thermal zone, the *throttling status* (`Thermal Status: 0..6`), and the cooling devices /
  mitigation state. This is the **policy output**: it already encodes the OEM's rules over many
  sensors. [DOC — Android thermal service / thermal HAL]
- **`PowerManager.getCurrentThermalStatus()`** (API 29+) — same status as an app-readable enum:
  `THERMAL_STATUS_NONE(0)`, `LIGHT(1)`, `MODERATE(2)`, `SEVERE(3)`, `CRITICAL(4)`,
  `EMERGENCY(5)`, `SHUTDOWN(6)`, plus `OnThermalStatusChangedListener` for in-app sampling at events,
  not just poll. [DOC]
- **`/sys/class/thermal/thermal_zone*/temp` + `type`** — raw sensor millidegrees. Names are
  OEM/SoC-specific (`gpu`, `big0`, `lit0`, `BIG`, `LITTLE`, `G3D`, `ap`, `battery`, `skin`, `xo_therm`,
  `charger`, …). **Do not hardcode names**: enumerate with
  `adb shell 'for z in /sys/class/thermal/thermal_zone*; do printf "%s " "$z"; cat $z/type; cat $z/temp; done'`
  and paste the listing into the report. [DOC for sysfs interface; INFERENCE for "enumerate"]

### 2.2 Skin vs junction — which one governs your benchmark

- Junction/die temperature (Tj) is what physics throttles on, but production phones expose it only
  indirectly, through OEM-smoothed zones and the thermal HAL. You generally cannot read Tj.
  [INFERENCE]
- **Skin temperature is the variable the DTM/thermal policy actually acts on**, because the
  constraint on a phone is touchability, not silicon survival; that is why mitigation ladders
  (frame-rate caps, GPU clock caps) kick in long before any junction limit. [DOC-ish: Android
  thermal mitigation / DTM model; INFERENCE for the "therefore"]
- **Practical consequence:** use the *policy output* (`thermal status`) as the primary gate — it is
  the thing that actually changes the numbers you are measuring — and treat skin temperature as the
  slow-moving diagnostic that predicts when the next status step will arrive.

### 2.3 What a defensible cooldown criterion looks like

Two failure modes: (a) cooling "until it feels cool" (not reproducible, and skin lags by minutes);
(b) fixed 5-minute waits (unmeasured, and a 20 min run needs far more than 5 min to return to
baseline).

A defensible criterion is **conjunctive and measured**:

1. `PowerManager.getCurrentThermalStatus() == THERMAL_STATUS_NONE` (or `dumpsys thermalservice`
   `Thermal Status: 0`) continuously for the last N minutes;
2. the designated skin/battery zone within **Δ ≤ 1 °C of the session-start baseline** recorded at
   t=0 of the campaign (same ambient, same starting state);
3. a **minimum soak never shorter than X** (see below) regardless of how cool it reads;
4. ambient temperature and panel brightness unchanged (§3), since both drive the same heat path.

Record `dumpsys thermalservice` + the zone dump **immediately before and immediately after each of
the 3 runs**, not just once per condition.

### 2.4 How much soak an Exynos/Mali device needs

There is **no official number** in the Android docs for this, and I found none to cite — treat any
figure as empirical. What *is* documented is the shape of the problem: modern Android thermal
mitigation is a *closed loop with hysteresis and multiple control points*, so the device settles into
a throttled steady state over minutes and returns over minutes; and the first runs in a session are
systematically faster than later ones ("cold vs warm") even with no throttling, because of
governor/boost state, page caches, shader caches and (on some devices) charging heat. [DOC for the
closed-loop/hysteresis model; INFERENCE for the specific durations]

Concrete plan, to be *validated* by your own data rather than believed:

- **Soak before measurement: 20–30 min of continuous load in the target scene**, then start the
  measured runs. Rationale: you want steady-state, not "burst". Evidence for whether 20 min was
  enough = the drift of the *pre-run* thermal zone temperature and status across the last 10 min of
  soak; if still rising, extend. [INFERENCE]
- **Cooldown between runs: 10–15 min minimum**, gated as in §2.3. If thermal status is still ≥LIGHT
  at 15 min, that is data: extend and log it. [INFERENCE]
- If the campaign budget cannot afford this, the honest alternative is to **measure the throttled
  steady state only** (long soak, never measure a cold device) and to say so explicitly — a
  throttled-steady-state comparison is more representative of a real player session than a cold
  device anyway. [INFERENCE]

### 2.5 Recommendations

1. Gate every run on `thermal status == NONE` **and** skin/battery zone within 1 °C of session
   baseline, with a floor of 10–15 min cooldown; log the pre/post thermal snapshot for all 3 runs.
2. Adopt **cold-start exclusion**: the first measured run of every session is a discarded
   warm-up (do not use run 1 of 3 as data unless it passed the gate by the same margin).
3. Paste the full `thermal_zone* type/temp` listing into the report; never refer to "the CPU temp".
4. Add a *positive control*: one deliberately long run (e.g. 10 min) to demonstrate the device does
   throttle in your scene and to show where the cliff is — this is what makes the short-window
   numbers credible.

Sources: Android `PowerManager` (thermal status constants, `OnThermalStatusChangedListener`):
https://developer.android.com/reference/android/os/PowerManager —
Android thermal mitigation overview / Adaptive Thermal Management model:
https://source.android.com/docs/core/power/thermal-mitigation —
Android Power/thermal HAL (`dumpsys thermalservice` source of truth):
https://source.android.com/docs/core/power —
Android Dynamic Performance Framework (ADPF) thermal API, the *supported* app-facing way to read
headroom and thermal status: https://developer.android.com/games/optimize/adpf/thermal —
ADPF CPU/GPU headroom: https://developer.android.com/games/optimize/adpf/cpu-headroom ,
https://developer.android.com/games/optimize/adpf/gpu-headroom

---

## 3. Frequency and scheduling confounders

### 3.1 The confounders that actually bite

1. **CPU DVFS residency, not just current freq.** `scaling_cur_freq` is one instantaneous sample; the
   distribution of frequencies over the 30 s window is the confounder. Read residency from
   `/sys/devices/system/cpu/cpuN/cpufreq/stats/time_in_state` (per-policy, monotonic counters — take
   two snapshots around the window and subtract) and/or Perfetto `power/cpu_frequency` counters.
   [DOC for the knobs; INFERENCE for "residency, not snapshot"]
2. **GPU devfreq / Mali clock.** Mali (kbase) commonly exposes `/sys/kernel/gpu/gpu_clock`
   (current Hz), `/sys/kernel/gpu/gpu_freq` (available), `/sys/kernel/gpu/gpu_load` and
   `/sys/kernel/gpu/gpu_utilisation`; generic devices use `/sys/class/devfreq/*/cur_freq`. Names are
   kernel/vendor dependent — enumerate with
   `adb shell 'ls -l /sys/kernel/gpu 2>/dev/null; ls /sys/class/devfreq'`. In Perfetto: `gpu_counter`
   / `gpu_work_period` tables exist but require the platform to emit them. [INFERENCE on names;
   DOC on Perfetto tables]
3. **big.LITTLE placement.** Exynos 1380 is 4×Cortex-A78 + 4×Cortex-A55, so a thread that migrates
   from A78 to A55 for one frame produces a stall that has nothing to do with the client version.
   Detect from `sched/sched_switch` in the trace: per-thread CPU histogram, and specifically **the
   render thread's** affinity over time. [DERIVED]
4. **Refresh-rate / frame-rate adaptation.** This is the biggest single confounder on a 120 Hz
   panel and it is silent. If adaptive refresh (Samsung "Motion smoothness: Adaptive") or per-app
   frame-rate hints (Android 15+ `Surface.setFrameRate()`) change the panel between 120/60/30/24 Hz
   mid-window, Frame Timeline's *expected* presentation time changes and your interval histogram
   becomes bimodal for reasons unrelated to performance. [DOC for the mechanisms]
5. **Governor/boost state and cgroup placement.** `top-app` cpuset/uclamp differs from `background`;
   an app that lost foreground focus benchmarks differently. Guarantee foreground + screen on.
   [DOC for cgroups/uclamp existence]

### 3.2 Pinning without root — what is actually possible

- **You cannot set affinity of another app's threads from `adb shell` without root.** `taskset` in
  toybox can only be applied to processes you own/are permitted to; the app's threads belong to its
  own uid. [DOC-ish for Linux permission model; INFERENCE for Android specifics]
- **You *can* pin from inside the app**, for its own threads: `sched_setaffinity(2)` /
  `pthread_setaffinity_np` via JNI (or from Rust with `libc::sched_setaffinity`). Caveats: Android's
  cpuset controller may override it for the process cgroup, and pinning games to the little cluster
  produces numbers nobody should ship. [DOC-ish; INFERENCE]
- **Practical alternative: do not pin, but *observe and report* placement.** Record per-thread CPU
  residency from the trace and *discard* runs whose placement deviates from the modal placement.
  For a version-vs-version comparison this is more honest than pinning, because pinning changes the
  product being measured. [INFERENCE]
- If you must pin for a *microbenchmark* separately from the product comparison, run it as a
  standalone native binary on the device (`adb push`, `adb shell taskset -c 4-7 ./bench`) where you
  own the process and pinning is legitimate. [INFERENCE]

### 3.3 Record-per-run block (so a run can be discarded post hoc)

```bash
PKG=com.<pkg>
adb shell dumpsys display | grep -Ei 'mActiveModeId|refreshRate|DisplayMode'   # active mode
adb shell settings get system peak_refresh_rate; adb shell settings get system min_refresh_rate
adb shell cat /sys/class/display/panel0/... 2>/dev/null                        # if present
adb shell dumpsys power | grep -Ei 'mWakefulness|mScreenOn|low_power|thermal'
adb shell dumpsys thermalservice
adb shell 'for z in /sys/class/thermal/thermal_zone*; do printf "%s " "$z"; cat $z/type; cat $z/temp; done'
adb shell 'for c in /sys/devices/system/cpu/cpu*/cpufreq; do echo "$c"; cat $c/scaling_cur_freq; done'
adb shell 'grep -H . /sys/devices/system/cpu/cpu*/cpufreq/stats/time_in_state 2>/dev/null'
adb shell 'cat /sys/kernel/gpu/gpu_clock /sys/kernel/gpu/gpu_load 2>/dev/null; cat /sys/class/devfreq/*/cur_freq 2>/dev/null'
adb shell dumpsys meminfo $PKG
adb shell getprop ro.build.fingerprint; adb shell getprop ro.product.model
adb shell settings get global low_power; adb shell settings get system screen_brightness
```

Controls to set before the campaign (and to *not* change during it): auto-brightness off + fixed
brightness (`settings put system screen_brightness_mode 0`, `screen_brightness <fixed>`), adaptive
refresh off or pinned (`settings put system min_refresh_rate 120.0`, `peak_refresh_rate 120.0` —
verify it took with the `dumpsys display` active-mode read), battery saver off, airplane mode on or
Wi-Fi pinned to one AP, auto-rotate off, screen-on (`svc power stayon true`), Do Not Disturb on,
system animations irrelevant for a 3D scene but log `animator_duration_scale` anyway, and
`cmd package compile -m speed-profile -f $PKG` once so JIT/AOT state is identical across runs.
[INFERENCE + DOC for the settings names]

### 3.4 Recommendations

1. **Pin the panel**: disable adaptive refresh and verify with `dumpsys display` that the active mode
   is the same in every run; treat any run with a mode change as invalid.
2. Record the §3.3 block **before and after every run**, plus a 1 Hz sampler during the window
   (`cpu*/cpufreq/scaling_cur_freq`, GPU clock/load, thermal zones) — 1 Hz is cheap and turns a
   discarded run into a diagnosable one.
3. Do **not** pin threads for the 1.x-vs-2.x comparison. Instead report per-thread big/little
   residency from `sched_switch` and discard runs outside the modal placement pattern.
4. Discard-rule, written down before data collection: *any run with a display-mode change, thermal
   status ≠ NONE at start, GPU clock < X, a foreground-loss event, or a >2× deviation in window
   frame count is invalid and re-run.* State the discard rule and the number discarded in the report.

---

## 4. Mean screen luminance as a covariate for "scene brightness"

### 4.1 Is it legitimate? Only as a *manipulation check*, not as a covariate

Mean luma of a screencap is a real, reproducible measurement, and it does correlate with scene
irradiance — but it is a **compressed, non-monotone-in-the-useful-range** function of it, so using it
as a regression covariate across versions risks adjusting away part of the very difference you are
measuring. [INFERENCE]

Documented failure modes, each of which breaks the covariance assumption:

1. **Tone mapping and exposure adaptation.** A rendering pipeline maps HDR scene radiance through
   filmic/ACES-style tonemapping and (if enabled) eye adaptation; the output is deliberately
   compressive at the top end. Doubling scene light changes output luma by far less than 2×, and
   different versions may use different tonemap curves — so luma difference ≠ light difference.
   [DOC for tonemapping/auto-exposure existing in 3D engines generally; INFERENCE for Godot's
   specific configuration in this repo]
2. **Skybox dominance.** A bright sky occupies a large screen fraction and is nearly constant
   radiance; it can dominate mean luma while contributing almost nothing to fragment shading cost
   (it is often the cheapest thing drawn). Luma then tracks camera pitch, not scene complexity.
   [INFERENCE]
3. **HUD overlays and UI chrome.** The 2.x React Native shell and the 1.x HUD have different UI
   surfaces (different area, different colours, possibly translucent panels). A cross-version luma
   difference therefore contains a UI difference — a systematic, version-correlated bias. This is
   the most dangerous failure mode for *this* campaign. [INFERENCE]
4. **Letterboxing / aspect and safe-area differences.** Black bars or different viewport aspect
   between the two shells change the fraction of pixels that are scene pixels, so mean-over-all-pixels
   is not mean-over-scene. Crop to the render viewport before averaging. [INFERENCE]
5. **Colour-space and encoding ambiguity.** "Mean luma" must be defined: is it the arithmetic mean of
   the decoded **sRGB-encoded** values (what a screenshot gives you), or of linearised light? They
   differ substantially, and the luma weights (Rec.709: 0.2126 R + 0.7152 G + 0.0722 B) must be
   stated. Also: `screencap` output may be in the display's colour space with wide-gamut/HLG
   metadata — normalise explicitly. [DOC for the coefficients; INFERENCE for the risk here]
6. **The measurement perturbs the experiment.** `adb exec-out screencap` costs tens of ms, forces a
   composition/readback, and on some devices perturbs the frame pipeline — so capturing *during* the
   30 s window contaminates it. [INFERENCE]

### 4.2 A better proxy without engine instrumentation

Do not try to measure "how much light is in the scene". **Make it a controlled factor instead of a
covariate** — same scene state at the same point in the run, in both builds:

1. **Deterministic camera + deterministic scene clock.** Drive the camera along a recorded,
   timestamped path, and start measurement at a fixed scene time (e.g. `t_scene = 12.0 s`), so both
   builds render *the same* pixels at the same wall-clock offset. Then brightness is held constant
   by construction and needs no covariate at all. [INFERENCE, and the strongest recommendation here]
2. **If you need a brightness metric anyway, use two cheaper and better ones:**
   - *Pixel-fraction by luminance band* on a viewport-cropped screenshot (crop to the render
     surface, exclude HUD rectangles by layout coordinates): `% pixels > 200`, `% < 16` — far more
     informative than a mean, and insensitive to tone-map compression at the midpoint. [INFERENCE]
   - *GPU busy time* from the trace as the dependent measure, with the scene state pinned by (1),
     which removes the need for a brightness proxy entirely. [INFERENCE]
3. **Replace "time of day" with "scene state id".** If the campaign means "the virtual world's day
   cycle", record the scene's in-world clock from the client logs (or fix it), not a pixel mean.
   [INFERENCE]

### 4.3 Recommendations

1. **Stop using mean luma as a covariate across versions** — with different HUD/UI chrome it is
   confounded with the treatment. Keep it only as a same-build, same-version *manipulation check*
   that the scene rendered the same content, and only from screenshots taken **outside** the
   measurement window.
2. **Pin scene state**: fixed scene clock, scripted camera path, same avatar set, measurement start
   at a fixed scene timestamp. Report the scene-state id in every run row.
3. If a luminance number is required, define it precisely (sRGB-encoded, Rec.709 weights, viewport-
   cropped, HUD excluded) and report the pixel-fraction bands rather than a scalar mean.
4. Capture luma from a **separate short capture at a frozen camera pose**, not from the perf window.

Sources (for the documented parts): Godot environment/tonemap & auto-exposure:
https://docs.godotengine.org/en/stable/classes/class_environment.html —
Godot `WorldEnvironment`/`Camera3D` (for fixing exposure deterministically):
https://docs.godotengine.org/en/stable/classes/class_worldenvironment.html —
Android `screencap`/`adb` shell reference: https://developer.android.com/tools/adb —
Rec.709 luma coefficients (ITU-R BT.709) — standard, no URL required.

---

## 5. iOS: frame pacing and GPU timing without Instruments attached

### 5.1 Option-by-option, with the hard limits

| Method | Machine-readable? | Works without `xctrace`/Instruments? | What you can actually claim |
|---|---|---|---|
| **`os_signpost` / `OSSignposter`** (`os` framework) | Yes, as signpost intervals in a log/serialized trace — but *only* in Instruments or a custom consumer | Instruments is the supported consumer; `log stream --signpost` can surface signpost activity on a tethered device [DOC-ish] | Interval boundaries you defined, at kernel-timestamp resolution. Requires a build with signposts; gives you "how long did my frame block take", not "when was it presented" |
| **MetricKit** (`MXMetricManager`) | Yes, JSON payloads | Yes (in-app, `didReceive(_ payloads:)`) — **but payloads are delivered at most ~once per day** [DOC] | Aggregate `MXAnimationMetric` (scroll hitch time ratio), `MXAppResponsivenessMetric` (hang rate), CPU/GPU/memory. **24 h granularity ⇒ useless for a 30 s A/B.** `MXSignpostMetric` bundles your signposts but inherits the daily cadence |
| **`CADisplayLink`** | Yes (in-app timestamps) | Yes | Vsync-aligned callback times (`timestamp`, `targetTimestamp`) ⇒ genuine display-cadence samples; `preferredFrameRateRange` (iOS 15+) tells you the requested rate. **Does not** measure GPU time or whether the frame was presented on time |
| **Metal Performance HUD / developer HUD** | No — visual overlay only; you'd OCR screenshots | Yes (dev builds, via scheme env var `MTL_HUD_ENABLED=1` or the Metal developer tools setting) | A second-granularity fps + GPU-time readout as *pixels*. Adds its own overlay cost; not citable as data |
| **Engine HUD read from the accessibility tree** | Yes if the HUD is a real a11y element | Yes (XCUITest / `XCUIApplication().debugDescription`) | Depends entirely on whether the readout is an accessible element. Godot's debug HUD is drawn into the canvas, **not** an accessibility node ⇒ not readable for 1.x. The 2.x RN shell *can* expose a JS-side fps `Text` with `accessibilityLabel` ⇒ readable. [INFERENCE on Godot a11y; the split is the actionable part] |

Notes that matter for the plan:

- `xcrun xctrace record --template 'Time Profiler'` is not "Instruments attached" in the interactive
  sense but is still the Instruments stack and still requires a Mac + tether. If a Mac is available
  *at all*, `xctrace` with the **Metal System Trace** template is by far the best iOS frame-pacing
  source; the "no Instruments" constraint is really "no Mac in the loop".
- Apple's `os_signpost` guidance and the Instruments-side workflow: *Recording Performance Data* /
  *Measuring Performance Using Signposts*. Apple developer docs (unfetched):
  https://developer.apple.com/documentation/os/ossignposter ,
  https://developer.apple.com/documentation/xcode/measuring-performance-using-signposts
- MetricKit: https://developer.apple.com/documentation/metrickit ,
  `MXAnimationMetric`: https://developer.apple.com/documentation/metrickit/mxanimationmetric
- `CADisplayLink`: https://developer.apple.com/documentation/quartzcore/cadisplaylink
- Metal developer tools / HUD: https://developer.apple.com/documentation/metal/developer_tools
- Godot iOS export (embedding in the RN host): https://docs.godotengine.org/en/stable/tutorials/export/exporting_for_ios.html
- `xcrun devicectl` (device process launch, console, env vars): `xcrun devicectl --help`
  — https://developer.apple.com/documentation/xcode/device-management (see `devicectl`)

### 5.2 What a 1 Hz sample of an fps readout licenses you to claim

Definitions first, because the claim depends on them. Godot's `Engine.get_frames_per_second()` is
"frames drawn in the last second" (a 1-second trailing counter) [DOC]. If you sample a
*trailing-1-s counter* at 1 Hz with no phase overlap, each sample is the **mean fps over that
second** — non-overlapping 1 s windows. From those you may legitimately claim:

- the **mean fps over the window** (mean of the per-second means; identical to total frames / total
  seconds);
- the **distribution of 1-second means**, and hence "**% of seconds below target**" — an
  Android-vitals-like "slow frame rate time" statement;
- a **variance/noise** estimate, and its trend (e.g. drift under thermal load).

You may **not** claim:

- any **p99/p99.9 frame time** or "we never dropped below 30 fps for longer than X ms" — sub-second
  events are invisible (a 400 ms freeze inside one window moves that second's fps by a few percent
  and shows up nowhere else);
- anything about **vsync alignment** or frame *pacing* (a steady 30 fps and a 60/30 judder pattern
  averaged to 30 fps are indistinguishable);
- precision finer than the readout's own quantisation (Godot's HUD is integer fps ⇒ ±0.5 fps floor;
  1 fps at 60 fps = 1.7 %);
- that these are **presented** frames (they're drawn frames).

[All above DERIVED from the trailing-counter definition + sampling theory]

### 5.3 Recommendations

1. **Designate iOS as a secondary, coarser dataset and say so in the report.** Primary iOS metric:
   per-second means from an in-app 1 Hz `CADisplayLink` + engine fps log written to a file, both
   builds. Report mean, % of seconds below target, and the run-to-run spread — no percentiles. [DERIVED]
2. **Emit an iOS log line per second** (`fps`, `frame_time_ms`, `draw_calls`, `thermal/state`) from
   the Rust side for both builds — this is the only source that exists on both platforms and is
   therefore the backbone of the cross-version claim. [INFERENCE]
3. For 2.x, additionally expose the fps readout as an accessible RN element so an `XCUITest`
   (`debugDescription` / `XCUIElement.value`) can harvest it without touching the GPU; do not attempt
   the same for 1.x — Godot's HUD is not in the a11y tree. [INFERENCE]
4. Try **one** session with `xcrun xctrace record --template 'Metal System Trace'` if any Mac is
   reachable; if that succeeds, treat iOS as a first-class dataset and state the trace template +
   device UDID + iOS build in the report. If not, declare the iOS numbers as "1 Hz sampled,
   draw-rate only, no present-time evidence".

---

## 6. Statistical hygiene for a small campaign

### 6.1 How many runs, given 10–20 % run-to-run spread

Use the standard two-sample normal-approximation sample-size formula (α=0.05 two-sided, power=80 %,
per arm):

```
n ≈ 2 · (z_{1-α/2} + z_{1-β})² · (CV / Δ_rel)²      with (1.96+0.84)² = 7.85
```

[DERIVED — arithmetic]

| Assumed run-to-run CV | Smallest Δ we can detect with **n=3/arm** | n needed for Δ=10 % | n for Δ=20 % | n for Δ=50 % |
|---|---|---|---|---|
| 10 % | ~23 % | 16 | 4 | 2 |
| **15 %** | **~35 %** | **36** | **9** | **3** |
| 20 % | ~46 % | 63 | 16 | 6 |

Reading: **with 3 runs per condition you can only defend differences larger than roughly 35 %** when
spread is 15 %. A claimed 10 % win at n=3 is not supportable. This is the single most important
statistical consequence for the campaign, and it should be stated up front rather than discovered in
review. [DERIVED]

Also: **exact Mann–Whitney U cannot reach p<0.05 with 3 vs 3.** With n₁=n₂=3 there are C(6,3)=20
equally likely rank arrangements, so the smallest attainable two-sided p is 2/20 = **0.10**. With
4 vs 4 it is 2/70 = 0.029 (only under *complete* separation); 5 vs 5 gives 2/252 = 0.008. So **n=3
makes any "significant" claim arithmetically impossible** — a fact worth writing into the plan
before anyone runs a test. [DERIVED]

### 6.2 Reporting a difference with n=3–5

- Report **every run**, not just the summary: a strip/dot plot or a table of the raw values, plus
  **median [min–max]** and **IQR**. For n≤5, min–max is more honest than SD (SD from 3 points is
  nearly meaningless). Never report mean±SD as if it were a CI. [DERIVED]
- **Bootstrap CI of the difference in medians**: resample each arm with replacement (10 000
  resamples), compute median difference, take the 2.5/97.5 percentiles. With n=3/arm the CI will be
  enormous — that is the point; it communicates the uncertainty the point estimate hides. Report it
  even when it spans zero. [DERIVED]
- **Mann–Whitney U** only as a secondary, and only with n≥4/arm, using the **exact** distribution and
  saying so; with complete separation at 4 vs 4 the exact p is 0.029. Skip it at n=3. [DERIVED]
- Prefer **effect size + CI** over p-values: "median frame-interval p99 2.x is 0.82× that of 1.x
  (bootstrap 95 % CI 0.55–1.15, n=5/5)". Overlapping CI ⇒ "no difference detected at this sample
  size", **not** "equal". [DERIVED]
- **Report the observed MDR post hoc**: from your measured CV, state the smallest difference your n
  could have detected at 80 % power. This converts "we found nothing" into a bounded statement.
  [DERIVED]

### 6.3 Recommendations

1. **Raise to n=5 per condition, and 5 is a floor, not a target.** It costs ~2× the runtime of n=3
   and is the minimum that makes a p<0.05 statement arithmetically possible and a CI narrow enough
   to be informative. If the scene/thermal budget forbids it, declare the campaign as
   **"descriptive, MDR ≈ 40 %"** up front. [DERIVED]
2. Report **median [min–max], IQR, all raw points, bootstrap CI of the median difference, and the
   post-hoc MDR** for every metric. Ban mean±SD and ban bare p-values. [DERIVED]
3. **Pre-register the metric list and the discard rule** before the first run (cadence %, stalls per
   30 s, % window stalled, GPU busy ms, mean fps, meminfo PSS peak — one primary metric, the rest
   secondary). Multiple secondary metrics at n=5 without pre-registration will produce a spurious
   "win" with near-certainty. [DERIVED]
4. Interleave conditions (1.x, 2.x, 1.x, 2.x, …) rather than batching all runs of one build: this
   converts slow session drift (thermal, ambient, firmware state) from a bias into noise. [INFERENCE]

---

## 7. Benchmark-report disclosure checklist

A report is reproducible only if another team can rebuild the exact measurement. Disclose, per run:

**Device & OS**
- [ ] exact model + `ro.build.fingerprint` (+ build date), SoC, RAM; iOS: model id, iOS version+build
- [ ] display: physical size/resolution, **active mode & refresh rate**, adaptive-refresh setting, brightness value and auto-brightness state, ambient light/temperature
- [ ] battery level & charging state at run start (charging changes the thermal budget materially)
- [ ] any dev options changed (animations, GPU debug layers, "Force 4x MSAA", vendor game-mode/"Game Booster" toggles — vendor game modes change DVFS policy and must be named)

**Build under test**
- [ ] git commit SHA, build type (debug/release), R8/proguard & AOT state (`cmd package compile` mode), Godot engine version + fork SHA, feature flags (`use_livekit`, `use_deno`), and which shell (1.x vs 2.x) — with the SHA of each
- [ ] whether the two builds differ in anything else (HUD, resolution scale, renderer settings, MSAA, shadow quality) — **the report is void without this list**, since any of these changes the answer
- [ ] scene: parcel coordinates/URL, scene commit, number of avatars/players present, other participants in the room (comms traffic!), and network conditions

**Procedure**
- [ ] exact commands for every measurement tool, with versions (Perfetto trace-processor version, `adb version`, Xcode/`xctrace` version)
- [ ] window definition (30 s), how it was aligned to scene state (scene-clock offset), warm-up discarded, **n per condition**, run order/interleaving, cooldown criteria and measured cooldown durations
- [ ] the **discard rule** and how many runs were discarded and why
- [ ] thermal snapshot before/after every run; CPU/GPU frequency residency; per-thread big/little placement
- [ ] all settings pinned (airplane/Wi-Fi, DND, auto-rotate, screen timeout, stay-awake)

**Metrics & analysis**
- [ ] metric definitions with units and the exact SQL/queries; sample sizes (frame counts) next to every percentile
- [ ] all raw per-run values, plus median [min–max], IQR, bootstrap CI method & resample count, MDR
- [ ] the **negative results** and the metrics that did not separate the builds
- [ ] known limitations of the instrument (e.g. "iOS numbers are 1 Hz draw-rate samples; no present-time evidence"; "Frame Timeline measures presentation, not GPU time")

### 7.1 Recommendations

1. Ship the checklist as a **machine-written JSON/CSV run manifest** produced by the harness (one row
   per run: all device/thermal/frequency/build fields + metric outputs), and attach it to the report
   as an appendix. Hand-copied tables are where reproducibility dies. [INFERENCE]
2. Version-stamp the tooling (`adb version`, Perfetto trace-processor version/commit, `xctrace
   version`) — a percentile recomputed with a newer trace processor can move without any code change.
   [INFERENCE]
3. Require an explicit **"what differs between the two builds"** table signed off by whoever built
   them. Without it, a 1.x-vs-2.x difference is unattributable and the whole campaign is unfalsifiable.
   [INFERENCE]
4. Publish the harness (trace config, config file, sampling scripts, analysis SQL) in the repo so a
   second team can re-run against a future build; that is the only durable form of "reproducible".

---

## Residual uncertainty in this document

- **No source was fetched** (no web tool in this run): every URL is unverified, every `[DOC]` claim
  is memory-based platform knowledge. Verify before external publication.
- Sysfs paths for GPU (`/sys/kernel/gpu/*`, `/sys/class/devfreq/*`), Samsung thermal zone names, and
  the availability of Perfetto `gpu_counter`/`gpu_work_period` on Exynos 1380 are **unverified** and
  must be enumerated on the device.
- Whether `log stream --signpost` covers a **tethered iOS device** (vs macOS only) and whether
  `xcrun devicectl device process launch` can inject `MTL_HUD_ENABLED` are both uncertain; test
  rather than assume.
- Soak/cooldown durations (§2.4) are engineering inference, not documented values; the report should
  show the measured thermal traces that justify them.
- Godot HUD/a11y behaviour on iOS is inferred from how Godot draws its debug overlay; confirm by
  dumping the a11y tree once.
