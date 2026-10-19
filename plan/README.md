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

A calendar target sits on top of the gates: the full migration ends on **30 November 2026**, December
is the full team, and 2.0.0 ships in **January 2027**. Inside each stack the work is an ordered list of
tasks and each phase ends at a gate. The work is AI-assisted, so elapsed time says nothing about size
and no item carries an estimate.

- **Stack 1, phases 1–5: the full migration**, one developer plus AI sessions, until 30 Nov.
  Ends at the **full-migration gate**: the parity checklist (4.16) is green against 1.15 — every
  screen, every sign-in method, push, IAP, share, calendar, three languages — **and**
  `find godot/src -name '*.gd'` returns nothing (addons per D-open-5). Everything is in Rust and the
  new architecture. The end of phase 4 is the *feature-complete* milestone inside this stack; phase 5
  ports the engine-side GDScript to Rust. The gate is binary.
- **Stack 2, December, full team.** The branch merges to `main` at the start of December. First phase 6:
  fix everything the AI got wrong, validate that every part of the app is polished, check every
  screen. Then phase 7: the five new features (Challenges, Gallery, Discover v2, Android IAP + Web
  Shop, Badges + Scrapbook), of which Android IAP is the largest, and the release candidate. December is also how the team learns the architecture.
- **2.0.0 ships in January** as an update of the existing store app, with nothing a 1.15 user had
  missing and no GDScript in the client.
- **Phase 8, after the release:** publishing the emulator under its own name (8.1) and, if 2.0.0 kept
  the video-only plugin, moving video into the host (8.2). Neither is needed for the gate. VR is removed before
  the gate (5.12): 2.0.0 has no VR at all.
- **Checkpoint on 1 November** (D-39): where stack 1 stands, the RN-host measurement, and one of four
  options written in advance (gate stands; zero `.gd` moves to 2.1; the dates move or phase 7 shrinks;
  Plan B for the shell, D-open-16).
- `main` keeps shipping 1.16, possibly 1.17, for LMT3 until the December merge. LMT3 is a marketing
  campaign on the current client; the crash, performance, SDK and fonts/skybox work for it happens on
  `main` and is picked up by the branch every day (D-31). Before the merge, a permanent `1.x` branch is
  cut for every later 1.x release (D-40).

## Files

| File | What it holds |
|---|---|
| [00-why.md](00-why.md) | Why: the placement rule, pros and cons, why a native UI, why React Native and not Flutter or another, what happens if memory misses |
| [01-today.md](01-today.md) | Inventory on 13 Oct 2026: the numbers and the coupling edges the plan has to cut |
| [02-target.md](02-target.md) | The end state: crates, one core per process, who starts what, engine modes, what `godot/` becomes |
| [03-engine-api.md](03-engine-api.md) | Core ↔ engine contract, and the by-name calls and node paths it replaces |
| [04-core-api.md](04-core-api.md) | Shell ↔ core contract (FFI), the app state machine, and the JSON bridge it replaces |
| [05-system-placement.md](05-system-placement.md) | Every shared system: where it is, where it goes, which phase moves it |
| [phase-1-core-carve-out.md](phase-1-core-carve-out.md) | `dcl_core` exists and runs under `cargo test`; comms, cache policy and math types land in 5.1 |
| [phase-2-rust-root-emulator.md](phase-2-rust-root-emulator.md) | A Rust root boots the engine; the engine API is real; desktop is the emulator |
| [phase-3-ffi-entry-flow.md](phase-3-ffi-entry-flow.md) | Typed FFI, the core state machine, every sign-in method in RN, JSON bridge deleted |
| [phase-4-screens.md](phase-4-screens.md) | The GDScript UI sweep, every domain wired to the core, feature parity with 1.15 |
| [phase-6-testing-polish.md](phase-6-testing-polish.md) | team, December, first: fix what the AI got wrong, validate and polish everything |
| [phase-7-new-features.md](phase-7-new-features.md) | team, December, then: the five new features in RN on the polished core, release candidate, store readiness |
| [phase-5-engine-port.md](phase-5-engine-port.md) | gdext upgraded to its modern API first, then avatar, player, SDK components, fetcher into `dcl_engine`; VR removed; zero `.gd` |
| [phase-8-after-release.md](phase-8-after-release.md) | After the release: the emulator under its own name, video into the host if 2.0.0 kept the video-only plugin |
| [decisions.md](decisions.md) | Decided and open decisions, each with a recommendation |
| [risks.md](risks.md) | Risks, guardrails, and the measurement each one needs |

## Phases and status

| Phase | Gate to start | Who | Status |
|---|---|---|---|
| 1. Core carve-out | none | Mateo + AI | code done; phone regression open |
| 2. Rust root + emulator | 1.1 landed | Mateo + AI | code done; device checks (2.8, 2.9) open |
| 3. FFI + entry flow | 2.1 landed | Mateo + AI | code complete on `feat/react-native-phase5` (draft PR #3081); phase 5 closed on 17 Oct 2026; the open items are handed to phase 6 polish, see [phase-5-verification.md](phase-5-verification.md) |
| 4. UI sweep + every domain on the core | phase 3 (the 4.0 sweep goes first, D-44) | Mateo + AI | not started |
| 5. Modern gdext, engine-side port, VR removed | 2.1 landed; mostly after 4.0 | Mateo + AI | code on `feat/react-native-phase5` (draft PR #3081); phase 5 closed on 17 Oct 2026; the open items are handed to phase 6 polish, see [phase-5-verification.md](phase-5-verification.md) |
| **Checkpoint 1 Nov** (D-39): real state of stack 1, one of options (a)–(d) chosen | | | |
| **Gate: full migration** (parity checklist green and zero `.gd`), target 30 Nov | | | |
| 6. Fix, validate, polish (December, first) | full-migration gate | full team | not started |
| 7. New features, RC cut (December, then) | phase 6; specs in hand | full team | not started |
| **Release 2.0.0** (January) | RC green | | |
| 8. After the release | release | team | not started |

Phases overlap by dependency: 2 starts when 1.1 lands, 3 starts when 2.1 lands, 5 starts when 2.1 lands and mostly runs after the 4.0 sweep; in December 6 comes first and 7 follows. Update the status
column and the checkboxes inside each phase file as work lands. Items are an ordered stack, taken
top to bottom unless a dependency says otherwise. There are no size tags and no durations; the thing
to watch is how many items are open at the gate and how many `.gd` files remain (risks R-8, R-17).

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
7. **No Godot plugin.** App features use RN libraries where one exists; the few native parts we write
   (attestation, device id, video, engine view) live in the host (`rn/android`, `rn/ios`) and reach the
   core through uniffi's Kotlin/Swift bindings; the engine gets platform services from the core (D-41).

## How work lands

- Until the full-migration gate the work items are Mateo's PR list into the branch, one module or
  one domain per PR, so that the team can read the history. After it the team's PRs follow the same rule.
- Until the December merge, `main` is merged into `2.x` through a draft sync PR (an AI session with
  the `sync-main-into-2x` skill, porting with `port-main-pr-to-2x`): every upstream PR gets a
  verdict, UI-only fixes are skipped, a `.gd` the branch deleted stays deleted, and a change that
  matters is redone in RN or Rust or flagged for a person (D-31). The sync PR is merged as a merge
  commit, never squashed. Both skills are deleted with this folder in the December merge.
- The branch merges to `main` at the start of December, once the full-migration gate is green; this
  folder is deleted in that merge.
- Phase 1 PRs are reviewable Rust-only moves: no behaviour change, no renames beyond the move,
  `cargo test -p dcl_core` green.
