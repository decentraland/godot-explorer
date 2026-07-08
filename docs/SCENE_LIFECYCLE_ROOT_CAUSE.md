# Scene Lifecycle Issues — Root-Cause Analysis

> **Issue:** [#1640 — Scene Lifecycle Issues: Investigation and Root Cause Analysis](https://github.com/decentraland/godot-explorer/issues/1640)
> **Status:** DRAFT / living document. Root causes for **infinite loading (FM-1)** are
> settled from code reading; **large-scene (FM-2)** and some **weird-behavior (FM-3)**
> items need the `TODO(device)` capture pass. The 5 technical suspicions from the issue
> thread are each assessed in §5.
> **Companion docs:** [`LOADING_FLOW_REVIEW.md`](./LOADING_FLOW_REVIEW.md),
> [`LOADING_TDD.md`](./LOADING_TDD.md).

Issue #1640 asks, for three failure categories, for: (1) a compact + precise cause,
(2) reliable reproduction steps, (3) a reproduction chronology, (4) findings that
enable a targeted fix — **without assuming solutions**. This doc is organized that way.

The three categories: **FM-1 Infinite scene loading**, **FM-2 Larger-scene loading
issues**, **FM-3 Scenes with unexpected behaviors**.

---

## FM-1 — Infinite scene loading

### Compact cause

A scene never finishes loading when at least one of its GLTF entities never leaves
`scene.gltf_loading` (the per-scene `HashSet<SceneEntityId>`, `scene.rs:219`). An entity
leaves that set **only** when its Godot-side `dcl_gltf_loading_state` flips to
`Finished` / `FinishedWithError` / `NotFound` (consumed by `sync_gltf_loading_state`,
`gltf_container.rs:210-214`). Four code paths can leave it stuck at `LOADING` forever;
all are bounded only by a 120s GDScript timeout — long enough to feel infinite — and one
of them isn't bounded at all.

### Root causes (ranked)

**RC-1 — No HTTP timeout on the asset download path.** `ResourceProvider`'s client is
`Client::new()` (`resource_provider.rs:50`) with **no request/read timeout**. Neither
`download_file` (`:195-271`) nor `download_file_with_buffer` (`:273-350`) sets
`.timeout(...)`. The `send()` (`:203`) and the `while let Some(chunk) = stream.next()`
loop (`:227`) can block indefinitely on a half-open TCP connection, a server that
accepts but never sends the body, or a stalled CDN edge → the future **never resolves
and never errors** → `dcl_gltf_loading_state` stays `LOADING` → entity stuck.
(Contrast: `HttpQueueRequester::process_request` sets `.timeout(60s)`,
`http_queue_requester.rs:190-195` — but the **asset path bypasses it entirely**.)

**RC-2 — `pending_downloads` notify leak → permanent hash poisoning.**
`handle_pending_download` (`resource_provider.rs:373-400`): the first caller of a hash
inserts an `Arc<Notify>` and returns `Ok`; subsequent callers block on
`notify.notified().await` (`:389-390`). The notify is removed + `notify_waiters()`
called **only on the success tail** of `fetch_resource` (`:515-518`). If
`download_file(...).await?` fails (`:493`), the `?` early-returns and the notify is
**never removed and never fired**: (a) every concurrent duplicate requester hangs
forever, and (b) the stale map entry **poisons the hash permanently** — every future
`fetch_resource` for that hash registers as a waiter and blocks forever, so retries can
never succeed. Same bug in `fetch_resource_low_priority` (`:528-573`) and
`fetch_resource_with_data` (`:576-618`).

**RC-3 — Coordinator download-slot held across the whole await (zombie slots).**
`_async_download_group` (`gltf_loading_coordinator.gd:181-231`) holds one of the 24
fetch slots for the entire `await` on the Rust promise. Any promise hung per RC-1/RC-2
is a zombie that never releases its slot. Once 24 hashes are stuck this way,
`_pump_downloads` (`:139`) can never start another download → **all** subsequent GLTF
loads stall globally. A handful of stalled connections becomes a whole-session hang.

**RC-4 — Realize/complete miss.** `_complete_shared_load` no-ops unless state is exactly
`LOADING` (`gltf_container.gd:152-155`); `_needs_realize` requires `LOADING &&
child_count==0` (`:108-109`). If a waiter is pruned/invalidated at the wrong moment
(`_prune_dead_waiters`, `_retire_group`), a container can stay `LOADING` with no further
callback. Narrow, but possible.

**Safety nets (and their limits):**
- **120s per-container coalescer** — `async_load_gltf` schedules a 120,000ms deadline
  (`gltf_container.gd:62`); on expiry `_on_load_timeout → _finish_with_error("timeout")`
  (`:458-459`). Bounds a single stuck GLTF's contribution to 120s. **Does NOT** free the
  Rust promise (RC-1), the leaked `pending_downloads` entry (RC-2), or the zombie
  coordinator slot (RC-3). And it lives in a child node's `_process`
  (`gltf_load_timeout_coalescer.gd`, moved off the autoload because `_process` there
  crashed Android with a VkThread NULL-deref) — if that node is missing/paused, the
  only bound on RC-1..RC-4 disappears and the entity is stuck **truly forever**.
- **Per-scene 10s no-progress timeout** (`loading_session.rs:217-226`, driven from
  `scene_manager.rs:688-715`) marks a stalled scene *ready* so the loading **screen**
  can dismiss — but it does **not** advance `loaded_assets`, so the session can stay in
  the Assets phase, and the entity stays in `gltf_loading`.
- **GDScript 90s wall-clock** (`loading_screen.gd:141-168`) → "RUN ANYWAY" modal.

### Reproduction steps

`TODO(device)` — capture with the new instrumentation baked in. Candidate repros:
1. **Slow/lossy network (RC-1):** throttle to ~2G / introduce packet loss (e.g.
   `adb shell tc` or a proxy), cold cache, teleport to Genesis. Expect the bar to sit in
   the 25–30% band; `[GLTFSTUCK]` should name the entity(ies) stuck at `state=LOADING`;
   `[DLPROF]` for the stuck hash shows a `wire_us` that never terminates (no `[DLPROF]`
   line at all for the hung hash — the log lands only on success).
2. **Forced download failure (RC-2):** point one asset hash at a URL that 5xx's or
   resets after headers, requested by ≥2 entities. Expect all requesters of that hash to
   hang even after the network recovers (poisoned map).
3. **Slot exhaustion (RC-3):** 24+ distinct slow/hung hashes → observe all *other* GLTF
   loads stop starting (no new `asset.gltf_download_begin`).

### Reproduction chronology (RC-1, expected)

```
t0    teleport → begin_episode, loading.started
t0+   scenes fetched + spawned → bar climbs to ~25% (capped)
t1    N GLTFs start → asset.gltf_start / asset.gltf_download_begin
t1+   most complete → asset.gltf_added; a few hashes on stalled TCP produce NO
      [DLPROF] line (wire never terminates)
t2    bar pinned ~25-30%; [GLTFSTUCK] every ~1-2s names entity=X state=LOADING src=…
t2+10s  per-scene 10s timeout marks scene ready → screen MAY dismiss, entity still stuck
t2+120s coalescer fires → _finish_with_error("timeout") → asset.gltf_error → entity
        finally leaves gltf_loading (but user waited 2 minutes)
```

### Reproduced on device (2026-07-08 · A54 · cold cache + 1 Mbps throttle)

A slow-but-not-cut link reproduced the failure cleanly (Genesis Plaza):
- **`~734` GLTF assets errored `reason=timeout`** (the 120s coalescer), **0 network
  errors** — the throttle only slowed the pipe; per-asset **queue+wire exceeded 120s**
  (`[DLPROF]` wire p50 6.8s / **max 198s**; queue p50 **17.9s** / max 51.8s; **67% of
  per-asset time was queue-wait**). Result: a degraded world, "casi ningún modelo cargó".
- **No scene ever reached ready/rendered** (`LOADPROF-SUM first_ready=-1 scene_rendered=-1`);
  the screen dismissed at **97.7s only via the 90s wall-clock modal** (`reason=hidden`),
  never via genuine completion — exactly the FM-1 + FM-4-backstop combination.
- **`[GLTFSTUCK]` fired** and named a stuck entity: `entity=3743 state=LOADING
  src=…/head_wearables_female_emote.glb`.
- **Refinement:** the tested failure was *slow* (queue/wire > 120s coalescer), not a
  *stalled TCP* (RC-1's zero-response case) — both reach the same user-visible outcome
  (timeout errors, no models), but RC-1's true hang is only bounded by the 120s coalescer
  while the slow-link case is bounded by whichever fires first (coalescer or the 90s
  wall-clock). Confirms the coalescer/wall-clock are the *only* backstops. See
  LOADING_FLOW_REVIEW §6.2 for the full numbers.

---

## FM-2 — Larger-scene loading issues

### Compact cause (hypotheses — `TODO(device)` to confirm)

Larger/complex scenes amplify two known cost centers:
1. **`sync_gltf_loading_state` O(N)/tick re-scan** (see LOADING_FLOW_REVIEW §4.1): cost
   scales with `loading_len`. A scene with hundreds of simultaneously-loading GLTF
   entities pays ~61µs × N every tick until they drain — at peak ~900 entities that is
   ~23ms/tick of pure lookup, competing with the 60fps physics budget. Whether this is
   linear or hits a cliff at large N is `TODO(device)` (test T2).
2. **Download-pump + semaphore saturation**: a large scene requesting many distinct
   hashes queues behind the 24-slot pump and 32-permit semaphore; combined with RC-1/RC-3
   a single large scene is more likely to accumulate zombie slots.
3. **Memory pressure** (cross-ref issue #2002 handoff): large scenes push GPU/RAM;
   on low-memory devices the graceful-memory path can evict mid-load.

### Reproduction steps / chronology

`TODO(device)` — pick a known-heavy scene, capture `[SCENEPROF]` + `[GLTFSYNC]` +
`[DLPROF]`, compare against a small scene. Watch peak `loading_len`, per-tick
SyncGltfContainer µs vs entity count, and download queue_us.

---

## FM-3 — Scenes with unexpected behaviors

Catalog of concrete, reproducible-but-hard-to-track behaviors found so far:

**UB-1 — `DclUiControl` "access after free" panic.** During `SceneManager::start_scene()`
(`get_player_avatar_node` / `get_base_ui`) a freed instance is accessed. This is the
pre-existing **avatar nameplate reparent lifecycle bug** (`avatar.gd:539`, nickname_ui
freed by a reparent; see memory `project_avatar_nameplate_reparent_lifecycle`).
godot-rust catches it and the scene loads anyway, but it's a latent lifecycle hazard and
the LOD coordinator has the same antipattern. Fix: tie the free to
`NOTIFICATION_PREDELETE`.

**UB-2 — Avatar impostor failed-retry storm** (also a perf choke-point, LOADING_FLOW §4.2).
25 avatar slots regenerate 20–27× each because the impostor capture **fails**:
`generate impostor mipmaps: ERR_UNAVAILABLE` (`avatar_scene.rs:611`) and `save impostor
PNG: RGBA buffer too small: got 66976, expected 524288` (`avatar_scene.rs:1273`). The
`66976 vs 524288` mismatch is a real buffer-sizing bug in the capture path — the
produced buffer is smaller than the expected `512×256×4` (=524288). Mipmap generation
runs on the main/render thread, so the retry storm directly causes post-load hiccups.
Fix: size the capture buffer correctly / stop retrying on permanent failure.

**UB-3 — first-4-frames gate not blocking (Mateo's observation).** See §5.5.

`TODO(device)`: add any other reproducible weird behaviors observed during the capture
pass (tween pops, entities at world-origin, etc.).

---

## FM-4 — No destination / realm-resolution failure (infinite loading, upstream of GLTF)

This is a **distinct, earlier** infinite-loading family from FM-1: here the hang is not a
GLTF stuck in `gltf_loading` — it is that **no scene / no `LoadingSession` ever comes into
existence**, so there is nothing to `Done`, so the loading screen never auto-dismisses.

### Compact cause (the structural gap)

The loading screen is shown **synchronously and unconditionally** by
`enable_loading_screen()` (`loading_screen.gd:40` → `loading_screen_progress_logic.gd:33`
`show()`) at the start of every teleport / join-world / startup — **before any scene is
known and before any `LoadingSession` exists.** The only *automatic* dismissal paths are
`_on_loading_complete` (`loading_screen_progress_logic.gd:70`) and `_on_loading_timeout`
(`:75`), both of which require a **`LoadingSession` to have been started and reached
`Done`/timeout**. The dominant entry points (`async_teleport_to` `global.gd:1348`,
`async_join_world` `:1369`, explorer startup `explorer.gd:201`) kick off the realm change
**fire-and-forget** and never wire a failure → hide. So any failure that prevents
`realm_changed` from firing (and thus prevents `scene_fetcher` from ever starting a
session) leaves the screen up with no session to complete it.

`realm_change_failed` **is** surfaced — but only as a non-blocking toast
(`global.gd:1523` "World unavailable"); it does **not** dismiss the loading screen. The
sole entry point that explicitly hides on failure is `_async_try_change_realm`
(`explorer.gd:968-970`, i.e. the chat `/goto realm` and `/changerealm` commands only).

### Root causes (per your hypotheses)

**RC-5 — `/about` fetch fails or returns corrupt JSON → CONFIRMED endless screen.**
`async_set_realm` (`realm.gd:142-285`) GETs `<realm>/about` (`:154`). On network error /
non-2xx / timeout (`res is PromiseError`, `:159`) or corrupt/non-Dictionary JSON
(`:177-181`) it calls `_emit_realm_change_failed` and returns — **`realm_changed` is never
emitted**, so `scene_fetcher._on_realm_changed` (`:499`) never runs, no session is ever
started, `loading_complete` can never fire. Recovery only via the 90s/20s modal or the
close button.

**RC-6 — `/about` valid Dictionary but malformed `content` → PLAUSIBLE silent stuck (worse).**
`realm.gd:269`:
```gdscript
content_base_url = Realm.ensure_ends_with_slash(realm_about.get("content", {}).get("publicUrl"))
```
If `/about` has `"content": null` or omits `publicUrl`, `.get("publicUrl")` is `null` and
`ensure_ends_with_slash(null)` (`:65`) calls `null.is_empty()` → **GDScript runtime error**.
This aborts `async_set_realm` *after* realm state was partially committed (`:184-186`) but
*before* `realm_changed.emit()` (`:284`) and *without* emitting `realm_change_failed`:
**no `realm_changed`, no `realm_change_failed`, no toast, no session** → silent stuck
screen. (Not defensively guarded.)

**RC-7 — Empty realm / no `sceneIds` → HANDLED for dismissal (mostly).**
A valid empty realm emits `realm_changed`; the coordinator resolves 0 scenes; because it
was busy fetching pointers/entities, `scene_fetcher.gd:477-482` emits
`loading_complete.emit(-1)` and the screen hides (user lands in an empty void, **no error
surfaced**). The stuck sub-case is the RC-6 runtime error above.

**RC-8 — Position/cityPointers with nothing there → HANDLED, with a PLAUSIBLE edge gap.**
Empty parcel resolves to `""` (`scene_entity_coordinator.rs:344-346,750`) and the
coordinator-was-busy branch dismisses (`scene_fetcher.gd:477-482`). **Edge gap:** teleport
to a **previously-cached-empty parcel in the same realm** — `update_position` issues no
request (`scene_entity_coordinator.rs:599-607`), so `coordinator_was_busy=false` while
`is_reloading_now=true` (`scene_fetcher.gd:823`); the dismissal condition
`(coordinator_was_busy or not is_reloading_now)` = `(false or not true)` = **false** →
`loading_complete.emit(-1)` is **skipped** → screen stuck until the 90s/20s modal.

**RC-9 — HTTP failure at later discovery steps → HANDLED for dismissal, silent wrong-destination.**
- Catalyst pointer lookup (`REQUEST_TYPE_SCENE_POINTERS`) fails: cleaned up
  (`scene_entity_coordinator.rs:617-627`), `is_busy()` false, coordinator-was-busy → screen
  hides, but the parcel is neither loadable nor marked empty → **silent** wrong destination.
- Scene-entity fetch (`REQUEST_TYPE_SCENE_DATA` for a `scenesUrn` hash) fails: definition
  never cached → never loadable → no per-scene session; screen hides but **the world's
  scene silently never loads**, no error surfaced.

### Why it hangs (and the only backstops)

- **No global session-level timeout in Rust.** Only a per-scene 10s no-progress timeout
  (`loading_session.rs:92`, `SCENE_TIMEOUT_SECS`) that applies to scenes already in
  `scene_last_progress` (populated on `report_scene_spawned`, `:240`). A session stuck in
  `Metadata` — or, worse, **never started** — never times out in Rust.
- **The only universal backstop is GDScript wall-clock:** `loading_screen.gd:144`
  `inactive > 20s or total > 90s` → "RELOAD / RUN ANYWAY" modal
  (`modal_manager.gd:882-891`). It fires even when no session ever started (the timer is
  keyed off `loading_show_requested`, not off a session). Plus the manual close button
  (`loading_screen.gd:296`).
- **No automatic fallback realm and no `/about` retry** — `async_set_realm` makes a single
  attempt (`realm.gd:154`); `DAO_SERVERS` (`:8`) is used only for genesis classification,
  not as failover.

### Bottom line

Endless-loading-with-no-destination is driven by **`/about`-step failures (RC-5, RC-9-/about)
and the malformed-`content` runtime error (RC-6)**, because those prevent
`realm_changed`/session creation while the dominant entry points leave the screen shown and
never wire failure → hide. Post-realm "0 scenes discovered" cases (RC-7, RC-8, RC-9-later)
are mostly self-dismissing, except the cached-empty-teleport edge (RC-8). In every
non-dismissing case the sole automatic recovery is the 90s/20s modal; the realm error is
surfaced only as a non-blocking toast.

### Reproduction steps (device/proxy)

`TODO(device)` — candidate repros:
- **RC-5:** proxy `<realm>/about` to return 500 / a truncated body / non-JSON. Teleport via
  the world menu (not chat). Expect the screen to sit until the ~90s modal; a "World
  unavailable" toast appears but the screen stays.
- **RC-6:** proxy `/about` to a valid JSON with `"content": null`. Expect a silent stuck
  screen with **no** toast (harder to notice).
- **RC-8 edge:** teleport to an empty parcel, return, teleport to the *same* empty parcel
  again (now cached empty). Expect the second teleport to hang until the modal.

### Fixes → see [`LOADING_TDD.md`](./LOADING_TDD.md) F-10..F-13

---

## 5. Technical suspicions from the issue thread (Mateo)

Each assessed **without assuming it is the cause** — status is one of
CONFIRMED / PLAUSIBLE / NEEDS-DATA / UNLIKELY.

### 5.1 Shared CRDT state — "is this the best approach? copy Bevy's?" → **SOUND** (+ doc correction)

The CRDT state is **one shared instance behind a `std::sync::Mutex`**, not a copy or
double-buffer: `SharedSceneCrdtState = Arc<Mutex<SceneCrdtState>>` (`dcl/mod.rs:109`),
created once and cloned into the scene thread (`dcl/mod.rs:166-180`). Two channels carry
only the **dirty index sets**, never component data (data lives in the shared state):
renderer→scene capacity-1 `mpsc` (`dcl/mod.rs:114,162`), scene→renderer `SyncSender`
(`:139`). The **scene/V8 thread** `lock().unwrap()`s to apply JS messages + `take_dirty`
(`js/engine.rs:180,192`); the **main/Godot thread** uses **`try_lock()`**
(`update_scene.rs:296`) — if the scene holds the lock it returns `false` and retries the
scene next frame (which increments `stuck_frames` → eventually `force_complete`).

**No data race and no lock-induced lost update** — the single mutex serializes all access
and the main thread never blocks; contention only *defers*. Verdict: **SOUND**. The
suspicion is not itself a bug.

> **Doc correction (important):** earlier drafts of this doc and
> `CONTENT_LOADING_ARCHITECTURE.md` §"Thread Safety" imply the CRDT sync is guarded by
> `GodotSingleThreadSafety`. **It is not.** `GodotSingleThreadSafety` is a Tokio semaphore
> that lives only under `lib/src/content/` (serializes *content-loading* threads that
> call Godot APIs) and is never referenced in `scene_runner/` or the CRDT path. The CRDT
> boundary is purely `Arc<Mutex<SceneCrdtState>>` + `try_lock` + the two dirty-set
> channels. A bevy-explorer comparison should start from that correct model.

### 5.2 Discard CRDT messages — "following expected behavior?" → **PLAUSIBLE (spec deviation CONFIRMED)**

LWW acceptance is **timestamp-strictly-greater only** (`last_write_wins.rs:61-79`):
`timestamp > entry.timestamp` accepts, older **or equal** drops (value not written, entity
not dirtied). There is **no value-length / lexicographic tie-break** anywhere in `crdt/`.
Other silent drops: stale entity version (`message.rs:65-67,87-89,114-116`), unknown
component id (`:71-77` + byte-level pre-filter `:354-421`), deser failure
(`last_write_wins.rs:108-115`). `GrowOnlySet` **ignores timestamp entirely**
(`grow_only_set.rs:56-66`) and is hard-capped at `APPEND_SIZE=100`, silently popping the
**oldest** on overflow (`:110-119`).

**Confirmed deviation from the SDK7 CRDT protocol:** SDK7 resolves ties as timestamp →
value-length → lexicographic; this implements **only** the timestamp step, so on an equal
timestamp the existing value always wins and the incoming is dropped. Impact is muted
because the state is a **single shared replica** (§5.1), not two replicas being
reconciled — the deviation only bites when scene and renderer land on the same timestamp
for the same component/entity (then the renderer's value wins by construction). Verdict:
**PLAUSIBLE** (deviation confirmed; desync impact `NEEDS-DATA`). Interacts with §5.4(b).

### 5.3 Create/delete entities — "same as Foundation?" → **SOUND core + PLAUSIBLE counter-skew bug**

Identity is `SceneEntityId { number: u16, version: u16 }` (`dcl/components/mod.rs:11-14`),
a 65536-slot version vec (`crdt/entity.rs:11-12`). **Numbers are reused, but every reuse
bumps `version`, so `(number, version)` is a fresh distinct key** across every map (LWW,
GOS, the Godot node map, `gltf_loading`). `try_init` drops writes for a stale version
(`entity.rs:42-43` → `message.rs:65` early-return). Consequences:
- **Stale in-flight CRDT message applied to a reused entity: PREVENTED** — the old
  `(5,0)` write fails `try_init` and is dropped; the new `(5,1)` is a different key.
- **Delete + recreate racing an in-flight GLTF: ISOLATED** — the `(5,0)` node is
  `queue_free`d and its async result inert; `(5,1)` gets a fresh node/container.

**CONFIRMED secondary bug — GLTF loading-counter imbalance on delete-while-loading (RC-10).**
When a GLTF **component** is removed from a *live* entity, both update:
`gltf_container.rs:68-69` (`if gltf_loading.remove(entity) { gltf_loading_finished_count += 1 }`).
But when the **entity itself** is deleted mid-load, `deleted_entities.rs:48` does
`gltf_loading.remove(deleted_entity)` **without** touching `gltf_loading_finished_count`
(or any session counter), while `gltf_loading_started_count` was already drained into the
session's `expected_assets` (`scene_manager.rs:747-749,783`). Verified end-to-end:
- `report_asset_loading_started` bumps `expected_assets[scene]` (`loading_session.rs:253-256`);
  `report_asset_loaded` bumps `loaded_assets[scene]` (`:244-247`) — only ever called from
  the `finished_count` drain (`scene_manager.rs:787-798`).
- The **Assets→Ready** transition requires **`all_loaded`** — *every* scene with
  `loaded_assets >= expected_assets` (`loading_session.rs:363-365`). A single leaked delete
  makes `loaded < expected` for that scene **permanently** (later loads bump both counters,
  so the gap never closes).
- The Assets phase has **no timeout escape** — the per-scene 10s timeout only populates
  `ready_scenes` (the *Ready* phase, which is never reached), and there is **no global
  session timeout**. So the session is stuck in Assets → never `Done` → the loading screen
  hangs until the GDScript 90s wall-clock modal.

This is a **distinct infinite-loading cause (RC-10)** from the download-path RC-1..RC-4 and
the realm-path RC-5..RC-9 — an accounting bug, not a stuck asset or a failed realm. It
bites any scene that deletes a GLTF entity while it is still loading (common in dynamic
scenes → overlaps FM-2 large / FM-3 weird). Verdict: **SOUND** for the reuse/race core;
**CONFIRMED** for the counter skew. Fix F-14.

### 5.4 Tween race conditions → **PLAUSIBLE (one genuine race)**

`update_tween` (`components/tween.rs:153`) runs in `SceneUpdateState::Tween`, ordered
**after** `DeletedEntities` and **before** `TransformAndParent` in the same locked tick
(`scene.rs:142-145`). Per active tween it writes the transform via `get_transform_mut().put`
(bumps timestamp +1, `tween.rs:506-508`) and marks TRANSFORM dirty.
- **(a) Tween vs GLTF finishing/attaching — MITIGATED.** The collider-mode window (tween
  starts before colliders exist) is covered by the **sticky** `_kinematic_requested`
  (`gltf_container.gd:330-334,382`): the flag persists and `set_mask_colliders` applies
  `KINEMATIC` at load completion. No lost kinematic switch.
- **(c) Tween vs entity deletion — SAFE.** `DeletedEntities` runs first
  (`scene.rs:142-143`), removes the entity from `scene.tweens` (`deleted_entities.rs:50`),
  and `take_dirty` stripped the died entity's LWW value → the tween loop skips it.
- **(b) Tween vs scene-authored Transform — PLAUSIBLE (the genuine race).** Both write the
  same TRANSFORM LWW entry. If the tween's `put()` advanced the timestamp past an incoming
  JS Transform message, the scene's transform is **silently dropped** (via the §5.2
  equal/older-timestamp drop); for **continuous** tween modes the tween reads the current
  CRDT transform as its accumulation base (`tween.rs:323-327,386,434`), so a JS write that
  *does* win shifts the tween base mid-flight → visible **pops/jumps** (FM-3 territory). No
  memory hazard, but a real behavioral race. Verdict: **PLAUSIBLE**; repro `NEEDS-DATA`
  (entity with an active tween whose scene also writes Transform, or a GLTF finishing
  mid-tween).

### 5.5 Scene lifecycle first-4-frames — "doesn't block at frame 4. Why?"

`CONFIRMED (mechanism understood).` The gate is `update_scene.rs:322-333`:
```rust
// tick 0 => onStart() => tick=1 => first onUpdate() => tick=2 => second onUpdate() => tick=3
if tick_number <= 3 && !scene.gltf_loading.is_empty() && !force_complete {
    sync_gltf_loading_state(...);
    ...
    return false;   // do NOT advance the tick past 3 while GLTFs still load
}
```
So the block is **conditional on `!scene.gltf_loading.is_empty()`**. It holds the scene
at tick ≤3 *only while GLTFs are still loading*. Reasons it can appear "not to block":
1. **`gltf_loading` empties early / never fills.** If the first-wave GLTF components
   haven't been created yet on ticks 0–3 (asset discovery is dynamic — GLTF entities
   register as the scene's CRDT is applied), the set is empty and the gate is a no-op;
   the scene sails past tick 3, then GLTFs start loading afterward with no gate.
2. **`force_complete` bypass.** The `stuck_frames` → `force_complete` mechanism
   (`scene.rs:299`, `update_scene.rs:324`) deliberately bypasses the gate to unblock a
   stuck scene thread. If a scene trips the stuck detector, the gate is skipped.
3. **The gate only covers ticks ≤3.** Once tick ≥4, new GLTF waves never re-engage it.
**Confirmed on device (2026-07-08 · A54 · Genesis cold load), `[GLTFSYNC]` scene 0:**
```
tick=0 loading_len=0      tick=1 loading_len=0     (GLTFs not registered yet)
tick=2 loading_len=741                              (GLTFs appear)
tick=3 loading_len≈730   ← ~11 sync calls at tick=3, loading_len stays ~730
tick=4 loading_len=727   ← advances to tick 4 with 727 GLTFs STILL loading
```
So the gate **does hold at tick 3** (the scene re-runs `sync_gltf_loading_state` and
`return false`s for ~11 iterations while `loading_len` stays ~730) — but it **releases at
tick 4 with ~727 GLTFs still loading**, because the condition is `tick_number <= 3`, *not*
"wait until `gltf_loading` is empty". This is exactly why Mateo saw "it doesn't block at
frame 4": the gate is only a **3-tick delay**, not a load barrier — the scene proceeds to
its `onUpdate` loop with the bulk of its GLTFs still in flight. Verdict stands: mechanism
understood; the "block" is working as coded, but as coded it is a brief delay, not the
"wait for first-wave assets" barrier one might expect.

---

## 6. Deliverable summary (per issue #1640 format)

| Failure | Cause (compact) | Repro | Chronology | Evidence |
|---------|-----------------|-------|-----------|----------|
| FM-1 infinite (asset) | stuck entity in `gltf_loading`; RC-1 no-timeout / RC-2 notify-leak / RC-3 slot-exhaustion; **+ counter-skew (§5.3)** | §FM-1 repro (device TODO) | §FM-1 chronology | code (`resource_provider.rs:50,373-400`), `[GLTFSTUCK]`, `[DLPROF]` |
| FM-4 infinite (no dest) | screen shown before any session; `/about` fail (RC-5) / malformed `content` (RC-6) / cached-empty teleport (RC-8) never dismiss | §FM-4 repro (proxy TODO) | §FM-4 | `realm.gd:159-181,269`, `scene_fetcher.gd:477-482`, agent trace |
| FM-2 large | O(N)/tick sync + pump saturation + memory | T2 device TODO | TODO | `[SCENEPROF]`, `[GLTFSYNC]`, #2002 |
| FM-3 weird | UB-1 nameplate free / UB-2 impostor buffer / UB-3 gate / §5.4(b) tween-transform pop | UB-2 reproduced (warm cache); others TODO | — | `avatar_scene.rs:611,1273`, `avatar.gd:539`, `tween.rs` |
| first-4-frames | gate is `!gltf_loading.is_empty()`-conditional; empties early / force_complete / tick≤3 only | device TODO | — | `update_scene.rs:322-333` |
| CRDT (5.1-5.4) | shared-state SOUND (doc corrected); LWW equal-ts tie-break deviation; entity reuse SOUND; tween-transform race | §5 | — | `dcl/mod.rs:109`, `last_write_wins.rs:61-79`, `entity.rs:33-86`, `tween.rs:503-522` |
