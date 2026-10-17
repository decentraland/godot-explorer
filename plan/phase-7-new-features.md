# Phase 7: new features

Full team, December, after phase 6. Built in RN on the polished core; also where the release
candidate is cut. Android IAP + Web Shop is the largest.

## Goal

Five features that do not exist in 1.15 ship in 2.0.0: Challenges, Gallery, Discover v2, Android
IAP + Web Shop, Badges + Scrapbook. Each is an RN screen set on typed core calls, specified by
product before the team starts, built the way every phase-4 domain was built, and QA'd on phase 6's device matrix before the RC.

Each feature's specification (Figma, backend endpoints, metrics) is needed before the gate so the
core calls it needs are in the FFI when the team arrives (D-34). Anything that needs the identity
key is a core function; public GETs stay in RN; hosts from `core.urls()`.

## Exit criteria

- [ ] Each feature behind a feature flag from the core, on by default in the RC.
- [ ] Each feature has its parity-checklist rows (4.16) and a QA script in the `REVIEW.md` §4 format.
- [ ] No new `.gd`, no hardcoded host, no engine command from RN: the branch rules hold.
- [ ] Crash-free sessions on the RC, from the daily Sentry KPI, at or above the 1.15 baseline over
      the last seven days before the RC.
- [ ] Genesis Plaza fps and memory on the RC within the phase-2 baseline (R-1) on both QA phones.
- [ ] Store readiness done: listings, screenshots, privacy manifests and data-safety forms updated
      for the RN app, App Review notes, the TestFlight and Play internal tracks carrying the RC.
- [ ] `plan/` deleted, version 2.0.0, rollout plan written (D-open-14).

## Work items

Owners are assigned at onboarding (D-30).

- [ ] **7.1 Challenges**. Spec: _pending_. Expected: a challenges list and detail, progress from
  a backend, rewards. Core: signed reads and claim calls; RN: screens and a Discover/menu entry.
- [ ] **7.2 Gallery**. Spec: _pending_. Expected: the profile photo gallery (camera screenshots,
  `screenshot.capture` exists today as a bridge method) with upload and delete. Core: snapshot
  storage calls; RN: grid and viewer. Not to be confused with `rn/src/gallery`, the UI-spec preview
  gallery.
- [ ] **7.3 Discover v2**. Spec: _pending Figma_. Replaces the Discover built in 4.2 on the same
  data layer; the public GETs and `resolve_destination` do not change.
- [ ] **7.4 Android IAP + Web Shop**. Google Play Billing through an RN IAP library (the same
  library can take over iOS StoreKit from the host later), on `dcl_core::credits` from 4.8; the
  Web Shop as a checkout opened through `open_url` with a deep-link return handled by the state
  machine. Spec: _pending_ (products, pricing, receipts verification endpoint).
- [ ] **7.5 Badges + Scrapbook**. Spec: _pending_. Expected: the badges service already in the
  backend, a badge list and detail on the profile, and the scrapbook as its own screen. Core:
  signed reads; RN: screens.
- [ ] **7.6 Metrics**. Screen and button names for each feature agreed with product before the
  screens are built; tracked through the core.
- [ ] **7.7 Release candidate**. Code freeze on `main`; version 2.0.0; `godot-asc-deploy` switched to the RN workspace; RC on
  TestFlight and the Play internal track; `GODOT_BUILD_SHA` bumped to the merged engine and
  `GODOT_USE_BRANCH` reset at the December merge (D-23).
- [ ] **7.8 Rollout plan** (D-open-14). Staged percentages, kill criteria, who watches the KPI
  the first week after release, how a 1.x hotfix would ship if needed.

## Verification

- Each feature QA'd on both phones with a signed-in account before it closes, with phase 6's device
  matrix and QA scripts.
- The daily crash KPI and the Play vitals, read every morning of the RC window.
- `cargo test -p dcl_core` for every new core call; Jest for the screens against the mock core.

## Depends on

Phase 6 (the polished, validated app) and the feature specs.
