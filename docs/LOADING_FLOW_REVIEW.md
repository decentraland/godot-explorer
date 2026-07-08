# Scene Loading — Process & Flow Review

> **Issue:** [#1602 — Scene Loading Process + Flow Review](https://github.com/decentraland/godot-explorer/issues/1602)
> **Status:** DRAFT / living document. Sections marked `TODO(device)` need one more
> on-device capture pass (cold-cache + throttled network + a large scene). Everything
> else is settled from code reading + prior on-device profiling.
> **Companion docs:** [`LOADING_TDD.md`](./LOADING_TDD.md) (what to change),
> [`SCENE_LIFECYCLE_ROOT_CAUSE.md`](./SCENE_LIFECYCLE_ROOT_CAUSE.md) (why it hangs/breaks),
> [`CONTENT_LOADING_ARCHITECTURE.md`](./CONTENT_LOADING_ARCHITECTURE.md) (component reference).

This document covers issue #1602's first three deliverables:
1. Diagnose current loading-process technical performance
2. Document the current loading process (technical + user phasing)
3. Identify granular metrics + choke-points, dropout points, and common errors

The fourth deliverable (TDD) lives in [`LOADING_TDD.md`](./LOADING_TDD.md).

---

## 1. The loading process, end-to-end

### 1.1 Technical phases (`LoadingSession`)

The loading screen is driven by a Rust state machine, `LoadingSession`
(`lib/src/scene_runner/loading_session.rs`). It has 6 active phases, each with a
fixed weight that contributes to the 0–100% bar:

| Phase | Enum | Weight | Ratio driver | Source |
|------:|------|:------:|--------------|--------|
| Metadata | `LoadingPhase::Metadata` | 5% | `fetched / expected` scene entities | `loading_session.rs:136-137` |
| Spawning | `LoadingPhase::Spawning` | 5% | `spawned / expected` scenes | `loading_session.rs:140-141` |
| **Assets** | `LoadingPhase::Assets` | **60%** | `loaded / expected` GLTF assets | `loading_session.rs:154-169` |
| Ready | `LoadingPhase::Ready` | 15% | `ready / spawned` scenes | `loading_session.rs:172-174` |
| FloatingIslands | `LoadingPhase::FloatingIslands` | 15% | `created / expected` islands | `loading_session.rs:178-190` |
| Done | `LoadingPhase::Done` | — | 100%, dismiss screen | `loading_session.rs:127-130` |

Weights: `WEIGHT_METADATA/SPAWNING/ASSETS/READY/FLOATING_ISLANDS` (`loading_session.rs:~305-314`, must sum to 100).

### 1.2 Signal / data flow

```
scene_fetcher.gd  (start_loading_session / report_scene_fetched / floating islands)
      │
      ▼
SceneManager (Rust) ── owns ──► LoadingSession
  report_scene_spawned                    (scene_manager.rs:324)
  update_loading_session_from_scenes      (scene_manager.rs:718, per physics frame :1406)
      ← drains scene.gltf_loading_started_count / _finished_count
  emit_loading_progress → calculate_progress()   (loading_session.rs:123)
      │ signal loading_progress(percent, ready, total)   (scene_manager.rs:676)
      ▼
loading_screen_progress_logic.gd:66  _on_loading_progress()
      ▼
loading_screen.gd:103  set_progress()  →  Label_LoadingProgress / TextureProgressBar
      ▲
loading_complete (scene_manager.rs:667) → _on_loading_complete → set_progress(100)+hide
```

### 1.3 User-facing phasing

| User sees | Under the hood | Instrument event |
|-----------|----------------|------------------|
| Clicks teleport / walks into parcel | `begin_episode` / `loading.started` | `[LOADPROF] ev=episode.begin` |
| "Resolving realm" | `/about` fetch, realm change | `realm.about_fetch_start/end`, `realm.changed` |
| Bar climbs to ~25–30% then **sticks** | early-phase 30% cap + Assets-bucket gate (see §2) | `loading.progress pct=..` |
| Bar jumps 30→100% | 20s cap released + islands finish opens Assets bucket | `loading.progress` |
| Screen disappears | `LoadingPhase::Done` → `loading_complete` | `screen.hidden` |
| First hiccups after screen | GltfNodeModifiers burst + avatar impostor storm (see §4) | `[SCENEPROF]`, impostor warnings |

---

## 2. How the progress % is computed — and why it "hangs at 25–30%"

This is the headline symptom in #1602. It is **substantially a progress-computation
artifact, not (only) a stall.** Two mechanisms in `calculate_progress()`
(`loading_session.rs:123-214`) compound:

**(A) Hard 30% cap for the first 20 seconds.** `loading_session.rs:199-209`:
```rust
const EARLY_PHASE_DURATION_SECS = 20;   // EARLY_PHASE_MAX_PROGRESS = 30.0
let dampened = if elapsed_secs < 20 && phase != FloatingIslands && phase != Done {
    raw.min(30.0)   // capped at 30% for the first 20s
} else { raw };
```
For the first 20s the bar **cannot exceed 30%**, regardless of real progress. The
high-water-mark (`self.max_progress = max(dampened)`, `:212`) means it never falls back.

**(B) The 60% Assets bucket is gated to 0 until floating-islands finish + 5s.**
`loading_session.rs:146-168`:
```rust
let asset_discovery_ready = floating_islands_phase_start
    .map(|s| s.elapsed().as_secs() >= 5).unwrap_or(false);
let assets_ratio = if !metadata_complete || !asset_discovery_ready { 0.0 } else { ... };
```
`floating_islands_phase_start` is set **only** in `finish_floating_islands()`
(`loading_session.rs:292-297`). Until `_on_islands_generation_complete` fires
(`scene_fetcher.gd:748-749`), the whole 60% Assets weight contributes **0**. So the
reachable maximum before that point is `metadata 5 + spawning 5 + ready ≤15 +
islands ≤15 = ≤40%`, held to **≤30%** by the 20s cap.

**Result:** a Genesis-city load that has fetched + spawned + marked scenes ready, but
is still waiting on the gated Assets bucket, sits at **~25%** (the unit test at
`loading_session.rs:532-539` asserts exactly `5 + 5 + 15 = ~25%`), and the 20s cap
pins it ≤30%. **That is the plateau — by design.**

The plateau turns into a genuine *hang* when a real stall stacks on top (a stuck
GLTF or stalled island generation). See [`SCENE_LIFECYCLE_ROOT_CAUSE.md`](./SCENE_LIFECYCLE_ROOT_CAUSE.md).

> **Note:** `DclSceneNode.gltf_loading_count / max_gltf_loaded_count`
> (`dcl_scene_node.rs:13-14`, written in `update_scene.rs:328-331`) feed **only** the
> `[LOADPROF]` profiler (`loading_profiler.gd:237-238`), **not** the visible bar. The
> bar is 100% `LoadingSession`-driven.

---

## 3. Granular metrics & instrumentation

All instrumentation is **off by default** and flips on when the `LoadingProfiler`
autoload connects (it calls `SceneManager.set_scene_tick_profiling(true)`,
`loading_profiler.gd:148-149`). One global atomic gates all Rust channels
(`SCENEPROF_ENABLED`, `update_scene.rs:33`).

### 3.1 Instrumentation channels (inventory)

| Channel | Where | What it measures | Parser |
|---------|-------|------------------|--------|
| `[LOADPROF]` | `loading_profiler.gd` (GDScript autoload) | wall-clock event stream: realm→discovery→spawn→per-scene tick/GLTF→screen dismissal; download waves w/ mbps | `scripts/loadprof_report.py`, `loadprof_json.py`, `loadprof_viz.py` |
| `[LOADPROF-SUM]` | `loading_profiler.gd:359` | per-episode milestone summary (ms since begin) | `loadprof_report.py` |
| `[SCENEPROF]` | `update_scene.rs:46-68` | per-tick, per-component scene-update CPU (µs), heaviest-first | `scripts/sceneprof_report.py` |
| `[GLTFSYNC]` | `gltf_container.rs:248-260` | per-tick `sync_gltf_loading_state` cost: `loading_len`, `iterated`, `lookup_us`, `deferred_calls`, `deferred_us`, `done` | `scripts/gltfsync_report.py` |
| `[GLTFSTUCK]` **(new)** | `gltf_container.rs` | tail-report of entities stuck in `gltf_loading` (entity + src + Godot state), throttled `tick%60`, only when ≤8 remain | (grep) |
| `[DLPROF]` **(new)** | `resource_provider.rs` `fetch_resource` | per-asset download timing: `queue_us` (dedup+semaphore wait) vs `wire_us` (bytes on the wire) vs `bytes`, `cached` | (grep) |
| `STATE_TIMING` | `update_scene.rs:73-112` | aggregate per-state CPU for benchmark JSON (separate from `[SCENEPROF]`) | benchmark harness |
| `asset.download_speed` | `content_provider.rs:366-384` | aggregate download mbps, 60s history | `[LOADPROF] asset.download_progress mbps=` |

### 3.2 How to capture (device)

```bash
# 1. Full rebuild+export+install is REQUIRED for any Rust change — the installed
#    APK loads libdclgodot.so from INSIDE base.apk (extractNativeLibs=false), so
#    `--only-lib` hot-reload does NOT take effect. See handoff doc.
cargo run -- run --target android

# 2. Launch (am start hits a SecurityException; use monkey):
adb shell monkey -p org.decentraland.godotexplorer -c android.intent.category.LAUNCHER 1

# 3. Capture a single clean run to a file (no `| tail` — it buffers):
adb logcat -c && adb logcat > run.log        # then reproduce, Ctrl-C

# 4. Parse:
python3 scripts/loadprof_report.py  run.log
python3 scripts/sceneprof_report.py run.log --scene 0
python3 scripts/gltfsync_report.py  run.log --scene 0
grep '\[DLPROF\]'   run.log   # download timing
grep '\[GLTFSTUCK\]' run.log  # stuck-entity tail
```

`logcat` threadtime format: col3 = PID, col4 = **TID** (use `$4` for thread
attribution — the scene/main loop runs on one TID, worker pools on others).

---

## 4. Choke-points (ranked, with on-device numbers)

Measured on Samsung A54 (Mali-G68), Genesis Plaza, **warm cache**. `TODO(device)`
marks numbers still to be captured cold-cache / large-scene.

1. **`sync_gltf_loading_state` — per-tick O(N) re-scan (pull model).**
   Every scene tick, Rust iterates the entire `scene.gltf_loading` set, doing
   `ensure_node_3d` + `try_get_node_as::<DclGltfContainer>` + `.bind()...` per entity.
   - Genesis warm-cache: **SyncGltfContainer = 32%** of scene-update CPU; `[GLTFSYNC]`
     total ~4.5s over the load, `lookup_us` (the `try_get_node_as`) = **40.5%** of that
     (~25µs/entity), the rest is `.bind().get_dcl_gltf_loading_state()` + CRDT get/put +
     remove. ~61µs/entity/tick, scaling with `loading_len` (peak ~900 entities →
     ~23ms/tick just for the lookup).
   - `deferred_calls = 0` across the whole run → the `async_deferred_add_child` path is
     **dead** (`dcl_pending_node` is only ever null). Confirmed empirically.
   - Fix lever: pull→push + cache the `Gd<DclGltfContainer>` node ref. See TDD.

2. **Avatar impostor system — failed-retry storm (post-load).**
   After the loading screen dismisses, 93% of impostor warnings fire in bursts
   (+20–60s), 25 distinct avatar slots each regenerated 20–27× because the capture
   **fails**: `generate impostor mipmaps: ERR_UNAVAILABLE` (`avatar_scene.rs:611`) and
   `save impostor PNG: RGBA buffer too small: got 66976, expected 524288`
   (`avatar_scene.rs:1273`). Mipmap generation runs on the main/render threads, so it
   directly steals main-thread time → the sustained post-load hiccups. See
   [`SCENE_LIFECYCLE_ROOT_CAUSE.md`](./SCENE_LIFECYCLE_ROOT_CAUSE.md) FM-3.

3. **GltfNodeModifiers burst (post-load, brief).**
   Applying node-modifiers to just-finished GLTFs clusters at +18–29s; worst tick
   249ms (GltfNodeModifiers 129ms + GltfContainer 68ms). Bounded and early; scene CPU
   is otherwise flat (~17ms/tick) after the screen goes.

4. **Download pipeline concurrency — three stacked limiters.**
   Coordinator fetch pump (24, `gltf_loading_coordinator.gd:41`) → ResourceProvider
   `CappedSemaphore` (32, `content_provider.rs:263-269`) → (generic HTTP only:
   HttpQueueRequester 12). Source-dedup by content hash collapses N identical-asset
   loads to 1 download + 1 GPU upload (`gltf_loading_coordinator.gd`). Prior on-device
   win: Genesis queue_wait 42s→12s, screen 72s→55s. `TODO(device)`: confirm with
   `[DLPROF]` queue_us vs wire_us split, cold cache.
   > **Fixes [#2098](https://github.com/decentraland/godot-explorer/issues/2098)**
   > ("several GLTF instances with one unique source delay the loading queue"): before the
   > coordinator each of N instances of the same GLB took its own queue slot (MAX 10) and
   > re-downloaded the same bytes; now one `LoadGroup` per content hash shares one download
   > + one load across all waiters (`gltf_loading_coordinator.gd:78-123`), so N instances =
   > **one** slot. This is the branch's coordinator refactor — land it to close #2098.

---

## 5. Dropout points & common errors

| Failure | Mechanism | Safety net | Gap |
|---------|-----------|------------|-----|
| GLTF download never resolves | `ResourceProvider` uses `Client::new()` **with no timeout** (`resource_provider.rs:50`); stalled TCP hangs the future forever | GDScript 120s per-container coalescer → `_finish_with_error("timeout")` | 120s feels infinite; doesn't free Rust promise/slot |
| Hash "poisoned" after a failed download | `pending_downloads` notify leaked on failure (`resource_provider.rs:373-400` + error tail) → all future requesters of that hash block forever | none (only 120s coalescer per waiter) | permanent per-hash hang until app restart |
| Download pump saturated | coordinator holds a fetch slot across the whole await; 24 zombies stall all GLTF loading globally | none | whole-session stall |
| Scene never marked ready | needs `tick≥10 && gltf_loading.is_empty()` (`scene_manager.rs:758-760`); a stuck entity blocks it | per-scene 10s no-progress timeout marks ready anyway (`loading_session.rs:217-226`) | timeout marks ready but doesn't advance `loaded_assets` → session still stuck in Assets phase |
| Assets phase never completes (accounting) | delete-while-loading removes from `gltf_loading` but doesn't bump `gltf_loading_finished_count` (`deleted_entities.rs:48` vs `gltf_container.rs:68-69`) → `loaded < expected` forever | none | 60% bucket never reaches 1.0 → screen hangs w/ nothing actually stuck (ROOT_CAUSE §5.3, fix F-14) |
| **No destination / realm fail** | screen shown before any session; `/about` HTTP fail or corrupt JSON (`realm.gd:159-181`) / malformed `content` throw (`:269`) → `realm_changed` never fires → no session ever starts | `realm_change_failed` = **toast only**, doesn't hide screen; 90s/20s wall-clock modal | endless screen on teleport/join/startup; malformed-`content` case has no toast at all (ROOT_CAUSE **FM-4**, fixes F-10..F-13) |
| Whole loading screen never dismisses | `LoadingPhase` never reaches `Done`, or no session ever created | GDScript 90s wall-clock → "RUN ANYWAY" modal (`loading_screen.gd:141-168`) | user must manually bail; no global Rust watchdog |

Full root-cause analysis (with repro steps + chronology) in
[`SCENE_LIFECYCLE_ROOT_CAUSE.md`](./SCENE_LIFECYCLE_ROOT_CAUSE.md).

---

## 6. On-device capture results (2026-07-08 · Samsung A54 / Mali-G68)

Two capture passes on Genesis Plaza with the new instrumentation baked in
(`[DLPROF]`/`[GLTFSTUCK]`). Raw logs kept out of the repo; numbers below.

### 6.1 Warm-ish cache, good network (Run B/C)
- **SyncGltfContainer = 25.3%** of scene-update CPU (#1 component), p95 95.6ms, max
  231.6ms. TransformAndParent 14.0%, **Tween 12.6% (max 588.8ms** — confirms §5.4b),
  MeshRenderer 11.7%, GltfNodeModifiers 9.1%.
- **Peak `loading_len` = 1139 entities** simultaneously loading; ~99k entity-scans total;
  `[GLTFSYNC]` lookup = **45% of SyncGltfContainer** (~25µs/entity). `deferred_calls = 0`
  reconfirmed (dead path).
- Assets: 3292 added / 3179 downloaded / 1538 error.

### 6.2 Cold cache + 1 Mbps throttle (Run A) — the reported symptom, reproduced
- **Total load 97.7s**; `/about` ~1s; **first scene spawn at 27.6s**; **`first_ready=-1`,
  `scene_rendered=-1`** — *no scene ever reached ready/rendered*. The screen dismissed at
  97.7s (`reason=hidden`) **only via the 90s wall-clock timeout modal**, not real completion.
- **The 25–30% plateau, observed:** bar sat at **0% for the first 27.6s** (nothing until
  first spawn), then 10%→25%→30%, and **held at 30% from 35.5s to 60s (~25s in the band)**
  before moving to 40%. Total ~32s in the 25–30% band. Matches the reported bug exactly.
- **`[DLPROF]` (1294 real downloads, 196 MB):** `wire_us` p50 **6.8s** / p95 23s / **max
  198s**; `queue_us` p50 **17.9s** / p95 40.7s / max 51.8s. **Time split: wire 33% /
  queue 67%.** → On a constrained network, **two-thirds of per-asset time is queue-wait**,
  not the wire. The pump/semaphore can't help when the pipe is the bottleneck.
- **Timeout cascade:** **~734 GLTF assets errored `reason=timeout`** (the 120s coalescer
  expiring — per-asset queue+wire exceeded 120s; `max wire alone = 198s`), **0 network
  errors** (throttle only slowed, didn't cut). → "casi ningún modelo cargó": the world is
  degraded because hundreds of assets time out. Confirms FM-1 + the coalescer's limits.
- **`[GLTFSTUCK]` fired** and named the stuck asset: `entity=3743 state=LOADING
  src=…/head_wearables_female_emote.glb` — the instrument works.

### 6.3 Still open
- `TODO`: is the 30% early cap + Assets gate worth keeping? It makes healthy loads look
  stuck (and a throttled one indistinguishable from a hang). UX decision — see TDD F-7.
- The **queue-wait-dominates-on-constrained-network** finding (67%) elevates the priority
  of F-3 (coordinator slot mgmt) and the progressive-loading priority queue (TDD §3.1):
  on a slow pipe, *which* assets you spend the pipe on (critical-path first) matters most.
