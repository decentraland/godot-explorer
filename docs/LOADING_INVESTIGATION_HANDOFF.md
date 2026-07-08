# Loading Investigation — Session Handoff

> **Purpose:** everything a fresh session needs to continue the scene-loading
> investigation for issues **#1602** and **#1640** without re-deriving anything. Read
> this + the three companion docs + the git diff vs `main` + the two GitHub issues, and
> you are fully in context.
> **Branch:** `test/loading-review` (all work UNCOMMITTED — see §7).
> **Last updated:** 2026-07-08 (after sessions 1–2: code audits + on-device capture pass).

## 0. Next-session kickoff prompt

> Paste this to resume in a clean context:
>
> _"Continuing the scene-loading investigation for godot-explorer issues #1602 (flow
> review) and #1640 (lifecycle root-cause), plus #2098. Read `docs/LOADING_INVESTIGATION_HANDOFF.md`
> first (it indexes the other 3 docs: LOADING_FLOW_REVIEW, SCENE_LIFECYCLE_ROOT_CAUSE,
> LOADING_TDD), then `git diff main` on branch `test/loading-review`. Diagnosis is COMPLETE
> — all failure modes (FM-1..FM-4, RC-1..RC-10), the CRDT audit, and the on-device capture
> pass (A54, cold+1Mbps: 25-30% plateau reproduced, DLPROF queue 67%, ~734 timeout errors,
> GLTFSTUCK working) are documented with real numbers in LOADING_FLOW_REVIEW §6. Fixes are
> catalogued F-1..F-15 in LOADING_TDD §4. The work now is SHIPPING: (1) open the F-1..F-15
> GitHub issues linked to #1602/#1640/#2098, starting with F-1/F-2/F-10/F-11/F-14/F-15;
> (2) post the review summary as comments on #1602 and #1640; (3) optional T6 proxy test;
> (4) decide on landing the GltfLoadingCoordinator refactor (closes #2098). Do NOT commit
> without explicit ask. Instrumentation ([DLPROF]/[GLTFSTUCK]/[GLTFSYNC]/[SCENEPROF]/[LOADPROF])
> is baked into the current A54 build and gated on the SceneManager.set_scene_tick_profiling
> toggle."_

## 0.1 How to get in context fast (read order)

1. This file (mission, state, gotchas, next steps).
2. [`LOADING_FLOW_REVIEW.md`](./LOADING_FLOW_REVIEW.md) — #1602: process, progress-%
   computation, choke-points, dropout map.
3. [`SCENE_LIFECYCLE_ROOT_CAUSE.md`](./SCENE_LIFECYCLE_ROOT_CAUSE.md) — #1640: infinite
   loading / large-scene / weird-behavior root causes + the 5 technical suspicions.
4. [`LOADING_TDD.md`](./LOADING_TDD.md) — proposed fixes F-1..F-9 + progressive-loading epic.
5. [`CONTENT_LOADING_ARCHITECTURE.md`](./CONTENT_LOADING_ARCHITECTURE.md) — pre-existing
   component reference (ContentProvider, ResourceProvider, GLTF loader, caching, threading).
6. `git diff main` — the actual instrumentation code.
7. GitHub issues [#1602](https://github.com/decentraland/godot-explorer/issues/1602),
   [#1640](https://github.com/decentraland/godot-explorer/issues/1640).

## 1. Mission

Two related open issues:
- **#1602 (enhancement/planning):** review the scene-loading process — diagnose
  performance, document the flow (technical + user), map choke-points/dropouts/errors,
  write a TDD, and (proposed) progressive loading.
- **#1640 (bug/research):** root-cause three failure modes — **infinite loading**,
  **larger-scene issues**, **unexpected behaviors** — with compact cause + repro steps +
  chronology, no assumed solutions. Plus 5 technical suspicions (shared CRDT, discard
  CRDT msgs, create/delete entities vs Foundation, tween races, first-4-frames gate).

**Plan (agreed with user):** document everything into the 3 docs above, add the missing
instrumentation, then spin off specific actionable GitHub issues (F-1..F-9). Do **one
more device capture pass** (cold-cache + throttled network + a large scene) to fill the
`TODO(device)` gaps. The two issues share one instrumentation harness and one capture
pass — run once, feed both.

## 2. TL;DR of findings (what we already know)

**The "hangs at 25–30%" (#1602 headline) is largely a progress-computation artifact.**
`LoadingSession::calculate_progress` (`lib/src/scene_runner/loading_session.rs:123-214`)
(a) hard-caps the bar at **30% for the first 20s** and (b) gates the **60% Assets bucket
to 0** until floating-islands finish + 5s. A healthy Genesis load therefore sits at ~25%
by design. It becomes a *real* hang when a stuck asset stacks on top. Details:
LOADING_FLOW_REVIEW §2.

**Infinite loading (#1640) has concrete root causes in the download path:**
- **RC-1:** `ResourceProvider` client has **no HTTP timeout** (`resource_provider.rs:50`)
  → stalled TCP never resolves.
- **RC-2:** `pending_downloads` notify is **leaked on failure** (`resource_provider.rs:373-400`)
  → the content hash is poisoned permanently.
- **RC-3:** the coordinator holds a fetch slot across the whole await
  (`gltf_loading_coordinator.gd:181-231`) → 24 zombies stall all loading.
- Only backstop is a **120s** per-container timeout coalescer (`gltf_container.gd:62`) —
  feels infinite and doesn't free the leaked resources. Details: SCENE_LIFECYCLE FM-1.
- **+ accounting bug — CONFIRMED (RC-10, SCENE_LIFECYCLE §5.3 / fix F-14):**
  delete-while-loading removes the entity from `gltf_loading` but doesn't bump
  `gltf_loading_finished_count` (`deleted_entities.rs:48` vs `gltf_container.rs:68-69`) →
  `loaded < expected` forever → the Assets→Ready `all_loaded` (`loading_session.rs:363-365`)
  is never true, and the Assets phase has no timeout escape → session never `Done` → hang
  with nothing actually stuck. Verified end-to-end this session.

**No-destination infinite loading (#1640 FM-4, user's hypothesis — CONFIRMED).** The
loading screen is shown *before* any `LoadingSession` exists; if `/about` fails / is
corrupt / has malformed `content` (`realm.gd:159-181,269`), `realm_changed` never fires →
no session ever starts → nothing can `Done` the screen. `realm_change_failed` is a toast
only; the sole backstop is the 90s/20s wall-clock modal. Fixes F-10..F-13. Details:
SCENE_LIFECYCLE FM-4.

**CRDT audit (T5) done — SCENE_LIFECYCLE §5:** shared CRDT state is **SOUND**
(`Arc<Mutex<SceneCrdtState>>` + non-blocking `try_lock`; **the guard is NOT
`GodotSingleThreadSafety`** — that's content-loading-only; earlier drafts +
CONTENT_LOADING_ARCHITECTURE mislabel this). LWW has a **confirmed SDK7 spec deviation**
(timestamp-only, no value-length tie-break on equal timestamp). Entity reuse is SOUND
(`(number,version)` keying). Tween-vs-scene-transform is a PLAUSIBLE FM-3 pop race.

**Post-load hiccups are mostly avatars, not the scene.** After the screen dismisses,
scene CPU collapses 8× (136ms→17.6ms/tick). The sustained churn is the **avatar impostor
retry storm**: 25 slots regenerated 20–27× because the capture fails (`RGBA buffer too
small: got 66976, expected 524288`, `avatar_scene.rs:1273`; mipmap `ERR_UNAVAILABLE`,
`:611`), on the main/render thread. A brief scene-side spike (+18–29s) is GltfNodeModifiers.

**Biggest scene-load CPU choke-point:** `sync_gltf_loading_state` is a per-tick O(N)
re-scan of `scene.gltf_loading` (pull model) = 32% of Genesis scene-update CPU, ~61µs/entity,
of which 40.5% is the `try_get_node_as` node lookup. `deferred_calls=0` proven (the
`async_deferred_add_child` path is dead). Fix levers F-4/F-5.

## 3. Instrumentation inventory (audited this session)

All Rust channels gate on one atomic `SCENEPROF_ENABLED`
(`update_scene.rs:33`), flipped on by the `LoadingProfiler` autoload calling
`SceneManager.set_scene_tick_profiling(true)` (`loading_profiler.gd:148-149`). So simply
running the app with the autoload present turns everything on.

| Channel | Location | Emits | Parser |
|---------|----------|-------|--------|
| `[LOADPROF]` / `[LOADPROF-SUM]` | `godot/src/logic/loading_profiler.gd` | wall-clock event stream + per-episode milestone summary | `scripts/loadprof_report.py`, `loadprof_json.py`, `loadprof_viz.py` |
| `[SCENEPROF]` | `lib/src/scene_runner/update_scene.rs:46-68` | per-tick per-component scene-update CPU (µs) | `scripts/sceneprof_report.py` |
| `[GLTFSYNC]` | `lib/src/scene_runner/components/gltf_container.rs:248-260` | per-tick sync cost: loading_len/iterated/lookup_us/deferred_calls/done | `scripts/gltfsync_report.py` |
| `[GLTFSTUCK]` **← added this session** | `gltf_container.rs` (after the sync loop) | stuck-entity tail: `scene tick entity state src`, throttled `tick%60`, only ≤8 remain | grep |
| `[DLPROF]` **← added this session** | `lib/src/content/resource_provider.rs` `fetch_resource` | per-asset `queue_us` / `wire_us` / `bytes` / `cached` | grep |
| `STATE_TIMING` | `update_scene.rs:73-112` | aggregate per-state CPU for benchmark JSON | benchmark harness |

Where the LOADPROF events come from (call sites, all in the diff):
`loading_profiler.gd` (autoload, polls scene + downloads), `gltf_container.gd`
(asset.gltf_start/instantiated/added/error), `gltf_loading_coordinator.gd`
(asset.gltf_download_begin/downloaded), `realm.gd` (about fetch), `scene_fetcher.gd`
(discovery), `loading_screen.gd` (screen.place_data/hidden), `global.gd`.

### 3.1 What the two NEW channels are for

- **`[GLTFSTUCK]`** answers #1640's "which asset hangs?" — when `gltf_loading` shrinks to
  a small tail that won't drain, it names each remaining entity + its `src` + Godot
  loading state every ~60 ticks. Cross-reference the entity/src against LOADPROF
  `asset.gltf_start` and `[DLPROF]`.
- **`[DLPROF]`** answers #1602/#1640's "network vs queue vs processing?" — brackets
  `fetch_resource` so you can tell a slow load apart: `queue_us` (dedup + semaphore wait)
  vs `wire_us` (actual bytes). A hung download per RC-1 produces **no** `[DLPROF]` line at
  all (the log only lands on success), which is itself the signal.

Both compile clean (`cargo check --features use_livekit,use_deno`, verified). They are
gated on the same profiling toggle → near-zero cost when off.

## 4. Device capture recipe

```bash
# FULL rebuild is REQUIRED for the Rust instrumentation (see §5 gotcha):
cargo run -- run --target android

# Launch (am start → SecurityException; use monkey):
adb shell monkey -p org.decentraland.godotexplorer -c android.intent.category.LAUNCHER 1

# Capture ONE clean run to a file (do NOT pipe to `tail` — it buffers to nothing):
adb logcat -c && adb logcat > run.log      # reproduce the scenario, then Ctrl-C

# Parse:
python3 scripts/loadprof_report.py  run.log
python3 scripts/sceneprof_report.py run.log --scene 0
python3 scripts/gltfsync_report.py  run.log --scene 0
grep '\[DLPROF\]'    run.log
grep '\[GLTFSTUCK\]' run.log
```
logcat threadtime: col3 = PID, **col4 = TID** (thread attribution — use `$4`, not `$3`).

## 5. CRITICAL deploy gotchas (learned the hard way)

1. **`--only-lib` hot-reload does NOT work on the installed APK.** It loads
   `libdclgodot.so` from **inside `base.apk`** (`extractNativeLibs=false`, verified via
   `/proc/<pid>/maps`). The `--only-lib` copy to `/data/data/<pkg>/` is never loaded.
   **Any Rust change needs a full `cargo run -- run --target android`** (build+export+install).
2. **`am start ...` → SecurityException** ("not exported from uid"). Launch with
   `adb shell monkey -p org.decentraland.godotexplorer -c android.intent.category.LAUNCHER 1`.
3. **Do not `| tail` a long-running build/logcat** — it buffers and the file stays empty.
   Redirect to a file, or stream without tail.
4. **Godot binary:** use `.bin/godot/godot4_bin` (the custom DCL fork), not system godot.
5. A pre-existing **`DclUiControl` access-after-free panic** during `start_scene` is the
   avatar nameplate reparent bug (UB-1), **not** caused by the instrumentation; godot-rust
   catches it and the scene loads anyway.
6. macOS: a rebuilt `libdclgodot.dylib` can SIGKILL Godot silently — fix with
   `codesign --force --sign - <dylib>`.

## 6. Pending work (device tests → deliverable)

| Test | Fills | Doc gap | Status |
|------|-------|---------|--------|
| **T1** cold-cache + 1 Mbps throttle | 25–30% plateau (held ~32s), `[DLPROF]` **queue 67% / wire 33%**, wire max 198s | FLOW §6.2 | **DONE** (A54, cold+1Mbps) |
| **T2** large scene (Genesis) | SyncGltfContainer **25.3%**, peak **loading_len 1139**, lookup 45% | FLOW §6.1 | **DONE** (Genesis = heavy) |
| **T3** force a hang (throttle) | ~734 `reason=timeout` errors, `[GLTFSTUCK]` named entity 3743, no scene ready | ROOT_CAUSE FM-1 | **DONE** (via T1 throttle) |
| **T4** verify first-4-frames gate | does the scene actually sit at `tick=3, loading_len>0`? | ROOT_CAUSE §5.5 | partial (analyze `[GLTFSYNC]` low-tick) |
| **T5** CRDT audit (code, no device) | suspicions 5.1–5.4 | ROOT_CAUSE §5 | **DONE** |
| **T6** no-destination / realm-fail (proxy) | RC-5/RC-6/RC-8 repros; screen hangs vs 90s modal | ROOT_CAUSE FM-4 | pending (proxy) |

**Done this session:** T1/T2/T3 device captures (A54, cold cache + 1 Mbps router throttle),
T5 CRDT audit, FM-4 code trace, F-14 verification. Headline device results in
LOADING_FLOW_REVIEW §6. Remaining: T4 (analyze existing `[GLTFSYNC]`), T6 (proxy for
malformed `/about`). **Key device finding:** on a constrained link, **queue-wait is 67% of
per-asset time** and hundreds of assets hit the 120s coalescer → world degrades, screen
dismissed only by the 90s wall-clock — never genuine completion (`first_ready=-1`).

## 7. Branch / diff state (UNCOMMITTED)

Branch `test/loading-review`, 1 wip commit (`df495012 wip instrumentation`) + a large
uncommitted working tree. **Do NOT commit or push without an explicit user request**
(standing constraint). Diff vs `main`:

```
 godot/project.godot                                    2 +
 godot/src/decentraland_components/gltf_container.gd   339 +/-   (coordinator refactor)
 godot/src/global.gd                                    3 +
 godot/src/logic/loading_profiler.gd                  389 +      (LOADPROF autoload)
 godot/src/logic/realm.gd / scene_fetcher.gd            3 +      (LOADPROF call sites)
 godot/src/ui/pages/loading_screen/loading_screen.gd    6 +      (screen.* events)
 lib/src/content/resource_provider.rs                  27 +      (← [DLPROF], this session)
 lib/src/scene_runner/components/gltf_container.rs      76 +/-   (GLTFSYNC + [GLTFSTUCK])
 lib/src/scene_runner/mod.rs                             2 +/-   (pub(crate) mod update_scene)
 lib/src/scene_runner/scene.rs                           8 +     (tick_prof field)
 lib/src/scene_runner/scene_manager.rs                   9 +     (set_scene_tick_profiling)
 lib/src/scene_runner/update_scene.rs                  62 +      (SCENEPROF + toggle)
 scripts/loadprof_{report,json,viz}.py                810 +
```
Untracked: `docs/LOADING_*.md` (the 3 docs + this handoff),
`godot/src/decentraland_components/gltf_loading_coordinator.gd` (+ `.uid`),
`scripts/{gltfsync,sceneprof}_report.py`.

**Nature of the changes:** almost all **instrumentation** (safe to keep), plus one
**functional refactor** — the `GltfLoadingCoordinator` (source-dedup GLTF loading,
`gltf_loading_coordinator.gd` + the `gltf_container.gd` rewrite). That refactor is a real
behavior change (prior on-device win: Genesis queue_wait 42s→12s, screen 72s→55s) and
**closes [#2098](https://github.com/decentraland/godot-explorer/issues/2098)** (N instances
of one GLB source no longer each take a queue slot — one `LoadGroup` per content hash). It
is a candidate to land on its own — evaluate separately (FLOW_REVIEW §4.4). The two Rust
instrumentation additions this session (`[GLTFSTUCK]`, `[DLPROF]`, + the
`pub(crate) mod update_scene` visibility fix) are isolated and compile-verified.

## 8. Next steps (recommended order)

**Done across sessions 1–2:** all code-only audits (T5 CRDT, FM-4 realm trace, F-14 verify)
**and** the on-device capture pass (T1/T2/T3 on A54 cold+1Mbps and warm; T4 from `[GLTFSYNC]`).
Every `TODO(device)` in the docs is filled — see LOADING_FLOW_REVIEW §6 for the numbers.
Diagnosis is complete; what remains is **shipping**:

1. **Open the F-1..F-15 issues** (LOADING_TDD §4) and link them from #1602 / #1640 / #2098.
   Suggested first batch (cheap + high-impact + device-validated): **F-1** (HTTP timeout),
   **F-2** (notify leak), **F-10/F-11** (no-destination hang), **F-14** (delete counter),
   **F-15** (asset retry). 
2. **Post the review summary** as a comment on #1602 and #1640; tick the scope boxes
   (#1602 boxes 1–3 done via the docs; box 4 = the TDD).
3. **T6 (optional):** proxy `/about` to malformed `content:null` to confirm RC-6 silently
   hangs (needs a host proxy; the app's reqwest ignores system proxy, so intercept at the
   realm `/about` GET — that one DOES go through the generic HTTP path, unlike assets).
4. **(Separate track)** decide whether to land the `GltfLoadingCoordinator` refactor — it
   **closes #2098** and is already in the branch.

## 9. Constraints (standing)

- **Never** `git commit` / `git push` autonomously — only on explicit request. Omit any
  `Co-Authored-By` trailer.
- Don't proactively edit READMEs / curated docs (this doc set is the exception — it was
  requested).
- Run `gdformat` / `gdlint` on the **entire** `godot/` folder, never single files.
- `cargo fmt --all` + `cargo clippy -- -D warnings` in `lib/` before any Rust commit.

## 10. Key file:line index

| What | Where |
|------|-------|
| Progress % formula (cap + gate) | `lib/src/scene_runner/loading_session.rs:123-214` (cap `:199-209`, gate `:146-168`) |
| `emit_loading_progress` | `lib/src/scene_runner/scene_manager.rs:672-685` |
| Scene "ready" condition | `scene_manager.rs:756-770` (`tick>=10 && gltf_loading.is_empty()`) |
| Per-scene 10s timeout | `loading_session.rs:217-226`, driven `scene_manager.rs:688-715` |
| `sync_gltf_loading_state` (O(N) sync + GLTFSYNC + GLTFSTUCK) | `lib/src/scene_runner/components/gltf_container.rs:142-` |
| `gltf_loading` add/remove sites | add `gltf_container.rs:87,116`; remove `:68,210-214`, `deleted_entities.rs:48`, `scene.rs:627` |
| First-4-frames gate | `lib/src/scene_runner/update_scene.rs:322-333` |
| No-timeout download client (RC-1) | `lib/src/content/resource_provider.rs:50` |
| `download_file` / `fetch_resource` (+DLPROF) | `resource_provider.rs:195-271` / `:479-` |
| notify leak (RC-2) | `resource_provider.rs:373-400` + success-only tail `:515-518` |
| Coordinator (RC-3, dedup) | `godot/src/decentraland_components/gltf_loading_coordinator.gd` |
| 120s timeout coalescer | `godot/src/decentraland_components/gltf_load_timeout_coalescer.gd`, scheduled `gltf_container.gd:62` |
| Impostor buffer bug (UB-2) | `lib/src/av/.../avatar_scene.rs:611` (mipmaps) `:1273` (PNG) |
| Nameplate free bug (UB-1) | `godot/src/decentraland_components/avatar/avatar.gd:539` |
| Loading screen UI | `godot/src/ui/pages/loading_screen/loading_screen.gd` + `loading_screen_progress_logic.gd` |

## 11. Related memories (auto-memory index)

`project_scene_tick_profiler`, `project_gltf_shared_loading`,
`project_avatar_nameplate_reparent_lifecycle`, `project_issue_2002_memory`,
`project_issue_2002_freeze_rootcause`, `feedback_never_autonomous_commits`,
`reference_godot_binary`, `feedback_gdformat_scope`.
