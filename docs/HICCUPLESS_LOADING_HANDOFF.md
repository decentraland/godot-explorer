# Hiccupless loading — handoff (#3015)

Status as of 2026-10-09. Everything below was measured on a **Samsung A54** (Mali-G68, Android 16) and
an **iPhone 13** (iOS 26), clean install, **Genesis Plaza**, with the benchmark crowd described in
[Benchmark crowd](#benchmark-crowd).

## 1. Branches and PRs

| Repo | Branch | What it is |
|---|---|---|
| decentraland/godotengine | `perf/hiccupless-loading` (PR #27, base `4.6.2`) | **Production.** Non-blocking pipeline/shader compiles, all behind fork-only settings. |
| decentraland/godotengine | `perf/hiccupless-loading-profiling` | Production + profiler zones (Perfetto/Tracy/Instruments) + a GDExtension zone API. |
| decentraland/godot-explorer | `perf/hiccupless-loading` | **Production.** Explorer-side changes; `GODOT_USE_BRANCH` points at the engine branch above. |
| decentraland/godot-explorer | `perf/hiccupless-loading-profiling` (this doc) | Production + profiler bridge, zones/marks, benchmark crowd, Perfetto scripts. |

The profiling branches are the production branches plus instrumentation only; diff them against the
production branches to see exactly what the instrumentation adds.

## 2. The problem and how it was found

After the loading screen hides, Genesis Plaza on mobile stalled for hundreds of ms to seconds at a
time, mostly when content arrived (avatars, wearables, emotes, scene models). The method:

1. **Engine zones with asset names.** The fork's `core/profiling` already supports Perfetto/Tracy.
   The profiling branch adds zones that carry identifiers: resource paths, mesh/material RIDs, the
   shader/version/source of each pipeline compile, deferred calls by method name, AnimationMixer node
   paths, GPU readback entry points, worker-pool tasks/waits.
2. **Explorer zones.** A GDExtension interface (`profiler_zone_begin/end/is_enabled`) is resolved by
   Rust (`lib/src/tools/profiler.rs`) and exposed to GDScript as `DclProfiler`. Zones/marks name the
   scene, the scene-runner state, the avatar, the wearable/emote URN and hash, the GLB source.
3. **Capture.** `scripts/bench/profile_android_hiccups.sh --fresh --launch '<deeplink>'` records a
   system Perfetto trace (sched + the `godot` track_event category) from a clean install.
4. **Attribution.** `scripts/bench/hiccup_frames.py` charges every engine-thread slice to the nearest
   zone that names an asset and charges engine-thread *sleeps* to whatever the waking thread was
   doing (this is how lock waits behind loader threads were found). It prints the frames over a
   threshold with "name and surname" causes, and a per-asset ledger (`--ledger`).

### Root causes found (in order of impact)

1. **Inline pipeline compiles on the engine thread** (fork #2002 fix made them synchronous): ~1 s per
   new wearable on Mali, plus ubershader + specialized compiles at first draw.
2. **Waiting for SPIR-V of new scene shaders** on the engine thread (~170 ms each), and a global
   `SceneShaderForwardMobile::singleton_mutex` held by loader threads while *they* waited for their
   own SPIR-V (engine thread asleep 160–700 ms).
3. **Avatar/emote work on the main thread in bursts**: wearable instantiate, skin rebind, emote
   `_merge_animations`, AnimationTree restarts, ~20 avatars arriving in the same few frames.
4. **Asset-pack mounts**: every `load_resource_pack` re-parsed the global class cache and UID cache
   (~14 ms each, 200+ per load) and the queue drained in one frame.
5. **GPU readbacks** requested from tokio threads by the runtime GLTF import (`ImageTexture.get_image`)
   and executed on the engine thread at its next RenderingServer call.
6. **Scene runner**: one 8.3 ms tick budget *per physics step*, so a slow frame bought up to 8 budgets.
7. **Steady per-frame cost**: AnimationMixer of scene GLBs and avatars, FloatingIslands frustum math,
   SceneInspector serialization without a consumer, a ~100 ms memory-map walk every 1000 frames in
   metrics.
8. **iOS boot**: the explorer scene's GDScript tree compiled on the main thread (up to a 5 s hang).

## 3. What changed (production branches)

**Engine (`perf/hiccupless-loading`)** — settings live under
`rendering/rendering_device/pipeline_compilation/`, all default to today's behaviour:

| Setting | Android | iOS | Effect |
|---|---|---|---|
| `async_executor` | on | on | Dedicated `PipelineCompileThread`; draws fall back to the ubershader or skip until ready; no thread waits for a compile it isn't running; retired shader versions go to a graveyard drained at `_render_scene`. |
| `async_shader_variants` | on | on | A material whose SPIR-V is not ready is skipped and re-queued instead of waited on; loaders release the version mutex while waiting. |
| `max_new_shader_versions_per_frame` | 1 | 1 | Throttles new SPIR-V compile groups. |
| `shader_compile_threads` | 3 | 2 | Dedicated SPIR-V pool for forward-mobile scene shaders only. |
| `disable_ubershaders` | off | off | Specialized-only path (kept for A/B). |

Always on: `singleton_mutex` released before SPIR-V waits; `load_resource_pack` reloads the class/UID
caches only when the pack contains them; new `RenderingServer.instance_geometry_is_draw_ready()`;
`WorkerThreadPool::get_named_pool(name, thread_count)`.

**Explorer (`perf/hiccupless-loading`)**, five commits:

- *content*: wearable/emote `-mobile.zip` bundles are extracted off the engine thread instead of
  mounted (multi-entry zips keep mounting); runtime GLTF import decodes images on the CPU instead of
  reading them back; pack mounts drained within 6 ms per frame.
- *avatars*: `FrameWorkBudget` (one heavy main-thread step per frame / 2 ms after loading, 12 ms while
  loading; local player, previews and the GLTF pump first); lazy emotes for remote avatars; no
  AnimationTree restart on add; one comms avatar spawned per frame; change-only transform and
  AnimationTree writes; 4 closest avatars at full animation rate (was 8); remote avatars hidden until
  assembled and GPU-ready (dithered draw at alpha 0 warms their pipelines), then faded in whole;
  central nameplate occlusion; idle per-frame work disabled; social list fixes.
- *scene*: one scene-runner budget per rendered frame after loading; off-screen/far scene animators
  paused/throttled (colliders within 30 m keep full rate); FloatingIslands frustum once per frame;
  SceneInspector idle without a consumer; metrics memory sampled off-thread.
- *startup*: the lobby preloads `explorer.tscn` with `load_threaded_request`.
- *settings*: the table above + `GODOT_USE_BRANCH`.

## 4. Results

Cold start (clean install), 300 s, 10 rotating fillers + whoever was online (≈20 avatars), two runs per
cell, no profiler attached on iOS:

| | load | fps | frames >100 ms/min | >200 ms/min | worst frame |
|---|---:|---:|---:|---:|---:|
| A54 main | 31–32 s | 14–15 | 32–39 | 11–13 | 496 ms |
| A54 production branch | 20–21 s | 17.6–18.0 | 4–8 | 1.3–1.7 | 270–465 ms |
| iPhone 13 main | 64–85 s | 16–17 | 19–29 | 13–20 | ~500 ms |
| iPhone 13 production branch | 17–20 s | 27–30 | 3.5 | 0.9–2.1 | 298–414 ms |

Warm relaunch (caches filled): A54 main ~210 frames >100 ms/min → 2.3 with the changes (0 over 200 ms);
iPhone 17.4 → 0.4. iPhone memory stable at ~1.4 GB over 7.5 min with 30 avatars. 14 iPhone sessions
entering Genesis Plaza with the async settings on: all loaded, every main-thread stall recovered (the
#2002 inline-compile freeze scenario).

## 5. Measurement pitfalls (read before measuring)

- **Build Rust in release** (`cargo run -- build -r --target android`): the dev profile inflates
  every GDScript→Rust call (~18 µs per property read).
- **A profiler zone must not change behaviour**: an early zone called `ArrayMesh::get_rid()`, which
  creates the RID and moved every pipeline compile to the engine thread.
- **Never judge iOS memory with Instruments attached**: the AGX driver leaks Metal annotation strings
  (~2.5 MB/s) while `xctrace` records, and the app gets jetsam-killed after ~5 min. Use the app's own
  `[BenchStats]` lines without a profiler.
- **Android logd drops lines** from a chatty app; `[BenchStats]` is also written to
  `user://bench_stats.log` (`adb shell run-as org.decentraland.godotexplorer cat files/bench_stats.log`).
- **DynamicGraphics caps the A54 at 18 FPS** ("Very Low", warming up): steady frames are ~55 ms by
  design; measure work time (Perfetto, minus `OS::add_frame_delay`) or frames over 75/100/200 ms.
- **Device state**: let devices cool between runs (thermal status changes results); pin the Android
  device with `ANDROID_SERIAL`; check its Wi-Fi has internet (a run without network "loads nothing").
- **`realm-provider.decentraland.org` no longer resolves**: use `realm-provider-ea`.
- **iOS deep links on a cold launch can be lost**: pass `--skip-lobby --guest-profile --realm … --location …`
  as launch arguments instead.

## 6. Tools (profiling branch)

- Engine with zones (Android): `scons platform=android target=template_debug arch=arm64 profiler=perfetto
  profiler_path=<perfetto sdk> module_text_server_fb_enabled=yes debug_symbols=yes`, then point
  `.bin/godot_engine_config.json` at the fork and run `cargo run -- update-libgodot-android`.
  The CI build of the profiling branch has no profiler backend: zones compile to no-ops.
- Capture: `scripts/bench/profile_android_hiccups.sh --fresh --duration 300 --launch
  'decentraland://open?realm=https%3A%2F%2Frealm-provider-ea.decentraland.org%2Fmain&position=0%2C0&skip-lobby=true&guest-profile=true&bench-avatars=10'`.
- Reports: `hiccup_frames.py TRACE --after-marker LoadingScreen::hidden --threshold 20 --ledger out.txt`
  (named per-frame causes and per-asset ledger), `hiccup_attribution.py` (pipelines/emotes/process),
  `hiccup_report.py` (thread states, wakers). Python env with `perfetto` installed.
- <a id="benchmark-crowd"></a>**Benchmark crowd** (`godot/src/tools/bench_crowd.gd`, non-production):
  `--bench-avatars=N` / `bench-avatars=N` adds N walking avatars from
  `godot/assets/bench/filler_avatars.json` (30 diverse looks picked from recent catalyst profile
  deployments by `scripts/bench/fetch_filler_avatars.py`, deduplicated like profile-images, only the
  look is stored); one changes outfit every 2 s. `--bench-avatars=0` logs stats only;
  `--bench-still`, `--bench-pause-scenes` isolate costs. Prints `[BenchStats]` every 5 s.
- Deep-link test flags (non-production): `skip-lobby`, `guest-profile`; benchmark runs skip the OS
  notification prompt.

## 7. What to review harder

- **Engine concurrency** (`pipeline_compile_thread.cpp`, `pipeline_hash_map_rd.h`, `shader_rd.cpp`
  `_compile_wait_unlocked`, `scene_shader_forward_mobile.cpp`): graveyard lifetime (drained only when a
  3D scene renders), the duplicate-compile path for keys in flight, lock order version mutex →
  `compile_wait_mutex`, and the always-on `singleton_mutex` handoff (it also reaches desktop/iOS).
- **`instance_geometry_is_draw_ready()`**: shadow/depth skips and colour draws share one frame number;
  a pipeline that permanently fails reads as "never ready" (the explorer gate caps it at 3 s).
- **`FrameWorkBudget`** (`godot/src/logic/frame_work_budget.gd`): ordering/fairness, the `loading`
  flag (reset when there is no explorer), the priority given to the GLTF pump.
- **Emotes**: adding a clip no longer stops the AnimationTree (a former crash guard); verify emotes
  arriving mid-play and scene emotes.
- **Bundle extraction** (`lib/src/content/content_provider.rs`, `bundle.rs`): old caches, multi-entry
  zips, iOS file protection, disk-full.
- **Scene animation throttle** (`scene_animation_throttle.rs`): non-looping clips that advance while
  off-screen jump on return; animated platforms beyond 30 m.

## 8. QA checklist

- Cold and warm entry to Genesis Plaza and to a busy world on Android (low/mid/high tier) and iPhone;
  watch for avatars/models that never appear, or pop in later than ~1–3 s.
- Desktop (settings off) regression pass: rendering, materials, particles, UI.
- Crowd join (20+ avatars), outfit changes, emotes from the wheel and from scenes, emotes mid-play.
- Loading screen: progress labels keep updating; cancel loading back to Discover, then enter again.
- Social panel (nearby list) open/closed while people join and leave.
- Video players / streams start, resize and stop.
- Teleports and realm/world switches during loading.
- Low-memory devices: no new OOMs (compare memory with `[BenchStats]`).
- iOS: the #2002 Genesis Plaza entry on an iPhone 13; first launch after install.

## 9. Open items / ideas not done

- **18 FPS cap on low-tier Android** while DynamicGraphics warms up: with these changes the A54 has
  headroom; revisit when it downgrades and how fast it recovers.
- **Avatar animation cost** is the largest remaining in-world cost (AnimationMixer interpolation).
- **ParticleSystem GPU readback** in Genesis Plaza (`buffer_get_data`, up to 12 s per 130 s in one
  run): find the caller and keep it off the per-tick path.
- **iOS microhangs** (250–900 ms) after loading: animation, GDScript `_process`, Metal encoding.
- **Mesh readback** in the runtime GLTF import (`ResourceSaver` → `ArrayMesh::_get_surfaces`) needs an
  engine change (keep a CPU copy of surfaces while importing).
- **Decode glTF images lazily** per material texture (today all are decoded while holding the Godot
  permit, which delays pack mounts).
- `AvatarBuildProfiler` numbers are meaningless now that builds span frames.
