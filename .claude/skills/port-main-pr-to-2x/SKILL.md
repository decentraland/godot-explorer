---
name: port-main-pr-to-2x
description: Use when re-implementing a change that landed on `main` (the 1.x GDScript line) on the `2.x` branch (React Native shell + Rust `dcl_core` + Rust `dcl_engine`, zero GDScript) — one upstream PR at a time, during a main→2.x sync or on its own. Encodes the 2.x invariants a port must keep, where every 1.x GDScript system lives on 2.x, how to translate main's Rust (gdext 0.4.5) into 2.x idioms (gdext 0.5, typed calls, core-owned runtime), flags/settings/analytics/notification rules, how to handle engine-fork APIs 2.x's engine lacks, and the checks and commit shape. Trigger on "port #N to 2.x", "re-implement this main PR on 2.x", "what is the 2.x equivalent of <file>.gd", or a conflict between a `main` change and 2.x code.
---

# Port a `main` PR to `2.x`

A `main` PR is written against the 1.x client: GDScript UI and logic, Godot plugins, Rust on gdext
0.4.5 under `lib/src`. On `2.x` the same behaviour lives somewhere else and is written differently.
Porting means: **read what the PR does for the player, find where that lives on 2.x, and make 2.x
do the same thing the 2.x way.** It never means copying a `.gd` file.

The workflow that decides *which* PRs to port and lands them is the `sync-main-into-2x` skill. The
architecture is `plan/` (`README.md`, `02-target.md`, `05-system-placement.md`, `decisions.md`).

## 1. Invariants a port keeps

| Layer | Where | Owns |
|---|---|---|
| Shell | `rn/` (TypeScript) + host `rn/android`, `rn/ios` (Kotlin/Swift) | every screen, modal and the HUD; OS services through RN libraries or the host (push, local notifications, IAP, share…) |
| Core | `lib/crates/dcl_core` (Rust, **no Godot**) | app state machine, session, settings, feature flags, analytics, notifications, destination, signed requests |
| Engine | `lib/crates/dcl_engine` (Rust, gdext 0.5) + script-less `godot/` | scenes, avatars, player and camera, joystick/gamepad/emote wheel, content → resources |

1. **No GDScript in the product.** `find godot/src -name '*.gd'` stays empty (CI). Only editor
   addons in `godot/addons/` keep GDScript (D-open-5). Offline converters and probes from `main`
   (`godot/src/tool/**`) are not brought in; if one is really needed, say so instead.
2. **RN never commands the engine; the engine never emits to RN** (D-3). Engine fact → core → RN;
   RN action → core → engine. A port that needs to cross adds a typed core call or event.
3. **No host/URL in TypeScript** (`core.urls()`); **analytics only through the core** (D-9);
   **settings and per-install state in `dcl_core::settings`** (D-18); **flags from
   `dcl_core::flags`**.
4. **No Godot plugin** (D-41): plugin features become RN libraries or host code.
5. **No by-name calls** (`call("x")`, `call_deferred("x")`, `emit_signal("x")`, `set("prop")` on
   our classes). `cargo test -p dclgodot --lib call_targets` enforces it.
6. **Big files are closed**: `scene_manager.rs`, `avatar_scene.rs`, `content_provider.rs`,
   `communication_manager.rs`, `player/body.rs` (>1500 lines). New behaviour gets its own module
   and one call from the big file.
7. **Desktop is a scene-only emulator** (D-4): no product UI on desktop.
8. **The backpack is the only screen with the engine surface over RN** (D-11). A 1.x feature that
   drew a live 3D avatar in a modal or card gets static art on 2.x (say so in the commit).
9. Repo hygiene (`CLAUDE.md` → Code hygiene): comments 2–3 lines and only what the code can't say,
   never history ("ported from avatar.gd", "previously…"), no single-call-site helpers, English only.

## 2. Where 1.x systems live on 2.x

`plan/05-system-placement.md` is the full map. The ones ports hit most:

| 1.x (main) | 2.x |
|---|---|
| `lib/src/<path>` | `lib/crates/dcl_engine/src/<path>`, or `lib/crates/dcl_core/src/` for pure logic (http, urls, env, session, flags, analytics queue, notifications, settings) |
| `avatar.gd`, `avatar_emote_controller.gd`, loaders, mesh assembler, impostors, nameplates | `dcl_engine/src/avatars/` (`avatar/{build,emote,emote_load,glider,lifecycle,lod,nametag,anchors}.rs`, `mesh_assembler.rs`, `nameplate.rs`, `proximity_fade.rs`, `sfxs.rs`, `impostor*.rs`); the AnimationTree is in the script-less `godot/src/decentraland_components/avatar/avatar.tscn` |
| `player.gd`, camera controller, camera clamp, mobile camera input, stuck detector | `dcl_engine/src/player/` (`body.rs`, `rig.rs`, `camera_controller.rs`, `camera_clamp.rs`, `mobile_input.rs`, `step_up.rs`, `stuck_detector.rs`) |
| joystick, joypad, emote wheel (in-world controls) | `dcl_engine/src/ui/` (engine controls, D-24) |
| SDK components in GDScript (video, lights, text shape, gltf container, nft…) | `dcl_engine/src/{av,media,lights,text_shape,nft,scene_runner/components}` |
| `scene_fetcher.gd`, loading orchestration | `dcl_engine/src/fetcher/` |
| `feature_flags.gd` | rules + fetch: `dcl_core::flags`; engine-side effects: `dcl_engine/src/app/feature_flags.rs` |
| `config_data.gd` (`settings.cfg`) | `dcl_core::settings` (RN edits, the engine reads via `engine/settings_link.rs`) |
| `notifications_manager.gd`, local notifications, push | `dcl_core::notifications` + RN `rn/src/notifications/` + host module `DclPush` |
| `global.gd`, `main.gd` boot | `dcl_engine/src/engine/{root,bootstrap}.rs`, `app/global_*.rs`; app state in `dcl_core::app` |
| `lobby.gd`, auth, avatar creation, FTUE carousel | RN `rn/src/flow/`, `rn/src/discover/FtueScreen.tsx` over `dcl_core::{app,session,avatar}` |
| `explorer.gd` HUD, menu, chat, coach marks | RN `rn/src/hud/`, `rn/src/menu/`; world screen `rn/src/world/` |
| `modal_manager.gd` | RN `rn/src/modals/` (`CoreModals`, `ModalProvider`); policy in the core |
| discover, events, places, backpack, profile, settings pages | RN `rn/src/{discover,backpack,profile,settings}/` |
| friends / social | deleted (D-43); RN TypeScript in phase 6 (6.9) — main's social changes are not ported |
| debug server (`tool/debug_server`), `gp_benchmark_runner.gd` | `dcl_engine/src/tools/{debug_hub,gp_benchmark}/`, `benchmark_*.rs` |
| `plugins/` (Android/iOS Godot plugins) | RN libraries or host code (D-41) |
| GDScript tests (`godot/src/test`) | Rust unit tests / `#[itest]` / jest with their subject (D-20) |
| `src/check_gdscript.rs`, gdlint/gdformat | deleted on 2.x |
| `godot/locale/*.csv` | generated from the RN `engine` i18n area (D-26); never edit by hand; see the `i18n` skill |

Finding the Rust twin of a GDScript function or constant:
`git log --oneline -S '<name>' origin/2.x -- lib/` usually lands on the phase-5 port (5cf67185bf)
or phase 4 (d56ccae8b0); then `grep -rn` the Rust name in that commit.

## 3. How to port

1. **Read the PR as a user would experience it**: `gh pr view <N>` (behaviour, flags, analytics,
   known gaps, test plan), then `git show <sha>`, then main's final files
   (`git show origin/main:<path>`). For a file main changed, diff it against the merge-base to see
   exactly what this PR added: `git diff $(git merge-base origin/2.x origin/main) <sha> -- <path>`.
2. **Find the 2.x home** of each touched system (§2) and read it. Phase 5 already ported every
   engine-side GDScript file: apply the PR's *diff* to the Rust twin; do not re-port the file.
3. **Keep what 1.x users and dashboards see identical**: constants, timings, flag names and their
   defaults (fail-open vs fail-closed), analytics event names and properties, settings semantics,
   Unity-parity references (issue numbers, Unity names) in comments.
4. **Remote flags**: `dcl_core::flags::{is_enabled, get_number}` with main's default; engine-side
   effects applied in `dcl_engine/src/app/feature_flags.rs`. Deeplink/CLI overrides go through the
   existing deep-link and `DclCli` paths.
5. **Persisted keys** from `config_data.gd` become `dcl_core::settings` keys (hidden when the user
   never sees them). No AsyncStorage for per-install state the core already tracks.
6. **Crossing layers** adds a typed core call/event. Regenerate bindings
   (`cd lib && cargo build -p dcl_bindings && cd ../rn && npm run ubrn:generate`), commit what the
   repo tracks, keep `rn/src/core/mock` working for no-engine builds.
7. **UI**: RN design system (`rn/src/ds`), i18n keys `t(...)` added to en/es/pt_BR in `rn/src/i18n` (the only source; see the `i18n` skill), screens registered in
   their `gallery.tsx` when the folder has one. Figma references from the PR body carry over.
8. **Tests** move with their subject (D-20): pure logic → Rust unit tests or jest; engine behaviour →
   `#[itest]` when feasible.
9. **Adapt, don't break a rule**: when the 1.x shape violates an invariant (§1), do the 2.x shape
   and write the difference in the commit body (and it goes to the sync PR's table).

## 4. main's Rust → 2.x idioms

2.x is on gdext 0.5 with typed bindings (from the engine branch's `extension_api.json`); main is on
0.4.5. When main's Rust conflicts or fails to compile on 2.x:

| main (1.x) | 2.x |
|---|---|
| `obj.call_deferred("method", &[args])` from any thread | `engine::local_task::defer_from_any_thread(&[args], move \|args\| { … typed call on Gd::try_from_instance_id(id) … })` |
| `call_deferred` on the main thread | `engine::local_task::defer(\|\| …)` |
| `self.base_mut().emit_signal("x", &[v])` | `self.signals().x().emit(&v)` |
| `promise.bind_mut().resolve…` on a tokio task | `PromiseDeferred::{resolve_with_data_deferred, reject_deferred}` (settles on the main thread) |
| `TokioRuntime::spawn`, `scene_runner::tokio_runtime` | `dcl_core::Core::spawn` (the runtime is the core's) |
| `#[var(get, set = f)]` | `#[var(get = g, set = f)]`, or no `#[var]` when only Rust uses it |
| `varray![…, s.to_godot()]`, `dict.set("k", packed)` | by reference: `&s.to_godot()`, `&packed` |
| `typed_array.to_variant().call("size"/"get")` (null-tolerant) | `typed_array.upcast_any_array()` then `.len()` / `.get(i)` |
| `has_method("x")` + `call("x")` on an engine-fork API | typed, once the engine change is merged into 2.x's pinned branch (`install` then regenerates the bindings; `cargo clean -p godot-ffi -p godot-codegen -p godot-core -p godot-bindings -p godot` if the old API is cached). If that cannot happen yet: one guarded helper probing `ClassDb::class_has_method`, the name added to `EXTERNAL` in `call_targets_test.rs` with its reason; without the method it behaves like the feature off; flag that 2.x needs an engine build carrying it |
| `DclGlobal`/node-path lookups of GDScript autoloads | the typed handles in `dcl_engine/src/engine/mod.rs` |
| `get_node("/root/Global").call("x")`, `Global.x` | the Rust owner of `x` (find it with §2's `git log -S`) |

## 5. Checks before committing

```bash
find godot/src -name '*.gd'                                   # empty
cd lib && cargo fmt --all \
  && cargo clippy --workspace --all-targets -- -D warnings \
  && cargo test -p dcl_core && cargo test -p dclgodot --lib     # includes call_targets
cd ../rn && npx tsc --noEmit && npm run lint && npm test        # when rn/ or the FFI changed
node scripts/i18n-engine.mjs --check                           # strings changed
```

Bindings: `npm run ubrn:generate` formats the generated C++ with `clang-format` when it is on the
PATH; without it the whole file reflows. Commit only a formatted regeneration.

Several sessions on one machine (worktrees):

- A fresh worktree lacks untracked build inputs: `.bin/godot/extension_api.json`, `.bin/protoc`
  and `lib/crates/dcl_engine/src/dcl/components/proto`. Symlink them from the main checkout and
  never commit the links.
- Sharing one cargo target (`CARGO_TARGET_DIR` at the main checkout's `lib/target`) saves disk, but
  cargo can reuse another worktree's build of a workspace crate: `touch` the crates' `src/lib.rs`
  before testing, and check your new tests actually ran.
- Don't symlink the whole `rn/node_modules`: `dcl-core` resolves to the other checkout's bindings.
  Link the packages one by one and point `dcl-core` at this worktree's `rn/modules/dcl-core`.

## 6. Commit shape

One commit (or a short series) per upstream PR, conventional title naming it:
`feat(avatar): locomotion iterations 2 + 3 in the Rust engine (main #3056)`. The body says where
each touched 1.x file's behaviour went (**Ported** / **Adapted** + how / **Discarded** + why /
**Needs follow-up** + why), in a few lines. No Co-Authored-By trailer. These lines become the
PR's per-upstream-PR table.
