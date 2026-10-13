# Phase 6: full testing and polish

1 – 30 Dec 2026, full team. The month the release is earned.

## Goal

Everything the migration and phase 5 produced is tested on real devices with signed-in accounts,
polished to the Figma designs, and cut as a release candidate that merges to `main` and goes to the
stores in the first days of January. The team comes out of the month fluent in the three-layer
architecture: every engineer has shipped at least one core call, one RN screen change and one
engine-facing fix.

## Exit criteria

- [ ] The parity checklist (4.16) plus the phase-5 rows is green on the device matrix below.
- [ ] Crash-free sessions on the RC, from the daily Sentry KPI, at or above the 1.15 baseline over
      the last seven days of December.
- [ ] Genesis Plaza fps and memory on the RC within the phase-2 baseline (R-1) on both QA phones.
- [ ] Store readiness done: listings, screenshots, privacy manifests and data-safety forms updated
      for the RN app, App Review notes, the TestFlight and Play internal tracks carrying the RC.
- [ ] The branch merged to `main`, `plan/` deleted, version 2.0.0, rollout plan written (D-open-14).

## Work items

- [ ] **6.0 Onboarding week** (1–5 Dec). Every engineer builds the app with and without the engine,
  reads `02-target.md`, `03-engine-api.md`, `04-core-api.md`, adds one core call end to end
  (Rust → generated bindings → RN screen) and fixes one item from the overflow list. Owners per
  layer and per domain assigned (D-30).
- [ ] **6.1 Phase-4 overflow** (first week). Whatever was not green on 30 Nov, taken first.
- [ ] **6.2 Device matrix QA**. Moto Edge 60 Pro, a low-end Android (Mali, 4 GB), iPhone 14 Pro, an
  older iPhone, one tablet per platform. Each parity row and each phase-5 feature run with a
  signed-in account, including the paths never tested on the branch: friends actions, notifications
  with data, account deletion, votes, reminders, push, IAP, deep links from a cold start.
- [ ] **6.3 Upgrade test** (R-6). A device on the store build of 1.15/1.16 updated to the RC keeps
  its session, settings and cache. Repeated with a guest account and with an email account.
- [ ] **6.4 Polish**. Visual pass against the Figma frames (the gallery in `docs/rn-ui-spec/`
  gives one preview per frame), animation and loading states, empty and error states, safe areas
  on every device in the matrix, orientation transitions in and out of the world.
- [ ] **6.5 Performance and stability**. GP benchmark on both phones vs 1.15; memory on the
  low-end Android through a 30-minute session; thermal and fps-cap behaviour; the deliberate crash
  test for symbolication (R-7) on both platforms.
- [ ] **6.6 Localization pass**. es and pt_BR strings reviewed in place on every screen.
- [ ] **6.7 Release candidate** (around 19 Dec). Code freeze on the branch; version 2.0.0; merge to
  `main` with `plan/` deleted; `godot-asc-deploy` switched to the RN workspace; RC on TestFlight
  and the Play internal track; `GODOT_BUILD_SHA` bumped to the merged engine and
  `GODOT_USE_BRANCH` reset (D-23).
- [ ] **6.8 Rollout plan** (D-open-14). Staged percentages, kill criteria, who watches the KPI
  the first week of January, how a 1.x hotfix would ship if needed.

## Verification

- The daily crash KPI and the Play vitals, read every morning of the RC window.
- QA scripts in the `REVIEW.md` §4 format, one per domain and per feature, executed by someone
  other than the author.

## Depends on

The 30 Nov checkpoint. Phase 5 feeds into 6.2 as features close.
