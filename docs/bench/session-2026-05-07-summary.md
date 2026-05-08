# Session 2026-05-07 — Genesis Plaza performance work

Branch: `feat/rendering-server-animations-1948` (stacked on `spike/genesis-plaza-profiling-1862` PR #1992).
Device: Samsung A54 (Android 14, Mali-G68 GPU). Bench: `--gp-benchmark` against pinned-commit local preview.

## Headline

**14.48 FPS → 18.03 FPS (+24%)** on Genesis Plaza after three landed perf wins.
**30 FPS goal: not reached.** Architectural work needed (mesh merging or GPU-side profiling), out of scope for this session.

## What landed (commits on `feat/rendering-server-animations-1948`)

| SHA | Subject | Net effect |
|---|---|---|
| `28e7347c` | per-state CPU + CRDT throughput instrumentation (gated) | Telemetry only, gated by AtomicBool so cost is zero outside the sampling window. Foundation for the per-state breakdown that drove the next two fixes. |
| `4b35522f` | async impostor PNG save + TweenState emit-on-transition | Two CPU wins. **Async PNG**: moved zlib encode off the render thread (was 13.8% inclusive on VkThread); save runs on a tokio blocking worker. **TweenState dedup**: only emit on TsActive→TsPaused/TsCompleted transitions; cuts ~98% of TweenState dirty entries (15162 → 331/frame). Combined: ~+2.6 FPS. |
| `14b9f449` | `--inspect-scene-title` + render-manager getter wiring | Bench/debug ergonomics. Lets `launch_devices.sh --param inspect-scene-title="Genesis Plaza"` attach Chrome DevTools to a specific SDK7 isolate without UI interaction. |
| `82dd5ada` | docs(bench): post-async-png profile analysis + RAM hog inventory | `docs/bench/profile-deep-2026-05.md` updated with the post-fix top hogs, the bench-screenshot zlib artifact callout, and the Phase C ranking. |
| `6262c41e` | **CRDT recv split into wait + drain to reuse JS recv buffer** | The biggest single jump: `op_crdt_recv_from_renderer` was returning `Vec<Vec<u8>>` via `#[serde]`, allocating a fresh V8 BackingStore per inner Vec on every recv. Replaced with `op_crdt_recv_wait` (async, stashes framed `Vec<u8>` in op_state) + `op_crdt_recv_drain` (sync, copies into a JS-owned `Uint8Array` via `#[buffer] &mut [u8]`). JS keeps a 64 KB persistent buffer that grows on demand. **+3 FPS** measured on Samsung A54. |

Also on the spike branch (`spike/genesis-plaza-profiling-1862`, part of PR #1992):

| SHA | Subject |
|---|---|
| `bea4078c` | auto-clone + spawn pinned GP preview in `launch_devices.sh` |
| `02ab0674` | default `position=0,0` for gp-benchmark on fresh state |

## A/B summary (Samsung A54, release-template builds)

| Run | FPS mean | frame_proc_ms | Notes |
|---|---|---|---|
| baseline (HEAD revert) | 14.48 | 74.7 | pre-session state |
| async-png-2 (post first-pass fixes) | 17.09 | 63.9 | async-png + TweenState |
| no-frustum-cull (rebased clean) | 15.03 | 81.6 | pre-CRDT-split, with infra cleanup |
| **post CRDT recv split** | **18.03** | **69.3** | **shipping target** |
| post profile build (debug template) | 13.30 | 84.7 | 25 % overhead from debug template; not comparable to release |

Frame_proc dropped 75 → 69 ms; the rest of the win comes from V8 GC pressure releasing CPU back to the render thread.

## Profile-deep findings (post-CRDT-split, debug template, 18288 samples)

Output dir: `bench-results/profiles/android-post-crdt-clean-20260507T203511Z/`.

**Per-thread CPU (% of total CPU time):**

| Thread | % | What |
|---|---|---|
| VkThread (Java) | 53.7 % | Vulkan command building + Godot main update |
| Thread-14 (V8 isolate of GP scene) | 31.9 % | SDK7 JS execution + GC |
| mali-cmar-backend | 3.3 % | Mali driver back-end |
| Thread-24 (Choreographer/swappy) | 2.3 % | display sync |
| V8 worker pool (4 threads) | ~3 % combined | GC concurrent + shared isolate pool |

**VkThread top self-time excluding bench-screenshot zlib artifact:**

| % | Function |
|---|---|
| 5.8 % | `abs(double)` inline in transform/animation math |
| 4.2 % | aarch64 atomics (cas/ldadd) — refcount bumps on RID binds |
| 3.2 % | `Node3D::_propagate_transform_changed` |
| 2.2 % | `read` syscall |
| 1.3 % | `Node3D::get_global_transform` |
| 1.0 % | `__memcpy_aarch64_simd` |
| 0.9 % | `hal::halp::draw_template_internal::draw_build_command` |
| 0.6 % | `RenderingDeviceDriverVulkan::command_render_bind_vertex_buffers` |
| 0.6 % | `GodotBody3D::set_state` |

**Thread-14 inclusive (V8 isolate of GP scene):**

| % of Thread-14 | What |
|---|---|
| 29.0 % | `Builtins_CEntry…` (V8 → C++ runtime trampolines) |
| 24.1 % | `Builtin_ArrayBufferConstructor` + `ConstructBuffer` |
| 20.6 % | `Heap::CollectGarbage`, `Scavenge`, GC stack scans |
| 8.1 % | WeakMap (Ephemeron) write barriers |
| 5.0 % | `BackingStore::Allocate` |
| 3.7 % | scudo allocator atomic |

CRDT recv split removed the *outer* per-recv BackingStore allocation, but `Builtin_ArrayBufferConstructor` is still ~24 % of Thread-14 because (a) JS-side `slice()` per message in `decodeRecvFrame` still allocates one Uint8Array per message, and (b) SDK7 internals create more ArrayBuffers as it ingests CRDT messages. Further reduction would require SDK7-side changes (out of scope) or pushing dispatch into Rust.

## RAM accounting (Samsung A54, GP loaded)

| Bucket | MB | Notes |
|---|---|---|
| GPU textures (`texture_mem_mb`) | 600 | Wearables, GLTF scene textures, impostor texture array (~89 MB fixed cost from RGBA8 256×512×128 layers) |
| GPU mesh buffers | 46 | Vertex/index buffers |
| GPU video mem total (`video_mem_mb`) | 745 | GPU heap as reported by Godot |
| Godot static (`memory_static_mb`) | ~490 | CPU-side Object instances, shaders, scripts |
| Godot peak (`memory_peak_mb`) | ~494 | Single-frame max |

**Largest fixed allocation:** the impostor texture array at ~89 MB. Candidate for RGBA4444 + halving layer count → ~67 MB drop. No FPS benefit, RAM-only.

## Experiments that regressed (reverted, not in branch)

| Experiment | Result | Why we kept the data |
|---|---|---|
| `rs_gltf_direct=true` (Stage 1 RS-direct migration) | 18.03 → 8.46 FPS, even though node_count dropped 19425 → 12385 (-36 %) and draw_calls 1542 → 601 (-61 %) | frame_proc actually *improved* (69 → 66 ms). The 51 ms missing time is GPU-side — Mali-G68 + MultiMesh upload. Confirmed the architectural promise (CPU savings) but the GPU cost outweighs them on this device. **Needs GPU-side profiling (AGI) to debug — blocked by phone-side install (no physical access).** |
| AnimationMixer.active = false (frustum cull, take 2) | 18.03 → 14.20 FPS | `active` toggle is cheaper than `process_mode = DISABLED` in theory, but the coordinator's per-frame iteration over ~140 mixers + the active-flag flip cost more than the AnimationMixer.advance work it skipped. Same direction as the original frustum-cull regression. |
| Phase A — single Vec<u8> + framing (instead of Vec<Vec<u8>>) | broke GP loading (hung at 0 %) | Suspect SDK7-side reader doesn't handle `byteOffset != 0` on the Uint8Array slices we returned. Phase B (this branch's `wait + drain` split) sidesteps the issue by giving SDK7 the same shape it had before but with a reusable backing buffer. |
| RS-direct with poll loop removed | 18.03 → 4.16 FPS | Confirmed the poll loop wasn't the FPS regression source — without it FPS got *much* worse (transforms drift between scene_crdt and MultiMesh slot). The cost is GPU-side. |

## Bench infrastructure improvements (separate from FPS work)

These shipped quietly but materially improved the dev loop:

- **Auto-clone GP preview** (spike commit `bea4078c`): `launch_devices.sh --gp-benchmark` now clones `genesis_plaza_repo` at `genesis_plaza_commit` to `~/.cache/dcl-bench/Genesis-Plaza-2025-<short-sha>/` and spawns `npx sdk-commands start` if port 8000 is free. Removed the manual "make sure preview is running" step.
- **Default position=0,0 for gp-benchmark** (spike commit `02ab0674`): post-`pm clear` runs were hanging at "loading 0 %" because `cmd_location` stayed Vector2i.ZERO and `last_parcel_position` defaulted to (0,0). Default to `0,0` when `--gp-benchmark` is set without explicit `--position`.
- **`--profile` flag** (`28e7347c`): `launch_devices.sh --profile` spawns `profile_android.sh` / `profile_ios.sh` in parallel, each watching its device log for the `PROFILE_WINDOW_BEGIN duration_s=<N>` marker.
- **`profile_android.sh` symbolication**: extracts unstripped `libgodot_android.so` from the installed APK before symbolicating, so VkThread offsets resolve to function names.
- **Screenshot reorder**: `gp_benchmark_runner.gd:_save_screenshot()` now runs *after* `END_RESULT_JSON` is printed and the public-path mirror is written. No more bench-screenshot zlib polluting the profile window or risking lost JSON.
- **`adb install -r` instead of uninstall+install**: preserves app data dir between dev iterations → impostor cache + content cache survive, scene loading drops from 90 s to ~25 s on subsequent runs.

## What's left for a 30 FPS roadmap

The 30 FPS target needs **+12 FPS from 18**. Quick CPU wins are exhausted on this device. Remaining levers are architectural:

1. **GPU-side profiling (AGI)** — the rs-direct experiment showed CPU savings get eaten on Mali-G68, but we don't know *which part* of the pipeline (MultiMesh upload, shader cost, fill rate). AGI on the Samsung A54 is the unblocker. APK is already pushed at `/data/local/tmp/gapid.apk`; install is blocking from `pm install` over wireless adb (probably Samsung Knox / Auto Blocker), needs phone-side tap to install.
2. **Mesh merging at GLB-import time** — the user's preferred path. Reduces `MeshInstance3D` count + `draw_calls` directly, instead of relying on runtime MultiMesh batching. Sidesteps the rs-direct GPU cost surprise.
3. **`resolution_3d_scale` reduction** — if dropping render resolution to 0.75× lifts FPS materially, the bottleneck is fill-rate and the answer is texture compression / fewer transparent surfaces. One-line test, high information value.
4. **`DynamicGraphicsProfile = Low`** — disable shadows, MSAA, anisotropic. Test for whether graphics quality is the budget eater.
5. **Stage 3 PhysicsServer-direct colliders** — `StaticBody3D` (3568) + `CollisionShape3D` (3595) account for 35 % of node count. Migrating to RID-direct is a multi-week refactor.
6. **SDK7 system port to Rust** — `Builtins_CEntry` at 29 % of Thread-14 implies a lot of JS↔C++ trampolining inside the SDK7 dispatch. Porting the message router to Rust would cut this. Out of scope without SDK collaboration.

## Sticky notes for next session

- AGI install needs `Settings → Biometrics & security → Auto Blocker → off` on the Samsung A54, plus probably "Install unknown apps" enabled for the Files app. Without that, `pm install` from adb hangs silently with no error.
- `bench-cache/` (legacy local clone) and `~/.cache/dcl-bench/` (current) are both gitignored. The script uses the latter.
- `gp_benchmark_runner.gd` resets per-state timing + CRDT metrics at warmup→sampling boundary and drains at done. The drain output is in the bench JSON under `state_timing_us` and `crdt_metrics`.
- Last validated bench config: `--gp-benchmark`, `position=0,0`, debug template build (≈ 25 % slower than release), pinned commit `30cdaffd`.
