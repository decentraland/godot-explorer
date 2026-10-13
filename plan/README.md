# Plan: three layers, two languages, one engine that only runs scenes

Working plan for taking the React Native app on `feat/react-native` (draft PR #3059) to the shipping
architecture from the proposal ("Explorer Architecture Split", 12 Oct 2026): a React Native shell for
every screen, a Rust core (`dcl_core`) for sessions, networking and rules, and a Godot engine
(`dcl_engine`) that only runs scenes. GDScript is retired. Desktop becomes a scene-only emulator
built from the same engine.

This folder lives on the branch while the work is in flight and is **deleted before the branch merges
to `main`**. Nothing outside `plan/` may link into it. Everything in it was checked against the tree on
13 Oct 2026; line counts are `wc -l`.

## The shape of it

- **13 Oct – 30 Nov 2026, one developer (Mateo), side project.** Phases 1–4: the migration itself,
  **feature-complete against 1.15** — every screen, every sign-in method, push, IAP, share, calendar,
  three languages. The checkpoint on 30 Nov is binary: parity checklist green or not.
- **1–30 Dec 2026, full team.** Phase 5, the five new features (Challenges, Gallery, Discover v2,
  Android IAP + Web Shop, Badges + Scrapbook) built in RN on the finished core, which is also how
  the team learns the architecture. Phase 6, full testing and polish of everything. Release
  candidate cut around 19 Dec; the branch merges to `main` then and this folder is deleted.
- **First days of January 2027: 2.0.0 ships**, as an update of the existing store app, with nothing
  a 1.15 user had missing.
- **After the release, phase 7:** the engine-side GDScript (avatar, player, SDK components, fetcher)
  moves into `dcl_engine`. 2.0.0 ships with that GDScript still inside the engine, behind the
  Rust `Engine` facade; it is internal debt, not a user-facing gap.
- `main` keeps shipping 1.16, 1.17… for LMT3, which is a marketing campaign on the current client
  and is not touched by any of this.

## Files

| File | What it holds |
|---|---|
| [01-today.md](01-today.md) | Inventory on 13 Oct 2026: the numbers and the coupling edges the plan has to cut |
| [02-target.md](02-target.md) | The end state: crates, one core per process, who starts what, engine modes, what `godot/` becomes |
| [03-engine-api.md](03-engine-api.md) | Core ↔ engine contract, and the by-name calls and node paths it replaces |
| [04-core-api.md](04-core-api.md) | Shell ↔ core contract (FFI), the app state machine, and the JSON bridge it replaces |
| [05-system-placement.md](05-system-placement.md) | Every shared system: where it is, where it goes, which phase moves it |
| [phase-1-core-carve-out.md](phase-1-core-carve-out.md) | `dcl_core` exists and runs under `cargo test`; the January scope of the carve-out |
| [phase-2-rust-root-emulator.md](phase-2-rust-root-emulator.md) | A Rust root boots the engine; the engine API is real; desktop is the emulator |
| [phase-3-ffi-entry-flow.md](phase-3-ffi-entry-flow.md) | Typed FFI, the core state machine, every sign-in method in RN, JSON bridge deleted |
| [phase-4-screens.md](phase-4-screens.md) | The GDScript UI sweep, every domain wired to the core, feature parity with 1.15 |
| [phase-5-new-features.md](phase-5-new-features.md) | December, team: the five new features in RN on the finished core |
| [phase-6-testing-polish.md](phase-6-testing-polish.md) | December, team: full testing, polish, release candidate, store readiness |
| [phase-7-engine-port.md](phase-7-engine-port.md) | After the release: avatar, player, SDK components, fetcher into `dcl_engine`; zero `.gd` |
| [decisions.md](decisions.md) | Decided and open decisions, each with a recommendation and a date |
| [risks.md](risks.md) | Risks, guardrails, and the measurement each one needs |

## Phases and status

| Phase | Window | Who | Status |
|---|---|---|---|
| 1. Core carve-out (January scope) | 13 – 30 Oct | Mateo | not started |
| 2. Rust root + emulator | 26 Oct – 6 Nov | Mateo | not started |
| 3. FFI + entry flow | 2 – 15 Nov | Mateo | not started |
| 4. UI sweep + every domain on the core | 12 – 30 Nov | Mateo | not started |
| **Checkpoint: feature-complete** | **30 Nov** | | |
| 5. New features | 1 – 19 Dec | full team | not started |
| 6. Full testing + polish, RC cut, merge to `main` | 1 – 30 Dec | full team | not started |
| **Release 2.0.0** | **first days of Jan 2027** | | |
| 7. Engine-side port | after the release | team | not started |

Phases overlap on purpose: 2 starts when 1.1 lands, 3 starts when 2.1 lands. Update the status
column and the checkboxes inside each phase file as work lands. Sizes in the phase files are S
(≤1 day), M (2–3 days), L (about a week), XL (more than a week); they are guesses, and for the solo
window they are the thing to watch — see risks R-8.

## Rules on this branch, from day one

1. **No new `.gd` file.** New behaviour goes to RN or Rust. CI fails a PR into the branch that adds one
   (phase 1, item 1.0).
2. **No HTTP call, no `user://` write and no business rule added to GDScript.** Fixes on `main` are
   exempt where the fix lives in existing GDScript; they are merged, not copied.
3. **RN never commands the engine and the engine never emits to RN.** Everything goes through the core.
   The native host (Kotlin/Swift) owns the engine *view*; the core owns the engine *state*.
4. **No host or URL hardcoded in TypeScript.** Hosts come from `core.urls()`. An eslint rule enforces
   it outside mock files (phase 3, item 3.8).
5. **Features for 2.0.0 land on this branch only.** `main` keeps shipping the 1.x line for LMT3 with
   its own fixes.
6. **No raising `max-file-lines` or `max-public-methods` in `.gdlintrc`.** The files at the cap are
   being deleted.

## How work lands

- Until 30 Nov the work items are Mateo's PR list into the branch, one module or one domain per PR,
  so that December reviewers can read the history. From 1 Dec the team's PRs follow the same rule.
- `main` is merged into the branch every day by a scheduled automated job (an AI session) that
  updates PR #3059 with what landed on `main`, skips UI-only changes, and resolves a conflict in a
  `.gd` file the branch deleted as deleted. A fix on `main` that matters outside the UI is redone
  in RN or Rust by the job or flagged for a person (D-31).
- The branch merges to `main` at the release-candidate cut in late December; this folder is deleted
  in that merge.
- Phase 1 PRs are reviewable Rust-only moves: no behaviour change, no renames beyond the move,
  `cargo test -p dcl_core` green.
