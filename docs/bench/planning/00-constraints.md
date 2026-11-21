# Campaign constraints for the #3206 redo (owner: parent agent)

Written while the three scouts run. Facts only from our own measurements and the device probes done today.

## What the previous pass proved is broken

1. The settings panel is not evidence. 3 of 6 runs in `gp-hi-pair/` loaded a profile other than the one
   shown, so every claim now requires the `[RenderReady]` line of that run's own pid.
2. The sun moves the result more than the build does: same build, same profile, 31.3 fps at luma 117 vs
   13.5-15.2 at luma 72-78.
3. The sky cannot be pinned in Genesis Plaza from the client: the scene sets `sdk_skybox_time_active`.
4. The sky cannot be pinned in 2.x anywhere yet: with `Dynamic Skybox` off + `Custom Skybox = Midday`,
   2.x renders night (luma 33.1) while 1.x renders day (luma 110.6-111.9) on parcel 100,100. The read site
   is `skybox_time.rs` with a `unwrap_or(0.0)` fallback, and 0.0 is midnight.
5. Within one session the light is stable (12 windows, 111 s apart, luma 56.4-61.0), so a session is a
   usable condition; a build switch is not (it wipes settings and takes minutes).

## Device capabilities probed today (Galaxy A54, R5CW60624NE)

- `dumpsys thermalservice` gives `Temperature{mValue=.., mType=0, mName=AP}`; AP reads 34-49 °C today.
- `/sys/class/thermal/thermal_zone*/type` is not readable (permission), so zone names are unavailable.
- No GPU devfreq node exposed (`/sys/class/devfreq` has no gpu/mali entry).
- CPU `scaling_cur_freq` is readable for cpu0 (1.6 GHz) and cpu7 (1.8 GHz) - usable as a recorded
  confounder, not as a control.
- Perfetto is present (`/system/bin/perfetto` plus configs in `/data/local/tmp`).
- Two APKs are cached: `.bench-dl/apk-1x-release-latest.apk` (1.16.1) and
  `.bench-dl/apk-2x-e3b7aa102.apk` (2.0.0.2723-e3b7aa1-staging).
- iPhone 16e is connected to devicectl (`1DD7B8A8-...`); no frame timeline there, HUD fps at 1 Hz only.
- Build switching needs uninstall+install (version downgrade refused), ~2-4 min, and wipes settings.

## Non-negotiables inherited from the user

- Measurement only: no client fixes in this PR. Findings go to the report and the roadmap.
- Only the worktree `sprint28-bench-2x-3206`; PR #3227 stays a draft.
- Every published number must come with its verified condition; no delta without a matched condition.
- Artifacts in English, source docs in Spanish.
- Do not edit scripts while they are running; kill by explicit pid.

## Open questions the plan has to answer

- Is there any configuration in which both builds render the same light, verified by measurement, without
  touching client code? Candidates: world clock with `Dynamic Skybox` on (shared reference), the scene's own
  `SkyboxTime` in Genesis Plaza, the gp-benchmark `fixed-skybox-time` override (2.x only), or a scene-free
  parcel where the client setting applies.
- If no such configuration exists, what is the defensible alternative: report bands of luma per build and
  refuse a single delta, or measure a different scene where the sun is not a variable.
- How many runs per condition are needed for the observed spread (10-20 %) at n=3-5.
- What has to be recorded per run so a reviewer can invalidate it without re-running.
