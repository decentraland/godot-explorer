# gp-hi-pair — invalid as a comparison

Two conditions the protocol requires were not actually held, and both were found after the fact by
re-reading the run evidence. The numbers in this directory are kept as history, not as results.

## 1. The graphics profile was not `HIGH` in every run

`[RenderReady]` `"profile"` in each run's `launch.log` (2 = medium, 3 = high):

| run | profile | fps avg |
|---|---|---|
| `1x-hi-p1` | **0 (low)** | 24.49 |
| `1x-hi-p2` | 3 | 15.47 |
| `1x-hi-p3` | 3 | 15.92 |
| `1x-hi-p4` | **0 (low)** | 13.50 |
| `2x-hi-p1` | **2 (medium)** | 31.31 |
| `2x-hi-p2` | 3 | 30.53 |
| `2x-hi-p3` | — (run failed) | — |

The settings were applied by tapping the UI, and the panel showed `High`, but the engine loaded a
different preset in three of the six runs: on 1.x the tap sequence can leave the dropdown on its
previous value, and on 2.x the shell writes the setting asynchronously. So `1x-hi-p1` (24.49 fps, the
outlier this directory was re-measured four times over) is a **low**-profile run, and `2x-hi-p1`
(31.31 fps) is a **medium**-profile one.

## 2. The sky time was not pinned

`Dynamic Skybox` was ON in every run here, so the sun follows the world clock and the window was
measured at whatever time of day the run happened to reach. Mean screen luminance of the measured
frame (a proxy for how low the sun is) against fps, all runs:

| run | luma | fps avg |
|---|---:|---:|
| `1x-hi-p1` | 129.4 | 24.49 |
| `2x-hi-p2` | 128.5 | 30.53 |
| `2x-hi-p1` | 117.6 | 31.31 |
| `1x-hi-p3` | 90.3 | 15.92 |
| `1x-hi-p4` | 89.5 | 13.50 |
| `1x-hi-p2` | 78.0 | 15.47 |

The swing inside one build across that column is larger than the difference between the builds.

## What replaced it

`gp-hi-fixed-sky/`: same phone, same pose, `Dynamic Skybox` OFF (pinned to 06:00, "Morning"), profile
verified as `"profile":3` in the `[RenderReady]` of that run's own pid, no profile change inside the
window, and `state.json` recording the luminance so a run that drifts is visible.
