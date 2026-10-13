# Phase 4: screens, domain by domain

December 2026, full team. Largest volume, lowest risk per step.

## Goal

Every screen runs in RN on core APIs. Each domain, when it lands, deletes its GDScript screen, its
`req` methods and any rule it had copied into TypeScript. The RN screens exist already (built from
Figma, `docs/rn-ui-spec/`); the work is the core API behind them, the rewiring, the deletion and the
QA pass on both phones.

Per domain, the same five steps: core API typed (rules and signed requests) → RN screen rewired from
`req` to `rn/src/core` → GDScript deleted → metrics taxonomy checked against
`docs/rn-ui-spec/reports/legacy-godot.md` → QA on Android and iOS. One PR per domain, or two when
the core part is large.

## Exit criteria

- [ ] No `.gd` under `godot/src/ui`, `godot/src/iap`, `godot/src/auth`, `godot/src/config`,
      `godot/src/notifications`, `godot/src/bug_report`; `godot/src/logic` keeps only
      `scene_fetcher.gd` and `player/` for phase 5.
- [ ] `global.gd` creates no UI and no manager; `modal_manager.gd`, `explorer.gd`'s UI and the
      `SplashOverlay`, `NotificationsManager`, `Iap`, `MarketplaceTracker`, `SessionTracker`,
      `TouchFeedback`, `UiSounds` autoloads are gone.
- [ ] `rn/src/backpack/{taxonomy,emotes,catalog,platformPolicy}.ts`, `rn/src/settings/keys.ts` and
      `rn/src/flow/flowMachine.ts` are gone or reduced to presentation.
- [ ] Every domain QA'd on both phones with a signed-in account (friends, notifications, favorites,
      reminders, account deletion have never been tested that way).

## Work items, in order

- [ ] **4.1 Friends and blacklist** (M). Core is pure since 1.2. RN `menu/` social screens on
  `core.friends*`; online status and location from comms presence. Delete `ui/pages/friends`,
  `ui/pages/discover/friends`, `organisms/social`, `friends_generator`, `social_list`.
- [ ] **4.2 Discover, places, events** (M). RN `discover/` (5.2k) exists. Public GETs stay in RN
  with hosts from `core.urls()`; favorites, votes, reminders become typed core calls;
  `resolve_destination` for Jump In (from 1.5). Delete `ui/pages/discover`, `ui/pages/events`,
  `place_item`, `featured_data_provider`, `fav_button`, `engagement_bar`.
- [ ] **4.3 Backpack and equip rules** (L). `wearables.gd` (681) and the rules in `backpack.gd`
  (1,267) → `dcl_core::avatar`: categories and slots, body-shape rules, forced defaults for eyes,
  mouth and eyebrows, base emotes, emote slots, "can I equip X" returning the resulting avatar, base
  catalog and owned items, profile deploy. The engine renders through `set_self_profile`. RN
  `backpack/` drops its copies. Delete `ui/pages/backpack`, the lobby avatar editor and
  `lambda_wearable_request`; `organisms/emotes` (the emote wheel) ports to the engine in 5.3.
- [ ] **4.4 Chat** (M). Chat through core events (comms in the core since 1.7); nearby, mentions,
  commands (`ChatCommands`), speech bubbles stay engine-side. RN `hud/` chat exists. Delete
  `ui/pages/chat` and the chat organisms.
- [ ] **4.5 Settings** (M). `config_data.gd` (767) schema → `dcl_core::settings` with the one-time
  import of `user://settings.cfg` (decisions D-18); the keys in `rn/src/settings/keys.ts` are the
  contract; the engine reads graphics, audio and gameplay values from the core; benchmark results and
  the dynamic graphics preset are written by the engine through core events. RN `settings/` exists.
  Delete `ui/pages/settings` (1,644), `config/`, `dcl_config.rs`'s `ConfigFile`.
- [ ] **4.6 Notifications** (M). Core poll and read state from 1.9; RN renders the list and toasts.
  OS push and local notifications are out of 2.0.0 and return in phase 6 (D-21). Delete
  `notifications_manager.gd` (1,539), `organisms/notifications`, `lib/src/notifications.rs`.
- [ ] **4.7 Profile** (M). RN `profile/` exists; names, deploy, snapshots through the core. Delete
  `ui/pages/profile` (2,580), `logic/player/player_identity.gd`.
- [ ] **4.8 IAP and credits** (S). Out of 2.0.0 (D-21): delete `iap/` (`iap_manager.gd` 1,078,
  `marketplace_tracker.gd` 363), `DclStoreKitPlugin`, `dcl_swift_lib`'s StoreKit part, the `Iap`
  and `MarketplaceTracker` autoloads and the credits UI. Store purchase in the shell and the
  credits logic in the core return in phase 6.
- [ ] **4.9 Modals and scene requests** (M). The 30 kinds in `modal_manager.gd` (1,900) become RN
  modals; the ones with policy (scene requests from 2.3, guest upgrade nudge, version gate, device
  support, scene timeout, discard-edit confirm) take their verdicts from the core. Delete
  `modal_manager.gd`, `organisms/modal`, `organisms/dialogs`, `upgrade_modal`,
  `update_available`.
- [ ] **4.10 Loading, HUD, menu** (M). RN `hud/`, `world/`, `menu/` exist; loading progress, place
  card and tips from core events; the emote wheel stays in the engine (D-24, item 5.3); coach marks; the
  version/FPS line moves to RN from core/engine state. Delete `ui/pages/loading_screen`,
  `organisms/menu`, `explorer.gd`'s UI half, `layouts/`, `SplashOverlay`, `TouchFeedback`,
  `UiSounds`.
- [ ] **4.11 Small coordinators** (S each). `session_tracker.gd` (App Opened chaining,
  `session_state.cfg`), `campaigns.gd`, `review_prompt_coordinator.gd`,
  `upgrade_nudge_coordinator.gd` (18 h / 5 d / 5 d schedule), `bug_report/` (Intercom proxy, log
  attachment) → core logic, RN UI. `connection_quality_monitor.gd` → core.
- [ ] **4.12 Analytics taxonomy** (S). The ~106 GDScript track calls disappear with their screens;
  RN tracks through `core.track` / `core.track_screen`. Diff the screen and button names against
  `legacy-godot.md` before each domain closes.
- [ ] **4.13 Hosts** (S). Whatever 3.8 left: `discover/api.ts`, `discover/eventTime.ts`,
  `backpack/catalog.ts`, `backpack/platformPolicy.ts`, `settings/links.ts`, `hud/Hud.tsx`,
  `flow/screens.tsx`, `menu/api.ts` read `core.urls()`.
- [ ] **4.14 Android back button, orientation, IME** (S). Back closes RN sheets, then asks the core
  (`exit_world` confirm); orientation stays RN-owned (portrait before a scene, landscape in it);
  virtual-keyboard height from the host to the core for scene inputs.

## Verification

- QA script per domain, written with the domain's PR in the `## Test plan` format of `REVIEW.md`
  §4, run on a Moto Edge 60 Pro and an iPhone 14 Pro from the TestFlight/R2 builds.
- `cargo test -p dcl_core` for equip rules, settings import, credits, scene-request policy.
- Jest against `rn/src/core/mock.ts`.

## Depends on / unblocks

Depends on phase 3 (FFI). 4.3 depends on 5.1 only for the Idle preview's quality, not for landing:
until 5.1 the preview is the GDScript `avatar_preview.tscn` behind `Engine::show_avatar_preview`.
Unblocks the January release.
