# Loading Investigation — GitHub Issue & Comment Drafts

> **Status:** DRAFT — nothing here has been posted/created on GitHub yet. This is the
> review file for the "shipping" step of the #1602/#1640/#2098 investigation.
> Once approved, each F-item below becomes one `gh issue create`; the three comment
> blocks become `gh issue comment` calls.
> **Source docs:** `LOADING_FLOW_REVIEW.md` (diagnosis §6 = device numbers),
> `SCENE_LIFECYCLE_ROOT_CAUSE.md` (root causes FM/RC), `LOADING_TDD.md` (fix catalogue §4).
> **Convention:** all created issues get the `claw-created` label (matches #1602).

---

## Part 0 — Existing related issues (do NOT duplicate)

| # | Title | Relation |
|---|-------|----------|
| **#2450** | [Bug] Loading Stuck at 25% Endlessly | **The user-facing symptom.** Root causes = F-1/F-2/F-14 + the 30%-cap progress artifact. Comment drafted in Part 2. |
| #2098 | Several Gltf instances with one unique source delays the loading queue | Closed by the `GltfLoadingCoordinator` refactor (source-dedup). F-3 also touches it. |
| #2128 | [Sentry] resource_provider.rs:619 ResourceLoader::load failed (6013 users) | Asset-load failure surface; related to F-15 (retry) + F-1/F-2. |
| #2144 / #2135 | [Sentry] glTF parser err families / sparse-indices mismatch | Permanent asset failures — F-15 must distinguish permanent (404/parse) from transient (timeout). |
| #2127 | scene_fetcher.gd entities_active_url empty (2954 users) | Realm/scene resolution — adjacent to F-10/F-11/F-13. |
| #1888 | Scene objects invisible until interaction | Overlaps FM-3 weird-behavior + possibly F-4 push model. |

---

## Part 1 — F-issue drafts (F-1 … F-15)

Each block below is one issue. Header line lists the **title**, **labels**, and the
**parent** issue to link. Bodies are copy-paste ready.

---

### F-1 — ResourceProvider: add HTTP request/connect timeout
- **Labels:** `bug`, `rust`, `mobile`, `claw-created`
- **Parent:** #1640 · **Effort:** S · **Risk:** low

**Problem.** `ResourceProvider`'s HTTP client is built with `Client::new()` and **no
request/read/connect timeout** (`lib/src/content/resource_provider.rs:50`). Neither
`download_file` (`:195-271`) nor `download_file_with_buffer` (`:273-350`) sets
`.timeout(...)`. `send()` and the `while let Some(chunk) = stream.next()` chunk loop can
block indefinitely on a half-open TCP connection, a server that accepts but never sends a
body, or a stalled CDN edge → the download future **never resolves and never errors** →
the entity's `dcl_gltf_loading_state` stays `LOADING` forever → infinite scene loading
(root cause **RC-1**). Contrast: `HttpQueueRequester::process_request` sets `.timeout(60s)`
— but the asset download path bypasses it entirely.

**Change.** Build the client with `.timeout(...)` + `.connect_timeout(...)` (mirror the
existing 60s in `HttpQueueRequester`; consider a lower value for assets). Ensure a timeout
surfaces as `Err` so the promise rejects and the GLTF container reaches
`FinishedWithError` (→ `_finish_with_error`) instead of hanging. Pairs with F-15 (retry on
transient failure).

**Why.** Removes the primary network-bound infinite-loading cause; converts a
feels-infinite hang into a fast, retryable error.

**Device evidence.** Cold cache + 1 Mbps: `[DLPROF]` max `wire_us` = **198s** on a single
asset; **~734 assets** ultimately errored `reason=timeout` at the 120s GDScript coalescer —
the download layer itself never gave up (FLOW §6.2).

---

### F-2 — Fix `pending_downloads` notify leak + lost-wakeup race
- **Labels:** `bug`, `rust`, `claw-created`
- **Parent:** #1640 · **Effort:** S–M · **Risk:** medium (concurrency — add a test)

**Problem.** `handle_pending_download` (`resource_provider.rs:373-400`) inserts an
`Arc<Notify>` for the first caller of a content hash; later callers block on
`notify.notified().await`. The notify is removed + `notify_waiters()` fired **only on the
success tail** of `fetch_resource` (`:515-518`). If `download_file(...).await?` fails
(`:493`), the `?` early-returns and the notify is **never removed and never fired** →
(a) every concurrent duplicate waiter hangs forever, and (b) the stale map entry **poisons
the hash permanently** — every future `fetch_resource` for that hash registers as a waiter
and blocks forever, so retries can never succeed (**RC-2**). Same defect in
`fetch_resource_low_priority` (`:528-573`) and `fetch_resource_with_data` (`:576-618`).
There is additionally a lost-wakeup window (**RC-3/§3c**) between registration and parking.

**Change.** Guarantee cleanup on **all** exit paths — RAII/`scopeguard`, or a `match` that
removes + `notify_waiters()` before returning `Err`. Close the lost-wakeup race by holding
the notify registration until the waiter is parked, or use a permit-storing primitive
(e.g. `Semaphore`/`tokio::sync::Notify` with stored permit semantics).

**Why.** Removes the "one failure poisons the asset until app restart" class — a strict
prerequisite for F-15 (retry can't work while the hash is poisoned).

---

### F-3 — Coordinator: bound/reap zombie fetch slots
- **Labels:** `bug`, `rust`, `performance`, `claw-created`
- **Parent:** #1640 (also touches #2098) · **Effort:** S · **Risk:** low (once F-1 lands)

**Problem.** `_async_download_group` (`gltf_loading_coordinator.gd:181-231`) holds one of
the 24 fetch slots for the entire `await` on the Rust promise. Any promise hung per
RC-1/RC-2 is a zombie that never releases its slot. Once 24 hashes are stuck, `_pump_downloads`
(`:139`) can never start another download → **all** subsequent GLTF loading stalls globally
(**RC-3**).

**Change.** Cap in-flight time per hash (with F-1 in place the await is already bounded);
optionally decouple the slot from the await, or reap slots whose await exceeds a deadline.

**Why.** Prevents a few stalls from becoming a whole-session hang.

**Device evidence.** Cold + 1 Mbps: **queue-wait = 67%** of per-asset time (`queue_us` p50
17.9s vs `wire_us` p50 6.8s) — on a constrained pipe the slot-holding model is a real
bottleneck, so slot hygiene matters (FLOW §6.2).

---

### F-4 — `sync_gltf_loading_state`: pull → push
- **Labels:** `performance`, `rust`, `claw-created`
- **Parent:** #1602 · **Effort:** M · **Risk:** medium (touches the loading-state contract)

**Problem.** Rust re-scans the entire `scene.gltf_loading` set every tick
(`gltf_container.rs:142-`, loop `:167`), O(N)/tick. Measured **25.3–32%** of Genesis
scene-update CPU, ~61µs/entity/tick, peak `loading_len` **1139 entities**, ~23ms/tick at
peak (FLOW §4.1 / §6.1). `deferred_calls = 0` confirmed on device — the
`async_deferred_add_child` path is dead.

**Change.** Invert to push: the container notifies its state transition **once**
(Finished/Error/NotFound) via a completion queue drained once per tick, so per-tick cost is
O(Δ) not O(N). Keep the CRDT sync semantics identical.

**Why.** Kills the single largest scene-load CPU choke-point; also shrinks FM-2 (large scene).

---

### F-5 — Cache `Gd<DclGltfContainer>` node ref per entity
- **Labels:** `performance`, `rust`, `claw-created`
- **Parent:** #1602 · **Effort:** S · **Risk:** low (lifecycle care on removal/reparent — cf. UB-1)

**Problem.** The per-entity `try_get_node_as::<DclGltfContainer>("GltfContainer")` lookup is
**~45% of SyncGltfContainer cost** on device (~25µs/entity; FLOW §6.1).

**Change.** Store the node handle when the container is created (`update_gltf_container`),
reuse it, invalidate on removal.

**Why.** Removes the 40–45% lookup even if F-4 is deferred; complementary to F-4.

---

### F-6 — Fix impostor capture buffer size + stop retry storm
- **Labels:** `bug`, `performance`, `rust`, `mobile`, `claw-created`
- **Parent:** #1640 · **Effort:** S–M · **Risk:** low

**Problem.** Avatar impostor capture fails — `RGBA buffer too small: got 66976, expected
524288` (`avatar_scene.rs:1273`) and `generate impostor mipmaps: ERR_UNAVAILABLE`
(`:611`) — and is retried **20–27× per slot** on the main/render thread → sustained
post-load hiccups (**UB-2**). After the loading screen dismisses, scene CPU collapses 8×
(136ms→17.6ms/tick) but the avatar retry storm keeps churning. Cross-ref Sentry #2138
(impostor_cache open failures).

**Change.** Size the capture buffer to the expected 512×256×4; on permanent failure, stop
retrying (back off / mark slot failed) instead of hammering.

**Why.** Removes the dominant sustained post-load hiccup source.

---

### F-7 — Loading-screen progress UX (25–30% plateau)
- **Labels:** `needs design`, `polish`, `mobile`, `claw-created`
- **Parent:** #1602 · **Effort:** S · **Risk:** low (product/design call)

**Problem.** `LoadingSession::calculate_progress` (`loading_session.rs:123-214`) hard-caps
the bar at **30% for the first 20s** (`:199-209`) and gates the 60% Assets bucket to 0
until floating-islands finish + 5s (`:146-168`). A **healthy** Genesis load therefore sits
at ~25–30% by design; users read it as frozen. **This is the reported symptom of #2450.**

**Device evidence.** Cold + 1 Mbps: bar sat at 0% for the first 27.6s (nothing until first
spawn), then held at **30% from 35.5s to 60s (~25s)** before moving — indistinguishable from
a hang (FLOW §6.2).

**Change (pick one).** (a) show a phase label / indeterminate spinner during the early-cap
window instead of a stuck number; (b) drop the hard cap and let the bar reflect
metadata/spawn progress honestly; (c) surface the existing "still downloading (N/M, X mb/s)"
(`loading_screen.gd:118-139`) more prominently.

**Why.** The reported symptom is as much perception as stall; cheap UX win.

---

### F-8 — Lifecycle: PREDELETE cleanup for nameplate / LOD coordinator
- **Labels:** `bug`, `claw-created`
- **Parent:** #1640 · **Effort:** S · **Risk:** low

**Problem.** The avatar nameplate reparent frees `nickname_ui` and can later access it
(`avatar.gd:539`, "previously freed"); the LOD coordinator has the same latent antipattern.
Manifests as a `DclUiControl` access-after-free panic during `start_scene` (**UB-1**);
godot-rust catches it and the scene loads anyway, but it's real UB.

**Change.** Tie freed-node cleanup to `NOTIFICATION_PREDELETE` for both sites.

**Why.** Removes the access-after-free panic. (Already captured in memory
`project_avatar_nameplate_reparent_lifecycle`.)

---

### F-9 — Loading-session accounting for force-ready scenes
- **Labels:** `bug`, `rust`, `claw-created`
- **Parent:** #1602 · **Effort:** S · **Risk:** low

**Problem.** The per-scene 10s no-progress timeout (`loading_session.rs:217-226`, driven
from `scene_manager.rs:688-715`) marks a stalled scene *ready* so the **screen** can
dismiss, but does **not** advance `loaded_assets` → the session can stay in the Assets
phase while the screen is already gone (FM-1 safety-net gap).

**Change.** When force-marking ready, reconcile the asset accounting (move the scene's
expected assets to loaded) so the phase machine converges.

**Why.** Keeps the progress model honest and prevents divergent screen/session state.

---

### F-10 — Realm-resolution failure → dismiss screen + error UI (all entry paths)
- **Labels:** `bug`, `mobile`, `claw-created`
- **Parent:** #1640 · **Effort:** S · **Risk:** low

**Problem.** On the dominant entry points (`async_teleport_to`, `async_join_world`,
startup — all fire-and-forget), an `/about` failure emits `realm_change_failed` (a **toast
only**, `global.gd:1523`) but **never dismisses the loading screen** (FM-4 / RC-5). Only
`_async_try_change_realm` (chat commands) hides on failure (`explorer.gd:968-970`). The
only backstop is the 90s wall-clock modal → feels infinite.

**Change.** Connect `realm_change_failed` (and failed realm-resolution generally) to
`_hide_loading_screen()` + a blocking "couldn't reach destination / retry" UI on **all**
entry paths, not just chat commands.

**Why.** Removes the largest no-destination infinite-loading class.

---

### F-11 — Guard malformed `/about` `content`
- **Labels:** `bug`, `claw-created`
- **Parent:** #1640 · **Effort:** S · **Risk:** low

**Problem.** A valid `/about` Dictionary with `"content": null` / missing `publicUrl` makes
`ensure_ends_with_slash(null)` throw (`realm.gd:269`, `:65`), aborting `async_set_realm`
**after** partial state commit and **without** emitting `realm_change_failed` → silent
stuck screen, no toast (FM-4 / RC-6).

**Change.** Validate `content.publicUrl` (and other required `/about` fields) before
committing realm state; on missing/invalid → `_emit_realm_change_failed(...)`.

**Why.** Converts a silent hang into a surfaced, recoverable error.

---

### F-12 — Global loading watchdog + cached-empty-teleport dismissal
- **Labels:** `bug`, `rust`, `claw-created`
- **Parent:** #1640 · **Effort:** S–M · **Risk:** low–medium

**Problem.** (a) There is **no global session/loading-screen timeout** — only a per-scene
10s post-spawn timeout that never fires if no session ever started (`loading_session.rs:92`);
the 90s wall-clock modal is the sole backstop. (b) Teleport to a previously-cached-empty
parcel skips the coordinator request, so `coordinator_was_busy=false && is_reloading_now=true`
makes the empty-set dismissal `loading_complete.emit(-1)` be **skipped**
(`scene_fetcher.gd:477-482`, RC-8).

**Change.** (a) Add a global loading watchdog (or bring the 90s wall-clock down to something
humane). (b) Fix the dismissal condition so a resolved-empty destination always emits
completion regardless of coordinator busy-state.

**Why.** Guarantees the screen always resolves, even with no session.

---

### F-13 — `/about` retry + fallback realm
- **Labels:** `bug`, `claw-created`
- **Parent:** #1640 · **Effort:** M · **Risk:** low

**Problem.** `async_set_realm` makes a single `/about` attempt (`realm.gd:154`), no retry,
no failover (`DAO_SERVERS` is only used for genesis classification).

**Change.** Bounded retry with backoff on transient `/about` failure; optional fallback to a
known-good realm on repeated failure.

**Why.** Transient network blips shouldn't strand the user.

---

### F-14 — `gltf_loading` finished-counter on delete-while-loading — **CONFIRMED (RC-10)**
- **Labels:** `bug`, `rust`, `claw-created`
- **Parent:** #1640 · **Effort:** S · **Risk:** low

**Problem.** Deleting an entity mid-GLTF-load removes it from `gltf_loading`
(`deleted_entities.rs:48`) but does **not** increment `gltf_loading_finished_count` —
unlike component-removal (`gltf_container.rs:68-69`) and normal completion (`:213-215`),
which both do. So `started > finished` **permanently** → `loaded_assets < expected_assets`
→ the Assets→Ready `all_loaded` transition (`loading_session.rs:363-365`) is never true →
the 60% Assets phase never completes → **loading screen hangs**, with nothing actually
stuck. Accounting bug, verified end-to-end (ROOT_CAUSE §5.3). The Assets phase has no
timeout escape.

**Change (one-liner).** In `deleted_entities.rs:48`, mirror `gltf_container.rs:68-69`:
```rust
if scene.gltf_loading.remove(deleted_entity) {
    scene.gltf_loading_finished_count += 1;
}
```

**Why.** Removes a **confirmed** infinite-loading cause independent of RC-1..RC-9. Smallest,
highest-confidence fix in the set → first PR candidate.

---

### F-15 — Retry failed/timed-out asset loads (no permanent missing models) — **device-observed**
- **Labels:** `bug`, `rust`, `mobile`, `claw-created`
- **Parent:** #1640 · **Effort:** M · **Risk:** medium (cap attempts; avoid retry storms)

**Problem.** Once a GLTF load fails or hits the 120s coalescer, `_finish_with_error` marks
it `FINISHED_WITH_ERROR` and it is **never retried** — even when the network fully recovers.
Device-observed: cutting internet mid-load → assets error → on reconnect the models never
reappear. Cold + 1 Mbps: **~734 assets** timed out (`reason=timeout`) and stayed missing;
the world was permanently degraded until a full scene reload (FLOW §6.2). Cross-ref Sentry
#2128 (6013 users).

**Change.** Bounded retry with backoff on **transient** asset failure (network / timeout),
and/or re-trigger `FINISHED_WITH_ERROR` entities on connectivity recovery. Distinguish
permanent (404 / `NotFound` / parse error — cf. #2144/#2135) from transient. Consider
shortening/adapting the 120s coalescer (on a slow link it mass-fails). Depends on F-2 (a
poisoned hash can't be retried).

**Why.** Transient blips currently degrade the world until a full scene reload — the
worst-felt symptom in the device test.

---

### (Epic) Progressive / streaming scene loading
- **Labels:** `enhancement`, `planning`, `claw-created`
- **Parent:** #1602 · **Effort:** XL

The #1602 proposal. Prerequisites = the fixes above (can't stream on a pipeline that can
hang). Design sketch in `LOADING_TDD.md` §3: asset priority system (critical/high/medium/low),
LOD strategy, content-system priority queue + bandwidth estimation, scene-runner push
integration (aligns with F-4). **Device finding elevates this:** on a constrained pipe 67%
of per-asset time is queue-wait, so *which* assets you spend the pipe on (critical-path
first) matters more than raw concurrency.

---

### F-16 — Loading telemetry → Segment (field data to drive the loading decisions)
- **Labels:** `enhancement`, `metrics`, `mobile`, `claw-created`
- **Parent:** #1602 · **Effort:** M · **Risk:** low

**Motivation.** The entire #1602/#1640 diagnosis currently rests on **one** device capture
(A54, cold + 1 Mbps). The open decisions — **F-7** (keep or drop the 30% cap), **F-3**
(queue-slot mgmt), **F-15** (asset retry), **F-1** (timeout tuning), **F-10..F-13**
(no-destination hangs) — need **field distributions across the fleet**, not a single run.
The `LoadingProfiler` autoload already **computes** exactly the milestones we'd want
(`[LOADPROF-SUM]`: first_spawn, first_ready, time-in-25–30%-band, queue-vs-wire split,
timeout count, wall-clock-dismissed vs real completion) — they're just logged locally. The
Segment pipeline already exists and was recently hardened (per-event timestamps / messageId
/ send-retry, #2440). This issue wires the two together.

**Proposed events (bucketed, no PII).**
- `scene_loading_started` — `{ realm, is_world, cold_cache, network_estimate_mbs, device_tier }`
- `scene_loading_completed` — `{ duration_ms, reached_ready:bool,
  dismissed_by: 'completion' | 'wall_clock_timeout' | 'error',
  time_in_25_30_band_ms, first_spawn_ms, first_ready_ms,
  phase_breakdown:{metadata,spawning,assets,ready,islands},
  assets_expected, assets_loaded, assets_errored, assets_timeout,
  queue_wait_fraction, wire_fraction, peak_loading_len }`
- `scene_loading_asset_failure` (**sampled/aggregated**) — `{ count, reason: timeout|network|notfound|parse }` for F-1/F-15.
- `realm_change_failed` — `{ reason }` to size the FM-4 no-destination class in the field (F-10..F-13).

**Decisions each field unblocks.**
| Property | Decision it drives |
|----------|--------------------|
| `time_in_25_30_band_ms` distribution + `reached_ready` vs `wall_clock_timeout` | **F-7** — is the plateau *perception* or a *real hang*, and how often |
| `assets_timeout` / `dismissed_by='error'` rates | **F-1**, **F-15** |
| `queue_wait_fraction` by `device_tier`/`network_estimate_mbs` | **F-3** + progressive priority queue (§3.1) |
| `realm_change_failed` volume by `reason` | **F-10..F-13** |

**Implementation notes.** Source the values from the `[LOADPROF-SUM]` milestone summary
(`loading_profiler.gd`); emit through the existing Segment analytics path; **bucket/round**
to avoid PII + high cardinality (no parcel coords, no wallet, no URLs — hash/bucket asset
sources); respect the existing analytics opt-in. Sample the high-frequency per-asset
failure event; keep the once-per-session funnel event unsampled.

**Why.** Converts a one-device anecdote into fleet data → the F-7 / F-3 / F-15 calls become
data-driven instead of guesses. **This is the prerequisite for "deciding just that."**

> **Splitting:** can ship as one issue, or split into **F-16a** (the core once-per-load
> `scene_loading_started`/`_completed` funnel — must-have for the F-7 decision) and **F-16b**
> (per-asset failure + `realm_change_failed` diagnostics — follow-up).

---

## Part 2 — Comment for #2450 ("Loading Stuck at 25% Endlessly")

> Root-cause comment. Links the diagnosis + fix issues. Does **not** close #2450.

---

**Root-cause analysis (from the #1602/#1640 loading investigation)**

This "stuck at 25%" has **two distinct contributors** — a perception artifact and several
genuine hangs. Reproduced on device (Samsung A54, cold cache + 1 Mbps throttle).

**1. The 25–30% number itself is partly by design.** `LoadingSession::calculate_progress`
hard-caps the bar at **30% for the first 20s** and gates the 60% "Assets" bucket to 0 until
floating-islands finish. A *healthy* Genesis load sits at ~25–30% for a while by
construction. On a slow link this is indistinguishable from a real hang — on device the bar
held at 30% for ~25s straight. → tracked as **F-7** (progress UX).

**2. Genuine infinite-loading causes** that make it *never* leave 25–30%:
- **No HTTP timeout** on the asset download path — a stalled connection hangs forever
  (RC-1). → **F-1**
- **`pending_downloads` notify leak** — one failed download poisons that asset's hash until
  app restart (RC-2). → **F-2**
- **Delete-while-loading accounting bug (CONFIRMED)** — deleting an entity mid-load never
  bumps the finished counter, so the Assets phase can never reach 100% and the screen hangs
  with nothing actually stuck (RC-10). → **F-14** (one-line fix)
- **No asset retry** — ~734 assets timed out on a throttled link and stayed permanently
  missing (F-15).

**On device (cold + 1 Mbps):** total load 97.7s, first scene spawn at 27.6s, **no scene ever
reached "ready"** (`first_ready=-1`) — the screen dismissed only via the 90s wall-clock
modal, never real completion. ~734 GLTF assets errored `reason=timeout`.

Fix issues: **F-1** (#TBD), **F-2** (#TBD), **F-7** (#TBD), **F-14** (#TBD), **F-15** (#TBD).
Full write-up in the branch docs (`LOADING_FLOW_REVIEW.md`, `SCENE_LIFECYCLE_ROOT_CAUSE.md`).

<!-- replace #TBD with the created issue numbers before posting -->

---

## Part 3 — Summary comment for #1602 (Scene Loading Process + Flow Review)

> Ticks the issue's deliverables; links the docs + spun-off fix issues.

---

**Loading-process review — complete.** Deliverables 1–4 done; write-ups on branch
`test/loading-review` (`docs/LOADING_FLOW_REVIEW.md` + `LOADING_TDD.md`).

**1. Technical performance diagnosed.** The loading bar is driven by a 6-phase Rust state
machine (`LoadingSession`): Metadata 5% · Spawning 5% · **Assets 60%** · Ready 15% ·
FloatingIslands 15%. Biggest CPU choke-point on scene load: `sync_gltf_loading_state` is a
per-tick O(N) re-scan of `gltf_loading` = **25–32% of scene-update CPU** (peak 1139 entities
loading at once; node lookup alone ~45% of that). → **F-4** (pull→push), **F-5** (cache node ref).

**2. Process documented** (technical phases + signal flow + user phasing) — FLOW §1.

**3. Choke-points / dropouts / errors mapped** — FLOW §2–6. Headline: the **"hangs at
25–30%"** is largely a *progress-computation artifact* (30% early cap + Assets-bucket gate),
which becomes a real hang when a stuck asset stacks on top. → **F-7** (UX).

**4. TDD written** — `LOADING_TDD.md`, fixes F-1…F-15 + a progressive-loading epic.

**On-device capture (A54, cold + 1 Mbps):** total 97.7s, first spawn 27.6s, **no scene ever
reached ready** (dismissed by the 90s wall-clock, not completion); 67% of per-asset time was
**queue-wait**, not wire; ~734 assets timed out. Full numbers FLOW §6.

**Spun-off issues:** F-4 (#TBD), F-5 (#TBD), F-7 (#TBD), F-9 (#TBD), progressive-loading epic
(#TBD). Cross-cutting infinite-loading fixes tracked on #1640.

<!-- replace #TBD before posting; check the deliverable checkboxes on the issue -->

---

## Part 4 — Summary comment for #1640 (Scene Lifecycle Issues: Root Cause)

> Compact cause + repro per the issue's ask, no assumed solutions in the body; links fixes.

---

**Root-cause analysis — complete.** Full write-up: `docs/SCENE_LIFECYCLE_ROOT_CAUSE.md`
(branch `test/loading-review`). Three failure modes + the 5 technical suspicions from the
thread, each with compact cause + repro + chronology.

**FM-1 — Infinite scene loading.** A scene never finishes when ≥1 GLTF entity never leaves
`scene.gltf_loading`. Ranked causes:
- **RC-1** no HTTP timeout on the asset path (`resource_provider.rs:50`) → stalled TCP never
  resolves. → **F-1**
- **RC-2** `pending_downloads` notify leaked on failure (`:373-400`) → hash poisoned
  permanently. → **F-2**
- **RC-3** coordinator holds a fetch slot across the whole await → 24 zombies stall all
  loading. → **F-3**
- **RC-10 (CONFIRMED)** delete-while-loading doesn't bump the finished counter
  (`deleted_entities.rs:48`) → Assets phase never completes → screen hangs with nothing
  stuck. → **F-14** (one-line fix)
- Only backstop = a 120s per-container coalescer — feels infinite, doesn't free the leaked
  resources. No retry after error → permanently missing models (**F-15**).

**FM-4 — No-destination infinite loading (CONFIRMED, matches the reporter's hypothesis).**
The loading screen shows *before* any `LoadingSession` exists; if `/about` fails / is
malformed (`content:null`), `realm_changed` never fires → no session → nothing can dismiss.
`realm_change_failed` is a toast only on the main entry paths. → **F-10** (dismiss + error
UI on all paths), **F-11** (guard malformed `/about`), **F-12** (global watchdog), **F-13**
(`/about` retry).

**FM-2 — Large-scene:** dominated by the O(N) `sync_gltf_loading_state` re-scan (→ F-4/F-5)
+ Tween spikes (max 588ms).

**FM-3 — Weird behavior:** tween-vs-scene-transform pop race (PLAUSIBLE); avatar impostor
retry storm (UB-2 → F-6); nameplate access-after-free (UB-1 → F-8).

**CRDT audit (suspicions 5.1–5.4):** shared CRDT state is **SOUND** (`Arc<Mutex>` +
non-blocking `try_lock`; the guard is **not** `GodotSingleThreadSafety`). Entity reuse is
SOUND. One **confirmed SDK7 spec deviation:** LWW is timestamp-only, no value-length
tie-break on equal timestamp.

**Repro (all FM-1 causes):** Samsung A54, cold cache + 1 Mbps router throttle, teleport to
Genesis Plaza → bar holds 25–30%, `first_ready=-1`, ~734 `reason=timeout`, `[GLTFSTUCK]`
named the stuck asset (`entity=3743 …head_wearables_female_emote.glb`).

**Fix issues:** F-1 (#TBD), F-2 (#TBD), F-3 (#TBD), F-6 (#TBD), F-8 (#TBD), F-10 (#TBD),
F-11 (#TBD), F-12 (#TBD), F-13 (#TBD), F-14 (#TBD), F-15 (#TBD).

<!-- replace #TBD before posting -->

---

## Part 5 — Suggested `gh` command sequence (for when approved)

```bash
# 1. Create issues (priority batch first), capturing numbers:
gh issue create --title "…F-1…" --label bug,rust,mobile,claw-created --body-file <(…)
# … repeat; record each returned issue number.

# 2. Back-fill the #TBD placeholders in Parts 2–4 with real numbers.

# 3. Post the comments:
gh issue comment 2450 --body-file part2.md
gh issue comment 1602 --body-file part3.md
gh issue comment 1640 --body-file part4.md
```

**Priority batch (per handoff):** F-14 → F-1 → F-2 → F-11 → F-10 → F-15, then the rest.
