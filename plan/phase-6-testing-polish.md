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
  - [ ] **Engine start policy tuning**. The policy is D-42, built in 3.2: check its three numbers on
    the device matrix (cold launch to the first RN screen, `engine_required` to the first 3D frame,
    memory in menus with the engine idle) and adjust when `engine_required` fires.
  - [ ] **Avatar/World resource pass**. On the device matrix, record CPU and peak/steady memory in
    Idle before the preview, with the avatar visible, in the first world, and after exit. Check time
    to the first usable avatar and first playable world, including a second enter/exit cycle. Trim
    resources retained by Idle or leaked by World, and verify that deferring world services does not
    cause an unacceptable first-scene delay.
- [ ] **6.5 Performance and stability**. GP benchmark on both phones vs 1.15; memory on the
  low-end Android through a 30-minute session; thermal and fps-cap behaviour; the deliberate crash
  test for symbolication (R-7) on both platforms.
- [ ] **6.7 Native wallets (Reown AppKit)**. MetaMask and WalletConnect through Reown AppKit for RN
  (D-25, D-41), the core making the message to sign and receiving the signature; iOS only if product
  wants native WalletConnect in 2.0.0, otherwise the web flow stays. Until then wallets sign in through
  the auth page.
- [ ] **6.8 Real sign-in on devices**. Complete a real email OTP, Google, Apple and wallet login on
  both platforms (none was completed in phase 3, only the screens and the hand-off), with the
  keyboard, cancel, close-the-browser and deep-link-return paths, and the sign-in resume cap.
- [ ] **6.9 Friends in RN/TypeScript**. The core's friends API and the Rust RPC client are gone since
  phase 3 (D-43). Reimplement friends, requests, blocked users, nearby, mutual friends and online status
  in RN with TypeScript (a social service client in `rn/`), behind the existing `menu/` and `discover/`
  screens, which run on mock data until then.
- [ ] **6.6 Localization pass**. es and pt_BR strings reviewed in place on every screen.
## Verification

- The daily crash KPI from the internal builds, read every morning.
- QA scripts in the `REVIEW.md` §4 format, one per domain and per feature, executed by someone
  other than the author.

## Depends on

The full-migration gate (phases 1–5). Unblocks phase 7: new features start on the polished app.
