# Phase 5: new features

Full team. Built in RN on the finished core; also how the team learns the
architecture.

## Goal

Five features that do not exist in 1.15 ship in 2.0.0: Challenges, Gallery, Discover v2, Android
IAP + Web Shop, Badges + Scrapbook. Each is an RN screen set on typed core calls, specified by
product before the team starts, built the way every phase-4 domain was built, and QA'd in phase 6.

Each feature's specification (Figma, backend endpoints, metrics) is needed before the gate so the
core calls it needs are in the FFI when the team arrives (D-34). Anything that needs the identity
key is a core function; public GETs stay in RN; hosts from `core.urls()`.

## Exit criteria

- [ ] Each feature behind a feature flag from the core, on by default in the RC.
- [ ] Each feature has its parity-checklist rows (4.16) and a QA script in the `REVIEW.md` §4 format.
- [ ] No new `.gd`, no hardcoded host, no engine command from RN: the branch rules hold.

## Work items

Owners are assigned at onboarding (D-30).

- [ ] **5.1 Challenges**. Spec: _pending_. Expected: a challenges list and detail, progress from
  a backend, rewards. Core: signed reads and claim calls; RN: screens and a Discover/menu entry.
- [ ] **5.2 Gallery**. Spec: _pending_. Expected: the profile photo gallery (camera screenshots,
  `screenshot.capture` exists today as a bridge method) with upload and delete. Core: snapshot
  storage calls; RN: grid and viewer. Not to be confused with `rn/src/gallery`, the UI-spec preview
  gallery.
- [ ] **5.3 Discover v2**. Spec: _pending Figma_. Replaces the Discover built in 4.2 on the same
  data layer; the public GETs and `resolve_destination` do not change.
- [ ] **5.4 Android IAP + Web Shop**. Google Play Billing through an RN IAP library (the same
  library can take over iOS StoreKit from the host later), on `dcl_core::credits` from 4.8; the
  Web Shop as a checkout opened through `open_url` with a deep-link return handled by the state
  machine. Spec: _pending_ (products, pricing, receipts verification endpoint).
- [ ] **5.5 Badges + Scrapbook**. Spec: _pending_. Expected: the badges service already in the
  backend, a badge list and detail on the profile, and the scrapbook as its own screen. Core:
  signed reads; RN: screens.
- [ ] **5.6 Metrics**. Screen and button names for each feature agreed with product before the
  screens are built; tracked through the core.

## Verification

- Each feature QA'd on both phones with a signed-in account before it closes, inside phase 6's plan.
- `cargo test -p dcl_core` for every new core call; Jest for the screens against the mock core.

## Depends on

The feature-complete gate (phase 4) and the feature specs.
