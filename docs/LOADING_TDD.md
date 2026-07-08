# Loading Pipeline — Technical Design Doc

> **Issue:** [#1602 — Scene Loading Process + Flow Review](https://github.com/decentraland/godot-explorer/issues/1602) (deliverable 4: TDD)
> **Status:** DRAFT. Fixes are grouped **Near-term (bug/perf)** → **Medium (architecture)**
> → **Progressive loading (the issue's proposal)**. Each item is a candidate GitHub issue
> (see §4). Effort is T-shirt sized; validate before committing.
> **Companion docs:** [`LOADING_FLOW_REVIEW.md`](./LOADING_FLOW_REVIEW.md) (diagnosis),
> [`SCENE_LIFECYCLE_ROOT_CAUSE.md`](./SCENE_LIFECYCLE_ROOT_CAUSE.md) (root causes).

---

## 1. Near-term fixes (correctness + perf)

### F-1 — HTTP timeout on the asset download path `[bug, critical]`
- **Problem:** `ResourceProvider` uses `Client::new()` with no timeout
  (`resource_provider.rs:50`); a stalled TCP connection hangs the download future
  forever → entity stuck in `gltf_loading` → infinite loading (RC-1).
- **Change:** build the client with `.timeout(...)` + `.connect_timeout(...)` (mirror
  `HttpQueueRequester`'s 60s, likely lower for assets). Ensure a timeout surfaces as
  `Err` so the promise rejects → `_finish_with_error`.
- **Why:** removes the primary network-bound infinite-loading cause; converts a 2-minute
  hang into a fast, retryable error.
- **Effort:** S. **Risk:** low (need to pick sane values; retries handled by F-3).

### F-2 — Fix `pending_downloads` notify leak `[bug, critical]`
- **Problem:** on download failure the `Arc<Notify>` in `pending_downloads` is never
  removed/fired (`resource_provider.rs:373-400` + success-only tail `:515-518`) →
  duplicate waiters hang forever + the hash is permanently poisoned (RC-2). Same in
  `fetch_resource_low_priority` / `fetch_resource_with_data`.
- **Change:** guarantee cleanup on **all** exit paths — RAII guard / `scopeguard`, or a
  `match` that removes+notifies before returning `Err`. Also close the lost-wakeup race
  (RC-3 / §3c in the root-cause map): hold the notify registration until the waiter is
  actually parked, or use a permit-storing primitive.
- **Why:** removes the "one failure poisons the asset until app restart" class.
- **Effort:** S–M. **Risk:** medium (concurrency-sensitive; add a targeted test).

### F-3 — Coordinator: release the fetch slot before the long await / zombie reaping `[bug]`
- **Problem:** `_async_download_group` holds one of 24 fetch slots across the whole
  Rust await (`gltf_loading_coordinator.gd:181-231`); zombie hangs (RC-1/RC-2) saturate
  the pump and stall all GLTF loading globally (RC-3).
- **Change:** cap in-flight time per hash (with F-1 in place, the await itself is
  bounded); optionally decouple the slot from the await, or reap slots whose await has
  exceeded a deadline.
- **Why:** prevents a few stalls from becoming a whole-session stall.
- **Effort:** S. **Risk:** low once F-1 lands.

### F-4 — `sync_gltf_loading_state`: pull → push `[perf]`
- **Problem:** Rust re-scans the entire `scene.gltf_loading` set every tick
  (O(N)/tick); measured 32% of Genesis scene-update CPU, ~61µs/entity/tick, ~23ms/tick
  at peak (LOADING_FLOW §4.1).
- **Change:** invert to push — the container notifies its state transition **once**
  (Finished/Error/NotFound) instead of Rust polling. E.g. a completion queue the
  container appends to, drained once per tick, so the per-tick cost is O(Δ) not O(N).
- **Why:** kills the single largest scene-load CPU choke-point; also shrinks FM-2.
- **Effort:** M. **Risk:** medium (touches the loading-state contract; keep the CRDT
  sync semantics identical).

### F-5 — Cache the `Gd<DclGltfContainer>` node ref per entity `[perf]`
- **Problem:** the per-entity `try_get_node_as::<DclGltfContainer>("GltfContainer")`
  lookup is 40.5% of SyncGltfContainer cost (~25µs/entity).
- **Change:** store the node handle when the container is created
  (`update_gltf_container`) and reuse it, invalidating on removal.
- **Why:** removes the 40% lookup even if F-4 is deferred; complementary to F-4.
- **Effort:** S. **Risk:** low (lifecycle care on removal/reparent — see UB-1).

### F-6 — Avatar impostor buffer-size mismatch + retry storm `[bug, perf]`
- **Problem:** impostor capture fails (`RGBA buffer too small: got 66976, expected
  524288`, `avatar_scene.rs:1273`; `generate impostor mipmaps: ERR_UNAVAILABLE`,
  `:611`), retried 20–27× per slot on the main/render thread → sustained post-load
  hiccups (UB-2).
- **Change:** size the capture buffer to match the expected `512×256×4`; on permanent
  failure, stop retrying (back off / mark slot failed) instead of hammering.
- **Why:** removes the dominant sustained post-load hiccup source.
- **Effort:** S–M. **Risk:** low.

### F-7 — Progress UX: the 25–30% plateau `[ux]`
- **Problem:** the 30% early cap (20s) + the Assets-bucket gate make a *healthy* load
  look frozen at 25–30% (LOADING_FLOW §2). Users read it as a hang.
- **Change (options, pick one):** (a) show a phase label / indeterminate spinner during
  the early-cap window instead of a stuck number; (b) let the bar reflect metadata/spawn
  progress honestly and drop the hard cap; (c) surface "still downloading (N/M, X mb/s)"
  which already exists (`loading_screen.gd:118-139`) more prominently.
- **Why:** the reported symptom is as much perception as stall; cheap UX win.
- **Effort:** S. **Risk:** low. Needs a product/design call.

---

## 2. Medium-term (architecture)

### F-8 — Lifecycle hardening (UB-1) `[bug]`
Tie freed-node cleanup to `NOTIFICATION_PREDELETE` for the avatar nameplate reparent
(`avatar.gd:539`) and the LOD coordinator (same antipattern). Removes the `DclUiControl`
access-after-free panic.

### F-9 — Loading-session honesty for stuck scenes `[bug]`
The per-scene 10s no-progress timeout marks a scene *ready* but does not advance
`loaded_assets`, so the session can stay in the Assets phase while the screen dismisses
(FM-1 safety-net gap). Reconcile: when force-marking ready, also reconcile the asset
accounting (or move the scene's expected assets to loaded) so the phase machine converges.

### F-10 — Wire realm-resolution failure → dismiss screen + error UI `[bug, critical]`
- **Problem:** on the dominant entry points (`async_teleport_to`, `async_join_world`,
  startup — all fire-and-forget), a `/about` failure emits `realm_change_failed` (toast
  only, `global.gd:1523`) but **never dismisses the loading screen** (FM-4 / RC-5, RC-9-/about).
  Only `_async_try_change_realm` (chat commands) hides on failure (`explorer.gd:968-970`).
- **Change:** connect `realm_change_failed` (and failed realm-resolution generally) to
  `_hide_loading_screen()` + a blocking "couldn't reach destination / retry" UI on ALL
  entry paths — not just chat commands.
- **Why:** removes the largest no-destination infinite-loading class; the 90s wall-clock
  modal is currently the only backstop and feels like a hang.
- **Effort:** S. **Risk:** low.

### F-11 — Guard malformed `/about` `content` `[bug]`
- **Problem:** a valid `/about` Dictionary with `"content": null` / missing `publicUrl`
  makes `ensure_ends_with_slash(null)` throw (`realm.gd:269,65`), aborting
  `async_set_realm` **after** partial state commit and **without** emitting
  `realm_change_failed` → silent stuck screen, no toast (FM-4 / RC-6).
- **Change:** validate `content.publicUrl` (and other required `/about` fields) before
  committing realm state; on missing/invalid → `_emit_realm_change_failed(...)`.
- **Why:** converts a silent hang into a surfaced, recoverable error.
- **Effort:** S. **Risk:** low.

### F-12 — Global loading watchdog + fix cached-empty-teleport dismissal `[bug]`
- **Problem:** (a) there is **no global Rust session/loading-screen timeout** — only a
  per-scene 10s post-spawn timeout that never fires if no session started
  (`loading_session.rs:92`). (b) Teleport to a previously-cached-empty parcel skips the
  coordinator request, so `coordinator_was_busy=false && is_reloading_now=true` makes the
  empty-set dismissal `loading_complete.emit(-1)` be **skipped** (FM-4 / RC-8,
  `scene_fetcher.gd:477-482`).
- **Change:** (a) add a global loading watchdog (or drop the 90s wall-clock to something
  humane); (b) fix the dismissal condition so a resolved-empty destination always emits
  completion regardless of coordinator busy-state.
- **Why:** guarantees the screen always resolves, even with no session.
- **Effort:** S–M. **Risk:** low–medium (don't dismiss a legitimately-slow load early).

### F-13 — `/about` retry + fallback realm `[robustness]`
- **Problem:** `async_set_realm` makes a single `/about` attempt (`realm.gd:154`), no
  retry, no failover (`DAO_SERVERS` is only used for genesis classification).
- **Change:** bounded retry with backoff on transient `/about` failure; optional fallback
  to a known-good realm on repeated failure.
- **Why:** transient network blips shouldn't strand the user.
- **Effort:** M. **Risk:** low.

### F-14 — Fix `gltf_loading` finished-counter on delete-while-loading `[bug]` — **CONFIRMED (RC-10)**
- **Problem:** deleting an entity mid-GLTF-load removes it from `gltf_loading`
  (`deleted_entities.rs:48`) but does **not** increment `gltf_loading_finished_count`
  (unlike component-removal at `gltf_container.rs:68-69`). `started > finished` permanently
  → `loaded_assets < expected_assets` → the Assets→Ready transition's `all_loaded`
  (`loading_session.rs:363-365`) is never true → the 60% Assets phase never completes →
  loading screen hangs (verified end-to-end, ROOT_CAUSE §5.3 RC-10). Accounting bug, not a
  stuck asset; no timeout escape in the Assets phase.
- **Change:** in `deleted_entities.rs:48`, mirror `gltf_container.rs:68-69` —
  `if scene.gltf_loading.remove(deleted_entity) { scene.gltf_loading_finished_count += 1 }`
  (or otherwise reconcile the session's `expected/loaded_assets` on delete).
- **Why:** removes a confirmed infinite-loading cause independent of RC-1..RC-9.
- **Effort:** S. **Risk:** low.

### F-15 — Retry failed/timed-out asset loads `[bug]` — **device-observed**
- **Problem:** once a GLTF load fails or hits the 120s coalescer, `_finish_with_error`
  marks it `FINISHED_WITH_ERROR` and it is **never retried** — even when the network fully
  recovers. Device-observed: cutting internet mid-load → assets error → on reconnect the
  models never reappear ("casi ningún modelo cargó"). Cold + 1 Mbps: **~734 assets timed
  out** (`reason=timeout`) and stayed missing; the world was permanently degraded until a
  full scene reload.
- **Change:** bounded retry with backoff on **transient** asset failure (network error /
  timeout), and/or re-trigger `FINISHED_WITH_ERROR` entities on connectivity recovery.
  Distinguish permanent (404/`NotFound`) from transient (timeout/network). Consider
  shortening/adapting the 120s coalescer (on a slow link it mass-fails; see FLOW §6.2).
- **Why:** transient blips currently degrade the world until a full scene reload — the
  worst-felt symptom in the device test.
- **Effort:** M. **Risk:** medium (cap attempts; avoid retry storms).

---

## 3. Progressive loading (issue #1602 proposal)

The issue proposes progressive/streaming scene loading. This is the **large** track;
the fixes above are prerequisites (you cannot stream reliably on top of a pipeline that
can hang). Design sketch, to be expanded into its own epic:

### 3.1 Asset priority system
- **Critical:** ground mesh, skybox, objects <5m.
- **High:** in-viewport 5–20m. **Medium:** off-viewport in-scene. **Low:** audio,
  decorative, distant.
- **Where:** a priority queue in front of the `GltfLoadingCoordinator` download pump
  (`gltf_loading_coordinator.gd`) + the ResourceProvider semaphore. The coordinator
  already prioritizes current-scene waiters (`_pop_next_download`,
  `gltf_loading_coordinator.gd:147-158`) — extend that to distance/viewport buckets.

### 3.2 LOD strategy
- Low-poly / placeholder geometry first; stream high-res via mip levels; defer audio.
- Interacts with the existing impostor/LOD system (`avatar/impostor/`) and optimized
  assets (`res://glbs/*.scn`).

### 3.3 Content-system changes (`lib/src/content/`)
- Priority queue for asset requests; bandwidth estimation (we already track
  `download_speed_mbs`, `content_provider.rs:366-384`); dependency tracking for the
  critical path.

### 3.4 Scene-runner integration (`lib/src/scene_runner/`)
- Notify a scene when an asset becomes available (aligns with F-4's push model); render a
  placeholder for a missing asset; emit progressive-load state events.

### 3.5 Considerations
- Cache eviction for partially-loaded scenes; chunked IPFS fetching; memory budget for
  concurrent LOD levels (cross-ref #2002); SDK7 scenes must tolerate missing assets.

---

## 4. Proposed issue breakdown (actionables)

| ID | Title | Track | Issue | Effort |
|----|-------|-------|-------|:------:|
| F-1 | ResourceProvider: add HTTP request/connect timeout | bug/critical | #1640 | S |
| F-2 | Fix `pending_downloads` notify leak + lost-wakeup race | bug/critical | #1640 | S–M |
| F-3 | Coordinator: bound/reap zombie fetch slots | bug | #1640 | S |
| F-4 | `sync_gltf_loading_state`: pull → push | perf | #1602 | M |
| F-5 | Cache `Gd<DclGltfContainer>` node ref per entity | perf | #1602 | S |
| F-6 | Fix impostor capture buffer size + stop retry storm | bug/perf | #1640 | S–M |
| F-7 | Loading-screen progress UX (25–30% plateau) | ux | #1602 | S |
| F-8 | Lifecycle: PREDELETE cleanup for nameplate/LOD | bug | #1640 | S |
| F-9 | Loading-session accounting for force-ready scenes | bug | #1602 | S |
| F-10 | Realm-resolution failure → dismiss screen + error UI | bug/critical | #1640 | S |
| F-11 | Guard malformed `/about` `content` | bug | #1640 | S |
| F-12 | Global loading watchdog + cached-empty-teleport dismissal | bug | #1640 | S–M |
| F-13 | `/about` retry + fallback realm | robustness | #1640 | M |
| F-14 | `gltf_loading` finished-counter on delete-while-loading | bug | #1640 | S |
| F-15 | Retry failed/timed-out asset loads (no permanent missing models) | bug | #1640 | M |
| — | Progressive/streaming loading (epic) | feature | #1602 | XL |

**Suggested order:** F-10/F-11 (no-destination hangs — cheap, high-impact) alongside
F-1 → F-2 → F-3 (asset-download infinite-loading class); then **F-15** (device-observed
no-retry / permanent missing models) and F-14/F-12 (accounting + watchdog), F-6/F-8
(post-load hiccups), F-4/F-5 (scene-load CPU), F-7/F-9/F-13 (UX/robustness), then the
progressive-loading epic on the hardened base.

> **Device-validated priority (2026-07-08, cold + 1 Mbps):** on a constrained link **67%
> of per-asset time is queue-wait** and hundreds of assets exceed the 120s coalescer
> (FLOW §6.2). This elevates F-3 (slot mgmt), F-15 (retry), and the progressive priority
> queue (§3.1): on a slow pipe, *which* assets you spend the pipe on — critical-path
> first — matters more than raw concurrency.
