# Plan: three layers, two languages, one engine that only runs scenes

Working plan for taking the React Native app on `feat/react-native` (draft PR #3059) to the shipping
architecture from the proposal ("Explorer Architecture Split", 12 Oct 2026): a React Native shell for
every screen, a Rust core
(`dcl_core`) for sessions, networking and rules, and a Godot engine (`dcl_engine`) that only runs
scenes. GDScript is retired. Desktop becomes a scene-only emulator built from the same engine.

This folder lives on the branch while the work is in flight and is **deleted before the branch merges
to `main`**. Nothing outside `plan/` may link into it. Everything in it was checked against the tree on
13 Oct 2026; line counts are `wc -l`.

## Files

| File | What it holds |
|---|---|
| [01-today.md](01-today.md) | Inventory on 13 Oct 2026: the numbers and the coupling edges the plan has to cut |
| [02-target.md](02-target.md) | The end state: crates, one core per process, who starts what, engine modes, what `godot/` becomes |
| [03-engine-api.md](03-engine-api.md) | Core ↔ engine contract, and the by-name calls and node paths it replaces |
| [04-core-api.md](04-core-api.md) | Shell ↔ core contract (FFI), the app state machine, and the JSON bridge it replaces |
| [05-system-placement.md](05-system-placement.md) | Every shared system: where it is, where it goes, which phase moves it |
| [phase-1-core-carve-out.md](phase-1-core-carve-out.md) | `dcl_core` exists and runs under `cargo test`; the client behaves identically |
| [phase-2-rust-root-emulator.md](phase-2-rust-root-emulator.md) | A Rust root boots the engine; the engine API is real; desktop is the emulator |
| [phase-3-ffi-entry-flow.md](phase-3-ffi-entry-flow.md) | Typed FFI, the core state machine, real sign-in in RN, JSON bridge deleted |
| [phase-4-screens.md](phase-4-screens.md) | Screens domain by domain; each deletes its GDScript when it lands |
| [phase-5-engine-port.md](phase-5-engine-port.md) | Avatar, player, SDK components, fetcher into `dcl_engine`; zero `.gd` |
| [decisions.md](decisions.md) | Decided and open decisions, each with a recommendation and a date |
| [risks.md](risks.md) | Risks, guardrails, and the measurement each one needs |

## Phases and status

| Phase | Window | Who | Status |
|---|---|---|---|
| 1. Core carve-out | Oct–Nov 2026 | side-project | not started |
| 2. Rust root + emulator | Oct–Nov 2026 | side-project | not started |
| 3. FFI + real entry flow | Nov 2026 | side-project | not started |
| Scope gate | week of 23 Nov 2026 | EM + team | |
| 4. Screens, domain by domain | Dec 2026 | full team | not started |
| 5. Engine-side port | Dec 2026, rest after release | full team | not started |
| Release | Jan 2027 | | |

Update the status column and the checkboxes inside each phase file as work lands. Sizes in the phase
files are S (≤1 day), M (2–3 days), L (about a week), XL (more than a week). They are guesses; the
scope gate corrects them with what phases 1–3 actually cost.

## Rules on this branch, from day one

1. **No new `.gd` file.** New behaviour goes to RN or Rust. CI fails a PR into the branch that adds one
   (phase 1, item 1.0).
2. **No HTTP call, no `user://` write and no business rule added to GDScript.** LMT3 fixes on `main`
   are exempt where the fix lives in existing GDScript; they are merged, not copied.
3. **RN never commands the engine and the engine never emits to RN.** Everything goes through the core.
   The native host (Kotlin/Swift) owns the engine *view*; the core owns the engine *state*.
4. **No host or URL hardcoded in TypeScript.** Hosts come from `core.urls()`. An eslint rule enforces
   it outside mock files (phase 3, item 3.8).
5. **From December the shipping client takes patch fixes only.** Features land on the branch, in RN or
   Rust.
6. **No raising `max-file-lines` or `max-public-methods` in `.gdlintrc`.** The files at the cap are
   being deleted, not grown.

## How work lands

- PRs target this branch, one module or one domain per PR, reviewed like any other PR (`REVIEW.md`).
  Each phase file's work items are the PR list.
- `main` is merged into the branch at least weekly. A conflict in a `.gd` file the branch deleted
  resolves as deleted; if the fix on `main` matters, it is redone in RN or Rust in the same merge PR.
- Nothing from the branch ships before December. LMT3 ships exactly what it would have shipped.
- Phase 1 PRs are reviewable Rust-only moves. Keep them that way: no behaviour change, no renames
  beyond the move, `cargo test -p dcl_core` green.
