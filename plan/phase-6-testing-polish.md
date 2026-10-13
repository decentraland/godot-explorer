# Phase 6: fix, validate and polish

Full team, December, first. Polish comes before any new feature.

## Goal

December, right after the branch merges to `main`. The AI-ported migration is reviewed first: fix
everything it got wrong, validate that every part of the app is polished and check every screen, on
real devices with signed-in accounts and against the Figma designs. What comes out is 1.15 parity,
polished; phase 7 then builds the new features on top of it. The team comes out of it fluent in the three-layer
architecture: every engineer has shipped at least one core call, one RN screen change and one
engine-facing fix.

## Exit criteria

- [ ] The parity checklist (4.16) is green on the device matrix below.
- [ ] The upgrade test (6.3) passes with a guest and an email account.
- [ ] Genesis Plaza fps and memory within the phase-2 baseline (R-1) on both QA phones.
- [ ] Symbolicated frames for a deliberate crash from Rust and from a scene on both platforms (R-7).

## Work items

- [ ] **6.0 Onboarding**. Every engineer builds the app with and without the engine,
  reads `02-target.md`, `03-engine-api.md`, `04-core-api.md`, adds one core call end to end
  (Rust → generated bindings → RN screen) and fixes one item from the overflow list. Owners per
  layer and per domain assigned (D-30).
- [ ] **6.1 What the AI got wrong**. Whatever was not green at the gate, plus defects found by reading and running the ported code, taken first.
- [ ] **6.2 Device matrix QA**. Moto Edge 60 Pro, a low-end Android (Mali, 4 GB), iPhone 14 Pro, an
  older iPhone, one tablet per platform. Each parity row run with a
  signed-in account, including the paths never tested on the branch: friends actions, notifications
  with data, account deletion, votes, reminders, push, IAP, deep links from a cold start.
- [ ] **6.3 Upgrade test** (R-6). A device on the store build of 1.15/1.16 updated to the branch build keeps
  its session, settings and cache. Repeated with a guest account and with an email account.
- [ ] **6.4 Polish**. Visual pass against the Figma frames (the gallery in `docs/rn-ui-spec/`
  gives one preview per frame), animation and loading states, empty and error states, safe areas
  on every device in the matrix, orientation transitions in and out of the world.
  - [ ] **Godot engine warm-up**. Since the first avatar preview is reached in the entry flow, start
    the engine in Idle through the core's lifecycle event and the native host after the first RN
    screen is visible, before the preview is requested. Prepare the avatar and its wearables as soon
    as the core has the profile, then show the live preview when its rect arrives. Keep engine
    startup one-time and off the RN launch path. Measure cold app launch, time from entering the
    avatar screen to the first usable 3D frame, and peak memory on Android and iOS before and
    after; keep the warm-up only if it reduces the preview wait without an unacceptable launch or
    memory regression. A snapshot placeholder is outside this item (D-29).
- [ ] **6.5 Performance and stability**. GP benchmark on both phones vs 1.15; memory on the
  low-end Android through a 30-minute session; thermal and fps-cap behaviour; the deliberate crash
  test for symbolication (R-7) on both platforms.
- [ ] **6.6 Localization pass**. es and pt_BR strings reviewed in place on every screen.
## Verification

- The daily crash KPI from the internal builds, read every morning.
- QA scripts in the `REVIEW.md` §4 format, one per domain and per feature, executed by someone
  other than the author.

## Depends on

The full-migration gate (phases 1–5). Unblocks phase 7: new features start on the polished app.
