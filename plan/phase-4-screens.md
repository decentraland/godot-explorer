# Phase 4: the UI sweep, then every domain on the core

Ends feature-complete against 1.15.

## Goal

The GDScript UI is deleted in one sweep, then every screen runs in RN on core APIs, with no feature
a 1.15 user had missing: push notifications, in-app purchases on iOS, share, calendar, three
languages, every modal. The RN screens exist already (built from Figma, `docs/rn-ui-spec/`); the work
is the core API behind them, the rewiring, and the parity check.

Why one sweep and not domain by domain: after phase 3 the RN app is the only product on the branch,
so every GDScript screen is dead weight that still costs — `explorer.tscn` keeps instantiating a
HUD nobody sees, `global.gd` stays 1.8k lines of glue, and the daily `main` merge keeps meeting
files about to be deleted. One bounded job, then pure wiring.

## Exit criteria

- [ ] No `.gd` under `godot/src/ui` except the engine-owned controls listed in 4.0, none under
      `godot/src/iap`, `godot/src/auth`, `godot/src/config`, `godot/src/notifications`,
      `godot/src/bug_report`; `godot/src/logic` keeps only `scene_fetcher.gd`, `player/` and what
      the engine-side GDScript needs, for phase 5.
- [ ] `global.gd` creates no UI and no manager; `modal_manager.gd`, `explorer.gd`'s UI half and the
      `SplashOverlay`, `NotificationsManager`, `Iap`, `MarketplaceTracker`, `SessionTracker`,
      `TouchFeedback`, `UiSounds` autoloads are gone.
- [ ] `rn/src/backpack/{taxonomy,emotes,catalog,platformPolicy}.ts`, `rn/src/settings/keys.ts` and
      `rn/src/flow/flowMachine.ts` are gone or reduced to presentation.
- [ ] **The parity checklist (4.16) is green**: every screen, modal, behaviour and metric in
      `docs/rn-ui-spec/reports/legacy-godot.md` has an RN equivalent, checked on an Android phone
      and an iPhone with a signed-in account.

## Work items, in order

- [ ] **4.0 The sweep**. Delete `godot/src/ui` wholesale except the engine-owned controls
  (scene UI container, safe-area controls, joystick, gamepad, emote wheel, mobile camera input,
  pointer tooltip, crosshair), `modal_manager.gd`, `explorer.gd`'s UI half, `iap/`, `auth/`,
  `config/` UI, `notifications_manager.gd`, `bug_report/`, the UI autoloads above, and
  `rn_ui_filter.gd` if 3.8 left it. Fix what breaks in the engine-side GDScript: `Global.modal_manager`
  and `loading_screen` references, chat-open and virtual-keyboard state read by `player.gd`,
  nameplate and scene-UI hooks in `explorer.gd`; each becomes a core call, an `Engine` call or
  nothing. Exit: `--itest`/`--stest` green, the RN app enters a world, `global.gd` well under 1k lines.
- [ ] **4.1 Friends and blacklist**. Core is pure since 1.2. RN `menu/` social screens on
  `core.friends*`; online status and location from comms presence through the engine sink.
- [ ] **4.2 Discover, places, events**. RN `discover/` (5.2k) exists. Public GETs stay in RN
  with hosts from `core.urls()`; favorites, votes, reminders become typed core calls;
  `resolve_destination` for Jump In (from 1.5).
- [ ] **4.3 Backpack and equip rules**. `wearables.gd` (681) and the rules in `backpack.gd`
  (1,267) → `dcl_core::avatar`: categories and slots, body-shape rules, forced defaults for eyes,
  mouth and eyebrows, base emotes, emote slots, "can I equip X" returning the resulting avatar, base
  catalog and owned items, profile deploy. The engine renders through `set_self_profile`; the Idle
  preview is the GDScript `avatar_preview.tscn` behind `Engine::show_avatar_preview` until 5.2. RN
  `backpack/` drops its copies.
- [ ] **4.4 Chat**. Chat through core events fed by the engine sink (transports stay in the
  engine until 5.1); nearby, mentions, commands (`ChatCommands`), speech bubbles stay engine-side. RN
  `hud/` chat exists.
- [ ] **4.5 Settings**. `config_data.gd` (767) schema → `dcl_core::settings` (own JSON, one-time
  import of `user://settings.cfg`, D-18); the keys in `rn/src/settings/keys.ts` are the contract;
  the engine reads graphics, audio and gameplay values from the core; benchmark results and the
  dynamic graphics preset are written by the engine through core events; `cache_size` and
  `clear_cache` work on the cache dir with no engine running. RN `settings/` exists. Delete
  `dcl_config.rs`'s `ConfigFile`.
- [ ] **4.6 Notifications, including push**. Core poll and read state from 1.9; RN renders the
  list and toasts; local OS notifications and the push token through an RN notifications library (D-41),
  checked for: data-only FCM messages shown with the app killed (headless task on Android), the same
  token kinds as today (FCM on Android, raw APNs on iOS), the same Android channel ids, and the
  reminders 1.x scheduled (rescheduled at the first 2.0 launch, or dropped by decision); `core.set_push_token`; push attribution in deep-link routing. Delete
  `lib/src/notifications.rs`.
- [ ] **4.7 Profile**. RN `profile/` exists; names, deploy, snapshots through the core. Delete
  `logic/player/player_identity.gd`.
- [ ] **4.8 IAP and credits on iOS**. Quote, verify, balance and the marketplace round-trip in
  `dcl_core::credits` (from `iap_manager.gd` 1,078 and `marketplace_tracker.gd` 363); StoreKit
  through one RN IAP library, its transaction listener started at app launch (D-41); the credits UI
  in RN. Android IAP is new and lands in phase 7 (7.4) on the same library. `dcl-swift-lib` is deleted.
- [ ] **4.9 Modals and scene requests**. The 30 kinds in `modal_manager.gd` (1,900) become RN
  modals; the ones with policy (scene requests from 2.3, guest upgrade nudge, version gate, device
  support, scene timeout, discard-edit confirm) take their verdicts from the core.
- [ ] **4.10 Loading, HUD, menu**. RN `hud/`, `world/`, `menu/` exist; loading progress, place
  card and tips from core events; the emote wheel stays in the engine (D-24, item 5.4); coach
  marks; the version/FPS line moves to RN from core/engine state.
- [ ] **4.11 Small coordinators, share, calendar**. `session_tracker.gd` (App Opened
  chaining, `session_state.cfg`), `campaigns.gd`, `review_prompt_coordinator.gd`,
  `upgrade_nudge_coordinator.gd` (18 h / 5 d / 5 d schedule), `bug_report/` (Intercom proxy, log
  attachment), `connection_quality_monitor.gd` → core logic, RN UI. Share sheet, calendar,
  review prompt and the bug-report image picker through RN libraries; install referrer, Firebase
  Analytics and the deferred deep link through RN libraries too (D-41).
- [ ] **4.12 Analytics taxonomy**. The ~106 GDScript track calls disappear with their screens;
  RN tracks through `core.track` / `core.track_screen`. Diff the screen and button names against
  `legacy-godot.md`.
- [ ] **4.13 Hosts**. Whatever 3.8 left: `discover/api.ts`, `discover/eventTime.ts`,
  `backpack/catalog.ts`, `backpack/platformPolicy.ts`, `settings/links.ts`, `hud/Hud.tsx`,
  `flow/screens.tsx`, `menu/api.ts` read `core.urls()`.
- [ ] **4.14 Android back button, orientation, IME**. Back closes RN sheets, then asks the core
  (`exit_world` confirm); orientation stays RN-owned (portrait before a scene, landscape in it);
  virtual-keyboard height from the host to the core for scene inputs.
- [ ] **4.15 Localization**. `i18n/es` and `i18n/pt_BR` generated from `godot/locale/*.csv`
  (keys already match `en.csv`); a CI check for missing keys; language switch in Settings (D-26).
- [ ] **4.16 Parity checklist**. One table from
  `docs/rn-ui-spec/reports/legacy-godot.md`: every screen, modal, behaviour, persisted key and
  metric name, with its RN location and a pass/fail per platform. It is the feature-complete milestone, half of the full-migration gate, and
  the backbone of phase 6's test plan.

## Verification

- `cargo test -p dcl_core` for equip rules, settings import, credits, scene-request policy.
- Jest against `rn/src/core/mock.ts`.
- The parity checklist on a Moto Edge 60 Pro and an iPhone 14 Pro from the TestFlight/R2 builds,
  with a signed-in account (friends, notifications, favorites, reminders, account deletion have
  never been tested that way).

## Depends on / unblocks

Depends on phase 3 (FFI). Unblocks phases 7 and 6: the team starts from a feature-complete
app. Overflow from this phase is the first thing the team absorbs (risks R-8).
