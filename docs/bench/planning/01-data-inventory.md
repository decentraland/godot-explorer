# 01 — Data inventory: `docs/bench/data/2x-vs-1x-3206/`

Read-only inventory. Every number below was read from a file in the tree. `missing` means the file/key/dir does not exist.

Root is `/Users/lordmanuel/Projects/decentraland/godot-explorer/sprint28-bench-2x-3206/docs/bench/data/2x-vs-1x-3206`.

Top-level layout: `android-a54/`, `ios-16e/`, `iphone16e/`, `smoke/`.

---

## 1. Scenario table

`runs` = number of run directories that contain at least one benchmark artefact (a directory holding `meta.json` / `frames.csv` / `cycles.csv` / `state.json` / `launch.log`, or an empty/partial run dir). `has fps` = a run has `meta.json` with `frames.fps_avg`; `has luma` = a run has `state.json` with `luma_mean`; `has profile` = a `[RenderReady]` line exists in a launch log for that run (see note in §6 about scenario-level `launch-<label>.log`); `invalid` = an `INVALID.md` at scenario level or inside a run.

| scenario (rel. to root) | runs | builds (count) | has fps | has luma | has profile | invalid | size |
|---|---:|---|---|---|---|---|---|
| `android-a54/enter-exit` | 2 | 1x 1, 2x 1 | no | no | no | no | 8,340,292 B (8.0 MiB) |
| `android-a54/gp` | 10 | 1x 5, 2x 5 | yes (10) | no | yes, profile 0 (10, scenario-level logs) | no | 5,129,856 B (4.9 MiB) |
| `android-a54/gp-empty` | 6 | 1x 3, 2x 3 | yes (6) | no | yes, profile 0 (6, scenario-level logs) | **yes** (`gp-empty/INVALID.md`) | 2,843,671 B (2.7 MiB) |
| `android-a54/gp-hi` | 5 | 2x 5 | yes (5) | no | yes, profile 3 (5, scenario-level logs) | no | 203,453,719 B (194.0 MiB) |
| `android-a54/gp-hi-ab` | 2 | 1x 1, 2x 1 | yes (1, 2x only) | yes (1, 2x only) | yes (1, 2x only) | no | 22,236,743 B (21.2 MiB) |
| `android-a54/gp-hi-fixed-sky` | 6 | 1x 3, 2x 3 | yes (6) | yes (6) | yes, profile 3 (6) | no | 119,370,341 B (113.8 MiB) |
| `android-a54/gp-hi-mid` | 1 | — (empty dir) | no | no | no | no | 0 B |
| `android-a54/gp-hi-midday` | 3 | 1x 3 | yes (3) | yes (3) | yes, profile 3 (3) | no | 60,567,195 B (57.8 MiB) |
| `android-a54/gp-hi-pair` | 7 | 1x 4, 2x 3 | yes (6 of 7) | no | yes (6 of 7) | **yes** (`gp-hi-pair/INVALID.md`) | 122,365,272 B (116.7 MiB) |
| `android-a54/gp-hi-pair-v2` | 1 | 2x 1 | yes (1) | no | yes, profile 3 (1) | no | 20,583,155 B (19.6 MiB) |
| `android-a54/gp-hi-v3` | 3 | 1x 3 | yes (3) | yes (3) | yes, profile 3 (3) | no | 74,841,310 B (71.4 MiB) |
| `android-a54/gp-smoke` | 0 | — (empty dir) | no | no | no | no | 0 B |
| `android-a54/gp-sun-drift` | 12 | 2x 12 | yes (12) | no | no | no | 233,485,761 B (222.7 MiB) |
| `android-a54/rss-states` | 1 | 1x 1 (no meta) | no | no | no | no | 3,298,296 B (3.1 MiB) |
| `android-a54/sky-fixed-midday` | 5 | 1x 3, 2x 2 | yes (3) | yes (3) | yes (1) | **yes** (`1x-skyfix-r1/INVALID.md`) | 55,512,596 B (52.9 MiB) |
| `android-a54/soak` | 1 | 2x 1 | meta present, `fps_avg` 0.000000 | no | no | no | 2,854,602 B (2.7 MiB) |
| `android-a54/ui-2x` | 1 | — (no meta) | no | no | no | no | 2,851,606 B (2.7 MiB) |
| `ios-16e/gp-hi-ios` | 1 | 2x 1 | no (`frames` key absent; HUD samples only) | no `state.json` (luma in `meta.json`) | no | no | 3,989,200 B (3.8 MiB) |
| `iphone16e/launch` | 2 | 1x 1, 2x 1 | no | no | no | no | 55,706 B (0.1 MiB) |
| `smoke` | 2 | 1x 1, 2x 1 | yes (2) | no | yes, profile 0 (2, scenario-level logs) | no | 37,758 B (0.0 MiB) |

Top-level total: **941,819,287 B (898.2 MiB)**.

---

## 2. Per-run file schema

### `meta.json`
Android runs have identical key order:
`label, build, scenario, device, device_serial, pkg, version_name, pid_before, trace_wall_seconds, started_at, frames{rss_peak_kb,...}, rss_peak_kb, rss_steady_kb, mem_samples, notes, thermal_end_c, cpu_mean_pct`

`frames` sub-keys: `frames, fps_avg, span_s, interval_p50_ms, interval_p95_ms, interval_p99_ms, interval_max_ms, fps_p1_low, fps_p5_low, fps_median, pct_missed_deadline, jank_any`. All values are **strings** (e.g. `"fps_avg": "17.940000"`).

Example (`android-a54/gp/2x-gp-r1/meta.json`):
```json
{"label":"2x-gp-r1","build":"2x","scenario":"gp","device":"SM-A546E","version_name":"2.0.0(2723)",
 "frames":{"frames":"1105","fps_avg":"18.360000","span_s":"59.880000",
           "interval_p50_ms":"58.200000","interval_p95_ms":"66.750000","interval_p99_ms":"74.860000",
           "interval_max_ms":"75.200000","fps_p1_low":"13.360000","fps_p5_low":"14.980000",
           "fps_median":"17.180000","pct_missed_deadline":"2.170000","jank_any":"1084"},
 "rss_peak_kb":2046348,"rss_steady_kb":2014122,"mem_samples":36,"notes":"",
 "thermal_end_c":42.7,"cpu_mean_pct":116.4}
```

iOS meta has a **different** shape (no `scenario`, no `frames` block): `label, build, platform, device, app, scene, settings{dynamic_graphics_enabled,graphic_profile,dynamic_skybox,skybox_time,shadow_quality,bloom_quality}, method, fps_samples[20], fps_median, fps_min, fps_max, luma_mean` (`ios-16e/gp-hi-ios/2x-ios-r1/meta.json`).

### `frames.csv`
Headers over all 59 `frames.csv` files: 58 files share one header; `android-a54/soak/2x-soak-60m/frames.csv` is the only outlier (header `dur_ms`, **1 line total**, no rows).
```
ts,interval_ms,jank_type,on_time_finish,present_type,display_frame_token
158943885069659,[NULL],Display HAL,1,Late Present,3563293
```
Typical rows: `frames` value + 1 header → e.g. 529 lines for 528 frames; 1080 lines for 1079 frames.

### `cpu.csv`
```
cpu_pct,t
164.0,0.0
```
Typical: 19 lines (30 s window, 1 Hz-ish) to 36 lines (60 s window); soak 1465 lines.

### `mem.csv`
```
pid,pss_code_kb,pss_graphics_kb,pss_java_heap_kb,pss_native_heap_kb,pss_private_other_kb,pss_stack_kb,pss_system_kb,swap_pss_kb,t,total_pss_kb,total_rss_kb
27411,122156,753190,16172,1144496,52608,3700,12469,167,0.0,2104791,2233378
```
Typical: same row count as `cpu.csv` (19 / 36 / 1465).

### `thermal.csv`
```
t,thermal_max_c,thermal_skin_c
0.0,43.8,36.3
```
Typical: same row count as `cpu.csv`.

### `state.json` (only 15 runs have it)
Two schemas:
- A (12 runs: `gp-hi-ab`, `gp-hi-fixed-sky`, `gp-hi-midday`, `gp-hi-v3`): `luma_mean, thermal_start_c, thermal_skin_start_c, thermal_end_ap_c` — e.g. `{"luma_mean":48.9,"thermal_start_c":45.5,"thermal_skin_start_c":38.1,"thermal_end_ap_c":49.0}`
- B (3 runs: `sky-fixed-midday/1x-skyfix-r1..r3`): `luma_mean, thermal_start_c, thermal_end_ap_c, note` — **no `thermal_skin_start_c`**, plus `"note": "pinned sky applied live in this session; window without relaunch"`.

### `cycles.csv` (only `android-a54/enter-exit/1x` and `/2x`)
```
cycle,version,load_ms,pss_kb,footprint_mb,level
1,1.16.1,15363,2096447,2139,0
```

---

## 3. Runs usable as-is

Definition used: `meta.json` with `frames.fps_avg` + `interval_p50_ms` + `interval_p99_ms`, `state.json` with `luma_mean` + `thermal_start_c`, a `[RenderReady]` profile line in that run's own `launch.log`, and no `INVALID.md` at run or scenario level.

**android-a54/gp-hi-fixed-sky — 1x:** `1x-sky-r1`, `1x-sky-r2`, `1x-sky-r3` (profile 3)  
**android-a54/gp-hi-fixed-sky — 2x:** `2x-sky-r1`, `2x-sky-r2`, `2x-sky-r3` (profile 3)  
**android-a54/gp-hi-midday — 1x:** `1x-mid-r1`, `1x-mid-r2`, `1x-mid-r3` (profile 3)  
**android-a54/gp-hi-v3 — 1x:** `1x-v3-r1`, `1x-v3-r2`, `1x-v3-r3` (profile 3)  
**android-a54/gp-hi-ab — 2x:** `2x-ab-r1` (profile 3)  
**android-a54/sky-fixed-midday — 1x:** `1x-skyfix-r2`, `1x-skyfix-r3` (no `launch.log` in run dir; profile 3 from `1x-skyfix-r1`'s log; r1 excluded by `INVALID.md`)

Total: **15 runs** (1x: 11, 2x: 4). Only 4 of these are 2x; 11 are 1x.

Near-miss (all data present except `state.json`): `android-a54/gp-hi-pair-v2/2x-hi-r1` — meta + profile 3 + `launch.log` + `screen.png` + trace, **no `state.json`**.

---

## 4. Incomplete or invalid runs (reason read from the files)

| run | reason |
|---|---|
| `android-a54/enter-exit/1x`, `.../2x` | no `meta.json`, no `state.json`, no `launch.log`; only `cycles.csv` + `cycle-01.png`/`cycle-20.png` |
| `android-a54/gp/1x-gp-r2,r4,r6,r8,r10`, `2x-gp-r1,r3,r5,r7,r9` | `meta.json` present, but `state.json` missing, no `launch.log` in run dir, no `trace.perfetto-trace`; `screen.png` only in `1x-gp-r4` and `2x-gp-r3` |
| `android-a54/gp-empty/1x-gp-empty-r2,r4,r6`, `2x-gp-empty-r1,r3,r5` | scenario `INVALID.md`: every run shows the "Connection lost / EXIT / RETRY" modal (protocol §8 rule 2) — "do not use these numbers" |
| `android-a54/gp-hi/2x-gp-hi-r1..r5` | `state.json` missing (no luma/thermal start); no `launch.log` in run dir (scenario-level `launch-2x-gp-hi-r*.log` has profile 3) |
| `android-a54/gp-hi-ab/1x-ab-r1` | only `settings-1-menu.png` + `settings-2-graphics.png`; no `meta.json`/`frames.csv` |
| `android-a54/gp-hi-mid/2x-mid-r1` | empty directory (`ls` returns nothing) |
| `android-a54/gp-hi-pair/1x-hi-p1..p4`, `2x-hi-p1..p2` | `state.json` missing (no luma); scenario `INVALID.md`: profile not HIGH in `1x-hi-p1` (0), `1x-hi-p4` (0), `2x-hi-p1` (ends 2, reason `settings`); sky not pinned. Re-read confirms profiles 0/0/2 |
| `android-a54/gp-hi-pair/2x-hi-p3` | failed run: dir holds only `settings.png`; no `meta.json` (INVALID.md: "— (run failed)") |
| `android-a54/gp-hi-pair-v2/2x-hi-r1` | `state.json` missing (meta + profile 3 present) |
| `android-a54/gp-smoke` | empty directory |
| `android-a54/gp-sun-drift/2x-drift/s1..s12` | `state.json` missing, no `launch.log` in run dir; meta + trace + screen present |
| `android-a54/rss-states/1x` | no `meta.json`; only `1-menus-1.log`…`5-after-2nd-exit-1.log`, `state-2-world.png`, `state-3-after-exit.png`, `states.csv`, `states-median.csv` |
| `android-a54/sky-fixed-midday/1x-skyfix-r1` | run `INVALID.md`: "Window luma 34.4 is far below every valid run..." (state.json records 110.6 — the INVALID.md quotes 34.4 for the measured window) |
| `android-a54/sky-fixed-midday/2x-skyfix-r1`, `2x-skyfix-r2` | each dir holds only `screen.png` (both 1,965,109 B, identical size); no `meta.json`. `sky-fixed-midday/RESULT.json` records `"2x": {"windows": 0}` |
| `android-a54/soak/2x-soak-60m` | `meta.json` `frames.frames = "0"`, `fps_avg = "0.000000"`, `span_s`/`interval_p50_ms`/`interval_p99_ms`/`interval_max_ms`/`fps_p1_low`/`fps_p5_low`/`fps_median`/`pct_missed_deadline`/`jank_any` all `"[NULL]"`; `frames.csv` header only (`dur_ms`, 1 line); no `state.json`, no trace |
| `android-a54/ui-2x/explore` | no `meta.json`; screenshots + `*.texts.txt` only; scenario has `README.md`, `navleak.csv`, `tapspam.csv`, `ttc-a11y.csv`, `ttc-pixel.csv` |
| `ios-16e/gp-hi-ios/2x-ios-r1` | `meta.json` has **no `frames` block** (no `fps_avg`/`p50`/`p99`): 20 `fps_samples`, `fps_median` 38.0, `fps_min` 37, `fps_max` 40, `luma_mean` 61.9; no `state.json`, no trace |
| `iphone16e/launch/1x`, `.../2x` | no `meta.json`; only `console-1..3.log` + `launch-1..3.json` |
| `smoke/1x-gp-smoke-r2`, `2x-gp-smoke-r1` | `meta.json` present with fps/p50/p99, but `state.json` missing, no `launch.log` in run dir, no trace |
| `smoke/smoke` | empty directory |

---

## 5. Disk size

| path | bytes | MiB |
|---|---:|---:|
| `android-a54/enter-exit` | 8,340,292 | 8.0 |
| `android-a54/gp` | 5,129,856 | 4.9 |
| `android-a54/gp-empty` | 2,843,671 | 2.7 |
| `android-a54/gp-hi` | 203,453,719 | 194.0 |
| `android-a54/gp-hi-ab` | 22,236,743 | 21.2 |
| `android-a54/gp-hi-fixed-sky` | 119,370,341 | 113.8 |
| `android-a54/gp-hi-mid` | 0 | 0.0 |
| `android-a54/gp-hi-midday` | 60,567,195 | 57.8 |
| `android-a54/gp-hi-pair` | 122,365,272 | 116.7 |
| `android-a54/gp-hi-pair-v2` | 20,583,155 | 19.6 |
| `android-a54/gp-hi-v3` | 74,841,310 | 71.4 |
| `android-a54/gp-smoke` | 0 | 0.0 |
| `android-a54/gp-sun-drift` | 233,485,761 | 222.7 |
| `android-a54/rss-states` | 3,298,296 | 3.1 |
| `android-a54/sky-fixed-midday` | 55,512,596 | 52.9 |
| `android-a54/soak` | 2,854,602 | 2.7 |
| `android-a54/ui-2x` | 2,851,606 | 2.7 |
| `ios-16e/gp-hi-ios` | 3,989,200 | 3.8 |
| `iphone16e/launch` | 55,706 | 0.1 |
| `smoke` | 37,758 | 0.0 |
| **TOTAL (`docs/bench/data/2x-vs-1x-3206`)** | **941,819,287** | **898.2** |

Per top-level dir (from `du -sh`): `android-a54/` 910M, `ios-16e/` 3.8M, `iphone16e/` 96K, `smoke/` 80K — total 914M (du block-rounded).

Largest single run dirs: `android-a54/gp-hi/2x-gp-hi-r4` = 42,875,780 B; `.../2x-gp-hi-r5` = 42,694,881 B; `.../2x-gp-hi-r3` = 41,394,180 B (each holds ~40 `meminfo-*.log` files + a large `trace.perfetto-trace`).

---

## 6. Inconsistencies observed

1. **`launch.log` lives in the run dir for some scenarios and at scenario level for others.** `gp-hi-fixed-sky`, `gp-hi-midday`, `gp-hi-v3`, `gp-hi-ab`, `gp-hi-pair`, `gp-hi-pair-v2`, `sky-fixed-midday/1x-skyfix-r1` keep `launch.log` (and `launch.json`) inside the run dir. `gp`, `gp-empty`, `gp-hi`, `smoke` keep `launch-<label>.log` + `<label>-launch.json` **one level up** and no `launch.log` in the run dir. Consequence: a checker that looks only for `<run>/launch.log` reports "no profile" for 23 runs that do have a `[RenderReady]` line with a profile.
2. **Two `state.json` schemas.** `sky-fixed-midday/1x-skyfix-r*` omit `thermal_skin_start_c` and add `note`; the other 12 runs use `thermal_skin_start_c` and no `note`.
3. **`state.json` coverage is 15 of 59 run dirs.** Entire scenarios have none: `gp`, `gp-empty`, `gp-hi`, `gp-sun-drift`, `soak`, `smoke`, `gp-hi-pair`, `gp-hi-pair-v2`.
4. **`frames.csv` schema breaks in the soak run**: header `dur_ms`, 1 line, 0 rows, while `meta.json` still reports a `frames` block full of `[NULL]`.
5. **`meta.json` schema differs by platform.** Android has a `frames{}` block and `scenario`; iOS has `platform`, `scene`, `settings{}`, `fps_samples[]`, `luma_mean` and no `scenario`/`frames`.
6. **Duplicate/leftover artefact dirs:** `sky-fixed-midday/2x-skyfix-r1` and `2x-skyfix-r2` each contain only `screen.png` of byte-identical size (1,965,109 B) — no metrics. `gp-hi-pair/2x-hi-p3` contains only `settings.png`. `gp-hi-ab/1x-ab-r1` contains only two settings screenshots.
7. **Empty directories:** `android-a54/gp-hi-mid/2x-mid-r1`, `android-a54/gp-smoke`, `smoke/smoke`.
8. **Asymmetric build coverage per scenario.** `gp-hi` is 2x-only (5). `gp-hi-midday` and `gp-hi-v3` are 1x-only (3 each). `gp-hi-pair-v2` is 2x-only (1) — no 1x counterpart. `gp-sun-drift` is 2x-only (12). `sky-fixed-midday` has 3 valid 1x windows and 0 valid 2x windows.
9. **`gp` labels interleave builds non-sequentially:** `1x-gp-r2,r4,r6,r8,r10` vs `2x-gp-r1,r3,r5,r7,r9` (alternating by index). No label string is used twice anywhere in the tree.
10. **`gp-hi-pair` profile drift within a single run.** `2x-hi-p1` has 4 `[RenderReady]` lines (3 loading at profile 3, then `reason:"settings"` → profile 2); `2x-hi-p2` has 4 (0,0,3,3); `1x-hi-p1` and `1x-hi-p3` have 2 lines each. Only `gp-hi-fixed-sky`, `gp-hi-midday`, `gp-hi-v3`, `gp-hi-ab` have exactly 1 `[RenderReady]` line per run.
11. **`INVALID.md` at both scenario and run level.** Scenario-level: `gp-empty/`, `gp-hi-pair/`. Run-level: `sky-fixed-midday/1x-skyfix-r1/INVALID.md`.
12. **`meta.json` notes field is `""` everywhere except `sky-fixed-midday/1x-skyfix-r1..r3`** (`"one session, pinned sky set live, no relaunch"`).
13. **`cpu_mean_pct` is `0.0` in both `smoke` runs** (`1x-gp-smoke-r2`, `2x-gp-smoke-r1`) while fps was measured — a zero CPU value in an otherwise-populated meta.
14. **Scenario-level extras:** `android-a54/summary.md` + `summary.json`, `android-a54/sky-fixed-midday/RESULT.json` (`"2x": {"windows": 0}`, `"luma_mean": [33.1]`), `android-a54/ui-2x/README.md`.
