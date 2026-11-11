# Reconstructed 2.x vs 1.x runs (offline, android-a54)

Source: `docs/bench/data/2x-vs-1x-3206/android-a54`  
Runs with a `frames.csv` or `meta.json`: **57**.  
Derivation: `frames.csv` cadence + stalls, `meta.json` stats, `screen.png` Rec.601 luma (full frame and central 70 % x 70 %, x/y in [0.15, 0.85]), `state.json`, launch profile.  
Arms where any `INVALID.md` covers a run are marked † and excluded from (b)/(c).

## (a) Per scenario (one row per build)

| scenario | build | runs | fps med [min-max] | modal P | pct_on_modal med | stalls>=2 med | luma_crop med [min-max] |
|---|---|---:|---|---:|---:|---:|---|
| `gp` | 1x | 5 | 17.94 [17.85-17.94] | 7 | 46.05 | 8 | 37.18 [37.18-37.18] |
| `gp` | 2x | 5 | 18.07 [17.92-18.36] | 7 | 44.76 | 13 | 73.06 [73.06-73.06] |
| `gp-empty` † | 1x | 3 | 17.95 [17.94-17.95] | 7 | 49.58 | 9 | 73.64 [73.64-73.64] |
| `gp-empty` † | 2x | 3 | 17.96 [17.95-18.00] | 7 | 41.52 | 21 | 45.69 [45.69-45.69] |
| `gp-hi` | 2x | 5 | 14.99 [13.64-16.94] | 10 (spread 7-10) | 47.14 | 3 | 80.73 [69.78-90.61] |
| `gp-hi-ab` | 2x | 1 | 13.51 [13.51-13.51] | 9 | 38.42 | 52 | 53.00 [53.00-53.00] |
| `gp-hi-fixed-sky` | 1x | 3 | 17.93 [17.48-18.82] | 7 (spread 6-7) | 44.44 | 26 | 105.57 [102.94-108.51] |
| `gp-hi-fixed-sky` | 2x | 3 | 14.42 [13.47-15.18] | 9 (spread 8-9) | 53.33 | 7 | 74.70 [68.53-79.17] |
| `gp-hi-midday` | 1x | 3 | 16.41 [16.26-17.95] | 7 | 38.16 | 70 | 103.67 [92.46-106.82] |
| `gp-hi-pair` † | 1x | 4 | 15.70 [13.50-24.49] | 8 (spread 5-9) | 30.25 | 36.00 | 82.47 [77.74-128.77] |
| `gp-hi-pair` † | 2x | 2 | 30.92 [30.53-31.31] | 4 | 60.23 | 20.50 | 127.05 [118.92-135.18] |
| `gp-hi-pair-v2` | 2x | 1 | 19.77 [19.77-19.77] | 6 | 58.22 | 4 | 86.17 [86.17-86.17] |
| `gp-hi-v3` | 1x | 3 | 15.83 [14.06-26.87] | 8 (spread 4-8) | 52.72 | 41 | 99.54 [85.14-110.58] |
| `gp-sun-drift` | 2x | 12 | 21.50 [18.38-22.89] | 6 (spread 5-7) | 43.50 | 29.50 | 54.20 [53.88-60.93] |
| `sky-fixed-midday` † | 1x | 3 | 36.70 [33.96-40.29] | 3 (spread 3-4) | 47.40 | 10 | 115.39 [114.86-115.48] |
| `soak` | 2x | 1 | 0.00 [0.00-0.00] | n/a | n/a | 0 | 119.85 [119.85-119.85] |

## (b) Within-arm coefficient of variation (same scenario and build)

Sample SD / mean over the runs of one arm; `n` is the run count that entered the CV. A metric whose arm mean is 0 (no stalls in every run) has no CV and prints `n/a`.

| arm | n | fps_avg | pct_frames_on_modal | stalls_ge2 | stall_time_frac | interval_p99_ms |
|---|---:|---:|---:|---:|---:|---:|
| `gp` / 1x | 5 | 0.2 % | 11.0 % | 73.3 % | 71.8 % | 6.4 % |
| `gp` / 2x | 5 | 1.0 % | 9.6 % | 31.1 % | 31.7 % | 4.8 % |
| `gp-empty` / 1x † | 0 | n/a | n/a | n/a | n/a | n/a |
| `gp-empty` / 2x † | 0 | n/a | n/a | n/a | n/a | n/a |
| `gp-hi` / 2x | 5 | 9.3 % | 37.2 % | 205.0 % | 198.6 % | 8.8 % |
| `gp-hi-ab` / 2x | 1 | n/a | n/a | n/a | n/a | n/a |
| `gp-hi-fixed-sky` / 1x | 3 | 3.8 % | 1.5 % | 71.3 % | 65.4 % | 6.5 % |
| `gp-hi-fixed-sky` / 2x | 3 | 6.0 % | 20.2 % | 84.4 % | 92.5 % | 13.6 % |
| `gp-hi-midday` / 1x | 3 | 5.5 % | 21.0 % | 73.9 % | 73.8 % | 5.9 % |
| `gp-hi-pair` / 1x † | 0 | n/a | n/a | n/a | n/a | n/a |
| `gp-hi-pair` / 2x † | 0 | n/a | n/a | n/a | n/a | n/a |
| `gp-hi-pair-v2` / 2x | 1 | n/a | n/a | n/a | n/a | n/a |
| `gp-hi-v3` / 1x | 3 | 36.7 % | 2.4 % | 74.5 % | 60.0 % | 22.2 % |
| `gp-sun-drift` / 2x | 12 | 5.7 % | 11.5 % | 57.0 % | 47.4 % | 8.4 % |
| `sky-fixed-midday` / 1x † | 2 | 6.6 % | 6.2 % | 79.9 % | 79.9 % | 14.7 % |
| `soak` / 2x | 1 | n/a | n/a | n/a | n/a | n/a |

## (c) Lowest within-arm CV = the metric to pre-register

Rule used here: for each candidate, take the CV of every valid arm (no `INVALID.md`) with n >= 3 runs, then report the count of such arms, the median of those CVs and the worst one. The median is the ranking statistic; the worst is the risk.

| candidate | arms with n>=3 | median CV | worst CV |
|---|---:|---:|---:|
| `fps_avg` | 8 | 5.6 % | 36.7 % |
| `interval_p99_ms` | 8 | 7.4 % | 22.2 % |
| `pct_frames_on_modal` | 8 | 11.2 % | 37.2 % |
| `stall_time_frac` | 8 | 68.6 % | 198.6 % |
| `stalls_ge2` | 8 | 73.6 % | 205.0 % |

**Pre-register `fps_avg`**: lowest median within-arm CV, **5.6 %** across 8 arms with n>=3 (worst arm 36.7 %).
Runner-up `interval_p99_ms` at 7.4 % over 8 arms. At CV = 5.6 % the MDR at n=6/build is 9.1 % (n=4: 11.1 %).

## (d) Is the existing 2.x arm `gp-hi/2x-gp-hi-r1..r5` usable?

Those runs have no `state.json` and no per-run `launch.log`, so nothing derived from them was ever published. `luma_crop` is a pure function of the stored `screen.png`; the cadence columns come from `frames.csv`; the profile comes from the scenario-level logs.

| run | build | luma_crop | luma_full | modal P | pct_on_modal | stalls>=2 | fps_avg | profile (source) |
|---|---|---:|---:|---:|---:|---:|---:|---|
| `2x-gp-hi-r1` | 2x | 83.5 | 81.0 | 7 | 25.47 | 210 | 16.94 | 3 (`gp-hi/launch-2x-gp-hi-r1.log`) |
| `2x-gp-hi-r2` | 2x | 80.7 | 82.9 | 10 | 42.09 | 7 | 13.84 | 3 (`gp-hi/launch-2x-gp-hi-r2.log`) |
| `2x-gp-hi-r3` | 2x | 79.8 | 80.8 | 10 | 47.14 | 3 | 13.64 | 3 (`gp-hi/launch-2x-gp-hi-r3.log`) |
| `2x-gp-hi-r4` | 2x | 69.8 | 66.7 | 8 | 73.86 | 2 | 16.00 | 3 (`gp-hi/launch-2x-gp-hi-r4.log`) |
| `2x-gp-hi-r5` | 2x | 90.6 | 92.8 | 9 | 62.42 | 3 | 14.99 | 3 (`gp-hi/launch-2x-gp-hi-r5.log`) |
| `1x-sky-r1` | 1x | 102.9 | 104.6 | 7 | 45.35 | 26 | 17.48 | 3 (`gp-hi-fixed-sky/1x-sky-r1/launch.log`) |
| `1x-sky-r2` | 1x | 105.6 | 107.4 | 7 | 44.07 | 10 | 17.93 | 3 (`gp-hi-fixed-sky/1x-sky-r2/launch.log`) |
| `1x-sky-r3` | 1x | 108.5 | 108.8 | 6 | 44.44 | 51 | 18.82 | 3 (`gp-hi-fixed-sky/1x-sky-r3/launch.log`) |

## (e) gp-sun-drift/2x-drift s1..s12 — sun sensitivity from existing runs

| run | luma_crop | luma_full | fps_avg | modal P | pct_on_modal | cpu % | thermal_end °C |
|---|---:|---:|---:|---:|---:|---:|---:|
| `2x-drift-s1` | 54.4 | 56.8 | 21.52 | 6 | 45.92 | 200.2 | 47.8 |
| `2x-drift-s2` | 54.5 | 56.8 | 22.02 | 6 | 42.77 | 186.5 | 48.2 |
| `2x-drift-s3` | 54.3 | 56.8 | 20.85 | 6 | 46.02 | 192.1 | 48.1 |
| `2x-drift-s4` | 57.1 | 60.6 | 22.33 | 6 | 43.54 | 188.1 | 48.6 |
| `2x-drift-s5` | 54.0 | 56.7 | 22.89 | 5 | 50.43 | 189.8 | 48.7 |
| `2x-drift-s6` | 53.9 | 56.3 | 22.38 | 5 | 40.89 | 193.9 | 48.7 |
| `2x-drift-s7` | 54.1 | 56.6 | 20.61 | 6 | 52.01 | 183.2 | 48.7 |
| `2x-drift-s8` | 54.1 | 56.7 | 21.40 | 6 | 36.28 | 200.7 | 48.8 |
| `2x-drift-s9` | 54.8 | 57.0 | 18.38 | 7 | 35.92 | 174.2 | 48.5 |
| `2x-drift-s10` | 54.0 | 56.7 | 21.48 | 6 | 39.26 | 194.8 | 48.9 |
| `2x-drift-s11` | 54.0 | 57.7 | 20.13 | 6 | 45.96 | 193.0 | 48.8 |
| `2x-drift-s12` | 60.9 | 60.0 | 21.55 | 6 | 43.45 | 185.7 | 49.1 |

luma_crop 53.9-60.9 with fps 18.38-22.89 (median 21.50) over 12 sequential windows; no new runs needed for this curve.
