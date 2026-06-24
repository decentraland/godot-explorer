# REVIEW.md

> **Purpose.** This file is the context a Claude reviewer reads before commenting on a PR in `decentraland/godot-explorer`. It should let a model with no prior knowledge of the repo produce a review that matches the tone, priorities, and depth established by the team.
>
> **Companion files.** Read `CLAUDE.md` (architecture, commands, tooling) before this. This file focuses on *what matters during review*, not how to build.
>
> **You are Claude Opus 4.7.** Be decisive. Front-load blockers. Cite file paths and PR numbers. Skip hedging when the codebase has a clear precedent — those are listed below.

---

## 0. Pre-flight — do this before reviewing

These are blocking prerequisites. Resolve each one (or explicitly note its status in your review) before reading the diff.

1. **Branch must be up to date with `main`.** Run `gh pr view <pr> --json mergeStateStatus,headRefOid,baseRefOid` (or check the PR page). If the PR is behind `main`, **request the author update the branch before a substantive review** — CI signals (especially Android/iOS artifacts) are not trustworthy on a stale base, and recently-merged fixes (autoload ordering, mouse-filter changes, skeleton recycling) frequently invalidate older diffs. A one-line "please rebase / merge `main` and I'll re-review" is the right output if the branch is stale.
2. **iOS build must be present for platform-sensitive changes.** Mobile builds are gated on the `build` label (alias: `build-ios`) and skipped by default. Check the PR's checks/comments for a `🍏 iOS` artifact in the sticky build report.
   - If the PR touches: native iOS plugins (`plugins/dcl-godot-ios/**`), `OS.get_name() == "iOS"` branches, deeplinks, virtual keyboard / safe-area / `UIView` paths, audio/video/livekit interop, or anything under `lib/src/comms/` or `lib/src/av/` → **an iOS build is required**.
   - If no iOS artifact exists, output exactly: *"No iOS build on this PR — a maintainer can add the `build` label to trigger one. I will not approve platform-sensitive iOS changes without a green iOS build."* and hold approval.
   - If the PR is purely backend / GDScript-with-no-platform-branch / docs, an iOS build is **not** required — call that out and proceed.
3. **Submodule pointer drift.** If `git diff main...HEAD` shows changes under `plugins/dcl-godot-ios/godot` or any submodule and the PR description does not mention it, treat it as accidental and ask the author to confirm.

If any of (1) or (2) fail and you proceed anyway, say so explicitly in the review header.

---

## 1. What this repo is

Decentraland's cross-platform metaverse client — the "Godot Explorer". Three languages collaborate inside one process:

| Layer | Language | Role |
|---|---|---|
| Engine / rendering / UI / scene tree | **Godot (custom fork)** + GDScript | `godot/` |
| Core systems (scene runner, comms, content, wallet, avatars, social) | **Rust** (compiled as GDExtension) | `lib/` |
| Decentraland SDK7 scene code at runtime | **JavaScript / V8 / deno_core** | per-scene threads driven by `lib/src/scene_runner` |
| `xtask` build system (doctor / install / build / run / export) | Rust | `src/` |

Target platforms: Linux, Windows, macOS, Android (API 29+), iOS, Meta Quest (OpenXR). The same binary ships to desktop and mobile — **any change has to be evaluated across touch *and* keyboard/mouse*, and on small screens as well as desktop.**

The engine is pinned to a custom Godot fork. The exact version lives in `project.godot` and is documented in `CLAUDE.md` — **do not hardcode the version into reviews**, and **do not suggest upgrading the engine** or using APIs that only exist on upstream Godot beyond the pinned version.

The project converges on visual and behavioral parity with Decentraland's **Unity Foundation Client**. Several fixes explicitly match the Unity implementation (e.g. camera FOV = 60°, avatar `rotation_y` wire convention, skybox GenesisSky port). If a PR cites Unity parity, treat Unity's behavior as the ground truth.

---

## 2. Where things live

Knowing which directory you're in usually tells you which language and which review lens applies.

```
lib/src/
├── dcl/                  DCL protocol types, SDK bindings, JS runtime glue
├── scene_runner/         Scene threads, CRDT handling, components, pointer events
│   └── components/       Per-SDK-component Rust systems (pointer_events, mesh_renderer, …)
├── comms/                LiveKit / WebRTC / voice / chat transports
├── content/              IPFS + content-server asset loading and caching
├── avatars/              Wearables, emotes, avatar assembly
├── profile/              Profile service, deploys
├── wallet/, auth/        Ethereum / sign-in
├── social/               Friends, blocks, mentions
├── analytics/, tools/    Telemetry, scene inspector, dev tools
└── godot_classes/        GDExtension binding definitions

godot/src/
├── decentraland_components/   GDScript mirrors of SDK components (gltf_container, avatar_attach, …)
├── ui/                        HUD, chat, notifications, dialogs, explorer scene
├── logic/                     Scene fetcher, realm, session, placement
├── tool/, tools/              Editor-only tools
└── global.gd                  Session-wide autoload (huge; touched by many PRs)

src/                    xtask commands (doctor, install, build, run, export, …)
docs/                   Architecture notes — scene-architecture.md is the best starting point
plugins/                Native iOS/Android plugins
```

A change inside `lib/src/scene_runner/components/*` almost always has a counterpart in `godot/src/decentraland_components/*` or vice versa. **If a PR changes one side only, ask whether the other side needs to follow** (and why it doesn't).

---

## 3. Review priorities — ranked

Apply this order. Everything below "Correctness" is negotiable; the top tier is not.

### Tier 1 — Blockers

1. **Crashes, hangs, and nil-access on autoloads.** `Global`, `DclGlobal`, `modal_manager`, `notifications_manager` come up frequently in review. Autoload `_ready()` order is load-bearing; the common fix is `call_deferred` (see #1874). If a PR adds an autoload or a new signal connection on one, verify the connected-to node exists by that frame.
2. **Decentraland SDK contract breaks.** Pointer/proximity events, CRDT state, component IDs, protocol field numbers, scene lifecycle (`SceneInit → OnStart → OnUpdate → SceneShutdown`). If a proto or component behavior changes, existing scenes in production must keep working — flag any breaking wire change.
3. **Debug prints / commented-out code / dead config left in.** `print("[DEBUG] …")`, `prints(…)`, `print_verbose` left behind, or orphaned `shader_parameter/foo` lines after a shader uniform is removed (see #1823, #1878). Cheap to flag, and the team consistently asks for it.
4. **Committed `.claude/` memory files.** Files under `.claude/projects/<someone>/memory/*.md` are per-contributor Claude Code state and do not belong in the repo. Call it out whenever it appears (precedent: #1852 existed just to remove them; #1823 was asked to clean them up). If the team eventually adds `.claude/` to `.gitignore`, this check becomes moot — flag that as a worthwhile follow-up if you see the pattern repeat.
5. **Changes to `project.godot` editor run args, local paths, or personal export presets.** e.g. flipping `--emulate-ios` ↔ `--emulate-android --landscape` is usually someone's local setting that leaked in (#1823). Hardcoded `/Users/<name>/…` absolute paths are always wrong (#1878).

### Tier 2 — Correctness

6. **UI tier structure compliance (Atomic Design — `godot/src/ui/`).** PR #2021 (issue [#1876](https://github.com/decentraland/godot-explorer/issues/1876)) reorganized `godot/src/ui/` into a 5-tier Atomic Design layout: `components/atoms/{buttons,inputs,images,controls}/`, `components/molecules/`, `components/organisms/`, `layouts/`, `pages/`. **Any PR that adds or moves UI files must land them in the correct tier.** Reject:
   - New files at the bare `components/` root (must be under `atoms/`, `molecules/`, or `organisms/`).
   - New feature-grouped folders like `components/profile/`, `components/discover/` — features are pages now (`pages/<feature>/`).
   - A new "Nth button variant" / duplicate input / duplicate toast — `godot/src/ui/COMPONENT_AUDIT.md` lists the 6 known duplication sets; ask the author to pick the canonical one or to file a follow-up issue rather than add another duplicate.
   - Hardcoded `res://src/ui/<old>/...` paths that survived a move (grep `lib/` too — Rust files like `lib/src/scene_runner/rpc_calls/handle_restricted_actions.rs` load UI scenes by string).
   - `.png` moves where the sibling `.import` file's `source_file=` still points at the old path.
   - Mixed-tier placement (e.g. a reusable molecule buried inside `pages/profile/` when it's used by other pages too).
   The skill `godot-ui-components` (in `.claude/skills/godot-ui-components/SKILL.md`) is the source of truth — it has the decision tree, naming conventions, and verification checklist. The audit doc `godot/src/ui/COMPONENT_AUDIT.md` is the lookup table for current placements and known duplicates.

7. **Cross-platform regressions.** Touch targets, gestures, virtual keyboard sync on Android/iOS, safe-area insets, landscape vs portrait. `DisplayServer.virtual_keyboard_show()` buffer sync after programmatic text insertion is a known class of bug (#1822). Godot `MOUSE_FILTER` behavior differs between `STOP` / `PASS` / `IGNORE` in non-obvious ways — siblings don't propagate (#1875).
7. **Mouse/input filter and focus stealing.** Buttons that steal focus from a `LineEdit`, containers with fixed `custom_minimum_size` that silently block scene UI underneath, `ScrollContainer` needing dynamic `mouse_filter` based on whether content overflows. Any new UI overlay on the left / bottom of the screen must be tested against SDK-rendered UI underneath.
8. **Async / race conditions.** Re-entrant `await` inside resize / rotation / teleport handlers (#1823 needed an `_is_switching` guard). Signals connected on a node that hasn't readied yet. Awaits that the caller doesn't `await` on (missing `await` on a coroutine is a real bug class here — see #1851).
9. **Resource leaks.** Godot does not auto-free bones, nodes outside the tree, or duplicated resources. Historical incident: spring-bone merge never recycled slots across outfit changes → unbounded `Skeleton3D` growth and stale bones silently binding to new meshes (#1849). When you see "add to skeleton / duplicate skin / instantiate on event", ask how it gets removed.
10. **Persistence.** Blocked users, friends state, profile deploys, per-user settings. Check that state written to disk survives a restart and that load happens before UI reads it (#1872 was an instance of this breaking).
11. **Log level discipline & Sentry quota.** Every log a PR *adds* has a cost, and the cost depends on its level. Error- and warning-level logs are routed to Godot's error stream and captured by the Sentry SDK in prod/staging builds — each one consumes the shared Sentry **event and attachment quota**, and a mis-leveled log on a hot path (scene-runner update loop, pointer events, per-entity/per-frame scans) can burst into *thousands* of events and exhaust the quota. The team already runs `_before_send` sampling and a `NOISE_PATTERNS` denylist (`godot/src/project_main_loop.gd`) precisely because over-reporting is a recurring problem — review new logs so that machinery doesn't have to. For **every** added log in the diff, ask: *is this an actionable fault a maintainer would want to see as a Sentry issue, or is it expected/recoverable noise?*
    - **Rust:** `tracing::error!` → Godot error → **Sentry issue**. `tracing::warn!` → Godot warning → Sentry (lower severity). `info!`/`debug!`/`trace!` → plain `godot_print!`, **never** Sentry (see `lib/src/tools/godot_logger.rs`). Debugging/diagnostic output belongs in `tracing::debug!`. Reserve `tracing::error!` for genuine faults we'd actually want paged on. A *missing texture / wearable / optional asset that already has a fallback* is **not** an error — it's a `warn!` (or `debug!` if routine) with a defined `else` branch.
    - **GDScript:** `push_error` / `printerr` → Godot error → **Sentry**. `push_warning` → warning. `print` / `prints` → console only, not Sentry. Same rule: don't `push_error` for expected-and-handled conditions; use `push_warning` or `print`, and reserve `push_error`/`printerr` for genuinely actionable failures.

    See Section 5 → "Logging discipline & Sentry quota" for the routing table and examples. Flag any new `error!`/`push_error`/`printerr` whose condition is expected, recoverable, or already handled by a fallback, and ask the author to downgrade it.

### Tier 3 — Quality

12. **Dev-only flags live in release builds.** Deep-link params like `fake-owned-wearables`, `disable-profile-deploy`, `dclenv=zone` parse unconditionally today. Acceptable but worth flagging for gating behind `#[cfg(debug_assertions)]` / a feature flag / a loud warning (#1849).
13. **Dead code / orphan uniforms / unused imports.** Rust `clippy -D warnings` catches most of this, but `.tres` / `.tscn` / `.gdshader` don't — reviewers catch those manually. A shader uniform removed in `.gdshader` should also be removed from every `.tres`/`.tscn` that set it, and from every material that references a different-typed replacement (#1878 had a `Texture2D → samplerCube` mismatch that would render black silently).
14. **Performance on the hot path.** The scene-runner update loop, pointer-event loop, and shaders are hot. Watch for per-pixel `acos`/`normalize`/`pow` that can be replaced by compares, per-frame `find_node` / `get_node` lookups, unbounded `for x in all_entities` scans inside scene systems, and JSON serialization on the scene thread.
15. **Test plan quality.** PR descriptions in this repo follow `## Summary` + `## Test plan` (bulleted checklist). A missing or vague test plan is a legitimate review comment, especially for UI changes. Mobile-visible changes should say *which* platform was tested on.
16. **Comments that explain "why", not "what".** Consistent with the CLAUDE.md guidance — reviewers flag comments that restate the code, and praise ones that cite a matching Unity file/line or explain a non-obvious Godot quirk.

---

## 4. Team conventions to uphold

### PR description shape
```
## Summary
- <bullet>
- <bullet>

Closes #<issue>

## Test plan
- [ ] <steps>
- [ ] <steps>
```
Larger PRs often add a "Root Cause" section before Summary, a Video/Images section after it, and a "Future plans" section at the end. Commit prefixes follow conventional commits: `feat:`, `fix:`, `chore:`, `refactor:`.

### Naming (from `.gdlintrc`)
- Classes / scenes / scripts: `PascalCase` (`ConnectionQualityMonitor`, `MentionItem`).
- Functions, variables, signals: `snake_case`. Signal handlers auto-named `_on_SomeNode_some_signal` (or `_on_` + `snake_case`).
- Constants: `SCREAMING_SNAKE_CASE`. Enums: `PascalCase` enum name with `SCREAMING_SNAKE_CASE` elements.
- Max file length: **1600 lines**, max public methods: **40**, max function args: **10**. `global.gd` / `notifications_manager.gd` are already large — new sprawl there gets pushback.

### Formatting / linting (must pass CI)
- Rust: `cd lib && cargo fmt --all && cargo clippy -- -D warnings`.
- GDScript: `gdformat godot/` and `gdlint godot/`. Use the `dcl-regenesislabs` fork of gdtoolkit — stock gdtoolkit 4 will produce spurious diffs.
- Asset imports: the project runs `tests/check_asset_imports.py`; lossless/`compress_mode` on imported textures matters. PRs that add PNGs should also commit the `.import` file.

### Validation
- Every GDScript file must pass `cargo run -- check-gdscript`. A script with a typo that only fails at runtime will pass CI — flag suspicious `get_node`/`$NodePath` references.
- `.tscn` files reference `.gd.uid` files; if a script is renamed or deleted, orphaned `.uid` files must go too.

### CI gates
The PR-level workflows a reviewer should expect green before approving:
- `📊 Static checks` — rustfmt + `gdformat -d` + `gdlint`
- `Clippy` — `-D warnings`
- `🐧 Linux`, `🪟 Windows`, `🍎 macOS` builds
- `🤖 Android` builds (APK/AAB posted as a sticky comment on the PR)
- `🍏 iOS` is **opt-in** — gated on the `build` label (alias: `build-ios`), which also posts a Slack "Android build ready" notification with the R2 APK download link. See Section 0 pre-flight: for platform-sensitive changes the iOS build is *required* and the PR should be held until a maintainer adds the label. For pure-backend / docs PRs, an absent iOS build is fine — say so explicitly.

### Release flow
`release` branch is used for production cuts. PRs titled `Release: merge release into main` / `Merge main into release` appear periodically and should usually be merge-only (no review nits on code that's already been reviewed upstream).

---

## 5. Recurring patterns to recognize

These come up in almost every review in the history. Knowing them saves you from re-deriving them.

### `call_deferred` for autoload signal wiring
Autoloads ready in a fixed order (`Global` first). A new autoload that connects to `Global.modal_manager.something` in `_ready()` will crash if it readies before `modal_manager` is built. Fix is `call_deferred("_connect_signals")` — see #1874.

### `mouse_filter` is per-node; `PASS` does not fan out to siblings
If an overlay (chat, notifications, modal) blocks underlying scene UI, the culprit is usually a `Control` with `MOUSE_FILTER_STOP` that's in the hit-test tree even when empty. Fixes: collapse its size to 0 when empty, set `MOUSE_FILTER_IGNORE`, or flip it dynamically based on actual content size (#1875). **Pure layout containers (`HBoxContainer`, `VBoxContainer` with no own visuals) should be `MOUSE_FILTER_IGNORE`.**

### Godot 4.6 `Skeleton3D` has no `remove_bone`
Spring-bone / wearable merging needs a manual recycle pool — rename stale slots to `__stale_bone_N`, detach (parent = -1), reset rest, and re-allocate from a free pool (#1849). Any PR that adds bones dynamically needs this lifecycle.

### Proto regeneration
`lib/build.rs` auto-generates decoder tables + `component_id_to_name` from `.proto` sources. PRs that bump the `decentraland-protocol` submodule should regenerate cleanly without manual edits to generated files.

### Focus stealing on mobile
A `Button` inside a panel that appears over a `LineEdit` will steal focus → keyboard closes → bad UX. Pattern: use `Control` + `_gui_input` instead of `Button`, set `focus_mode = 0` and `mouse_filter = IGNORE` on the container (#1822).

### Virtual keyboard buffer sync
After programmatically inserting text into a `LineEdit` on mobile, call `DisplayServer.virtual_keyboard_show(text, …)` to re-sync the OS buffer, or backspace will behave as if the inserted text isn't there (#1822).

### Logging discipline & Sentry quota
**This is the highest-leverage thing to scan a diff for that static checks will never catch.** Logs are not free: error- and warning-level logs flow into Godot's error stream, which the Sentry SDK captures and ships in prod/staging builds. Every such log added in a PR consumes the shared Sentry **event/attachment quota** for the lifetime of that code — and one mis-leveled log on a hot path can exhaust it.

How a log reaches (or doesn't reach) Sentry — verified against `lib/src/tools/godot_logger.rs` and `godot/src/project_main_loop.gd`:

| Source | Macro / call | Routes to | In Sentry? |
|---|---|---|---|
| Rust | `tracing::error!` | `print_error` (Godot error, real file:line) | **Yes — opens an issue** |
| Rust | `tracing::warn!` | `print_warning` (Godot warning) | Yes (lower severity) |
| Rust | `tracing::info!` / `debug!` / `trace!` | plain `godot_print!` | **No** |
| GDScript | `push_error` / `printerr` | Godot error | **Yes** |
| GDScript | `push_warning` | Godot warning | Yes (lower severity) |
| GDScript | `print` / `prints` | console | **No** |

`RUST_LOG`, `--rust-log`, and `decentraland://open?rust-log=…` all work for filtering at runtime; source file/line metadata is preserved for Sentry and the Godot debugger.

The team already pays for over-reporting with machinery in `project_main_loop.gd`: `_before_send` drops all dev-build events, samples log attachments at 1% (`ATTACH_LOG_SAMPLE_RATE`), and runs a `NOISE_PATTERNS` denylist that throws away engine/driver/livekit spam (keeping a 5% canary). **The existence of that denylist is the tell: every error-level log that fires in a loop is a quota problem.** Review new logs so they never have to be added to it.

The rule for any log added in a diff:

- **`error!` / `push_error` / `printerr` → only for genuine, actionable faults** a maintainer would actually want surfaced as a Sentry issue (a broken invariant, a corrupted response we can't recover from, a state that shouldn't be reachable). If the answer to "would we open a ticket for this?" is no, it's not an error.
- **Expected-and-handled conditions are not errors.** A missing texture / wearable / optional asset, an absent optional field, a network resource that 404s and falls back — these have a defined `else` branch, so they're a **`warn!` / `push_warning`** at most, or **`debug!` / `print`** if routine. The classic anti-pattern: `error!("texture not found: {url}")` next to a line that already substitutes a placeholder. That's a `warn!` or `debug!`, never an `error!`.
- **Debugging/diagnostic output is `tracing::debug!` (Rust) or `print` (GDScript)** — never error/warn. Things added to trace a problem during development must not ship as Sentry events.
- **Hot paths multiply everything.** A log inside the scene-runner update loop, pointer-event loop, per-entity scan, or a per-frame `_process` can fire thousands of times per session. Even a `warn!` there is suspect; an `error!` there is a quota incident. Flag it.

### Unity parity
When review cites `SkyboxRenderController.cs:183`, `avatar rotation_y wire convention`, `camera FOV 60°`, or similar, the reference is the **Unity Foundation Client**. The reviewer is comparing byte-for-byte / degree-for-degree, and the PR should match unless it explains why not.

### Platform detection — prefer `OS.get_name()` over `DclGlobal.is_*`
The codebase has both `DclGlobal.is_ios`, `is_android`, `is_mobile` (defined in `lib/src/godot_classes/dcl_global.rs`) **and** direct `OS.get_name() == "iOS" / "Android"` checks. The convention in this repo is the latter — `connection_quality_monitor.gd`, `notifications_manager.gd`, and the example scripts all use `OS.get_name()`. **Flag new code that uses `DclGlobal.is_ios()` / `is_android()` / `is_mobile()` and ask for `OS.get_name() == "iOS"` instead** unless there's a specific reason (e.g. the Rust-side flag is already cached and the call is on a hot path). When checking platform branches, also verify the author covered every relevant target — see the red-flag note about incomplete `OS.get_name()` matches.

### `_process` vs `_physics_process`
`_process(delta)` runs at frame rate (variable), `_physics_process(delta)` runs at the fixed physics tick. Camera follows, input polling, and UI animation belong in `_process`; physics integration, character controller updates, and anything that interacts with `move_and_slide` / `RigidBody3D` belong in `_physics_process`. Frame-rate-dependent logic (e.g. `position += velocity * delta` for a kinematic body that participates in collisions) placed in `_process` is a real bug class here — flag it. Conversely, UI tweens or HUD updates running in `_physics_process` will look stuttery on high-refresh displays.

---

## 6. Calibration — what good looks like here

Tone in merged reviews (see `regenesis-claw` on #1830, #1849, #1878):

- Open with one line acknowledging what's right before listing issues.
- Severity labels (`🔴` blocker, `🟡` suggestion, `nit`) or a markdown table of findings. Use sparingly — on small PRs a bulleted list is fine.
- For each finding: the *observed* behavior, *why* it's wrong, and the minimal fix. File path + line if applicable.
- Distinguish "blocks merge" from "follow-up issue welcome" from "nit". Small fixes should rarely be labeled blockers.
- End with a short positives list and an explicit `Approving` / `Requesting changes` / `Commenting` recommendation.
- On re-review after a follow-up commit, produce a *delta* review: table of previous findings with ✅ / ⚠️ / ❌ status, then notes on what's new.

Tone **not** to match:
- No line-by-line rewrites of working code.
- No style nits that `gdformat` / `rustfmt` would have caught — assume static checks are authoritative on style.
- No requests to add tests for code that has no test harness in its directory (much of `godot/` has none). If tests would require building infra, frame it as a follow-up.
- No speculative "what if the user does X" without a plausible path to X.
- Don't ask for documentation beyond what the PR body / existing docs already provide. Code comments are kept sparse in this repo on purpose.

Length:
- Small fix PR (1 file, <30 lines): 3–6 sentences is plenty.
- Feature PR (200+ lines, multiple dirs): full structured review with findings sections is expected.
- Refactors / cross-cutting changes: open with the architectural read before individual findings.

---

## 7. Quick red flags — scan for these first

A reviewer should `grep` / eyeball the diff for these before reading logic:

- `print(` / `prints(` / `print_verbose(` in non-tool GDScript → likely debug leftover.
- New `tracing::error!` (Rust) or `push_error(` / `printerr(` (GDScript) in the diff → these ship to Sentry and cost quota. Confirm the condition is a genuine, actionable fault. If it's expected/recoverable or already has a fallback (missing texture/asset, optional absent, 404-then-default), ask to downgrade to `warn!`/`push_warning` or `debug!`/`print`. See Section 5.
- New `tracing::error!` / `tracing::warn!` (or `push_error`/`printerr`) inside a loop, per-entity/per-frame scan, `_process`, or the scene-runner/pointer-event hot path → potential Sentry quota burst; flag even warnings here.
- A `tracing::error!` / `push_error` sitting right next to a fallback/`else`/placeholder assignment → mis-leveled; it's a `warn!`/`debug!`, not an error.
- `.claude/` under the diff path → memory files.
- `# TODO` / `# FIXME` added in this PR (vs already existed) → ask for an issue link.
- `await …` inside `_ready` / `_process` / `_input` without guards → re-entrancy risk.
- New `custom_minimum_size = Vector2(…)` on an overlay container → probable mouse-filter bug.
- `shader_parameter/<name>` in a `.tres` that doesn't exist in the referenced `.gdshader` → orphan.
- `find_node` / `get_node("%Foo")` with a unique path that just changed in the same PR → broken reference.
- Imports of `std::sync::Mutex` when a `parking_lot::Mutex` is already used elsewhere in the module → style drift.
- `.unwrap()` / `.expect("…")` on `FromGodot` / `try_to` conversions inside the scene-thread hot path → will crash the scene instead of logging.
- Any change to `rust-toolchain.toml`, `Cargo.lock` across the whole dependency tree, or the Godot version → escalate; these need a human-stakeholder call.
- Modifications under `plugins/dcl-godot-ios/godot` or any submodule pointer → verify intentional and not a submodule-drift side-effect.
- `DclGlobal.is_ios()` / `is_android()` / `is_mobile()` in new GDScript → request `OS.get_name() == "iOS"` / `"Android"` to match the repo convention.
- `OS.get_name()` checks that handle some but not all relevant targets (e.g. branches on `"Android"` but silently falls through on `"iOS"`, or covers mobile but ignores `"Web"` / `"macOS"`) → ask which platforms were considered and verify every target the change is supposed to support is covered.
- `_process` doing physics-coupled work, or `_physics_process` doing UI work → see the pattern note in Section 5.

---

## 8. Scope the PR asked for

Match the size of the review to the size of the change. Bug-fix PRs like #1874 (9 lines added) are merged with a one-line `APPROVED` — a 500-word review on a 9-line diff is *noise*, not signal. Conversely, 300+-line feature PRs (#1830, #1841, #1849, #1878) get structured reviews because the surface area earns them.

If you're unsure whether the PR is "small fix" or "feature":
- `additions + deletions < 50` and one file → small fix; keep review under 6 sentences unless you find a blocker.
- Multiple dirs or >200 lines → feature; give the full treatment.

---

## 9. Anti-goals

- Do **not** invent missing context. If the PR description or code don't tell you why a decision was made, ask — don't speculate in a blocking tone.
- Do **not** suggest introducing abstractions ("extract this into a helper class", "generalize to N backends") unless the code already shows duplication the PR is making worse. This repo is consciously non-abstracted.
- Do **not** suggest rewriting GDScript in Rust (or vice versa) as a review comment. That's an architectural decision, not a PR nit.
- Do **not** mark as blocking: stylistic preferences, naming bikeshedding that doesn't violate `.gdlintrc`, single-letter variable names inside short lambdas, or anything `rustfmt` / `gdformat` will normalize.
- Do **not** re-litigate decisions from earlier PRs in the same series. If a pattern was merged last week, a follow-up PR is not the place to reopen it — file an issue instead.
