# Phase 6: refinement, after the release

Q1 2027, full team, after 2.0.0 ships. Nothing here blocks the release.

## Goal

2.0.0 ships with fewer features than 1.15 on purpose (D-21, D-25, D-26): the refactor goes out
first, then the features removed to get it out come back as RN screens on typed core calls, plus
the features frozen during December. No "not available in this version" message is left in the app
at the end of this phase.

## Work items

- [ ] **6.1 Sign-in providers** (L). Google, Apple and wallets, purely in RN (a Magic Link RN SDK is
  a candidate); `sign_in_browser` and `sign_in_wallet` stop returning `NotAvailable`. Metrics keep
  the `AUTH_SUCCESS(login_type)` taxonomy.
- [ ] **6.2 Push and local OS notifications** (M). An RN notifications library; the push token
  reaches the core through `set_push_token`; local scheduling from core events; push attribution in
  deep-link routing.
- [ ] **6.3 In-app purchases and credits** (L). Store purchase through an RN IAP library; quote,
  verify, balance and the marketplace round-trip in `dcl_core::credits` (what `iap_manager.gd` and
  `marketplace_tracker.gd` did); the credits UI.
- [ ] **6.4 Share sheet and calendar** (S). RN libraries behind the place and event sheets.
- [ ] **6.5 Localization** (S). `i18n/es` and `i18n/pt_BR` regenerated from `godot/locale/*.csv`
  (keys already match `en.csv`); a CI check for missing keys.
- [ ] **6.6 Features frozen during December** (per the product backlog). Built directly in RN and
  the core.
- [ ] **6.7 Engine-side port leftovers** from phase 5, until `find godot/src -name '*.gd'` is empty.
- [ ] **6.8 Rollout follow-ups** from D-open-14: retire the 1.x pipeline, switch asc-deploy `main`.

A low-end avatar-preview mode (D-29) is not planned; it becomes an item only if the crash or
retention numbers on low-end Android ask for it.

## Exit criteria

- [ ] Every feature a 1.15 user had is back, checked against a product checklist.
- [ ] No `NotAvailable` path remains in the sign-in flow.
- [ ] Three languages shipped.

## Depends on

The 2.0.0 release and the rollout decisions (D-open-14).
