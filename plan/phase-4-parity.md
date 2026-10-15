# Phase 4 parity checklist (4.16)

One row per screen, modal, behaviour, persisted key and metric of `docs/rn-ui-spec/reports/legacy-godot.md`.
Columns: where it lives now, what was run on the Moto G60 (Android 16) in phase 4, and what is not done.
**iOS is unverified on every row** (no iPhone this phase). "Phase 3 pass" means it was exercised on a device
in phase 3 and not run again here. Rows marked "not done" are the work left before this checklist is green.

## Boot and entry (legacy section 1)

| Legacy | Now | Android (phase 4) | Left |
|---|---|---|---|
| Boot router, splash | Core boot + RN `EntryFlow` | Ran: cold start to entry flow | |
| Terms, sign in, guest | RN `flow/` over the core | Ran: terms, guest, name | Real OTP/Google/Apple: 6.8 |
| Avatar creation (randomize, customize) | RN `flow/previewScreens` + `dcl_core::avatar` | Ran: randomize, equip, unequip | Colour and emote slot not run |
| Deep link router | Core `deep_link_route` + RN `links.ts` | Ran: a world link | Push attribution not run |
| Version gate / update overlay | `CoreModals` update dialog (core `version_gate`) | Not run | Needs a build below the minimum |
| Device support / low-spec modals | `CoreModals` + `deviceFacts` | Not run | |
| Guest upgrade nudge | Not done | | Waits for the real logins (6.8) |
| Session chain, `App Opened` | `dcl_core::analytics::session` | Not run (events are queued) | Warehouse delivery unverified |

## Loading, discover, places, events (sections 2-3)

| Legacy | Now | Android | Left |
|---|---|---|---|
| Loading screen, tips, place card | RN `world/LoadingCover` over `loading_detail` | Ran: Genesis | |
| Loading timeout modal | `LoadingCover` slow state | Phase 3 pass | The core-driven timeout verdict is not done |
| Discover, search, categories, FTUE | RN `discover/` | Ran: Discover, place sheet | |
| Event reminders | Core `set_event_reminder` + OS reminder (`DclPush`) | Not run | Reminder delivery unverified |
| Friends rows | Mock data (D-43) | | Real friends: 6.9 |
| Last-visited list | Dropped with the GDScript discover | | Not in RN yet |

## Backpack, emotes, avatar (section 4)

| Legacy | Now | Android | Left |
|---|---|---|---|
| Backpack wearables (equip, unequip, filters) | RN `backpack/` + core rules | Ran: equip, unequip | Owned wearables and emotes (needs an account with items) |
| Colour editor | RN + core `set_color` | Not run | |
| Emote slots, emote wheel | RN `EmoteSlots` / native wheel | Ran: wheel opens, emote plays | Slot editing not run |
| Avatar preview | Engine preview (kept until 5.2) | Ran | |

## Profile, settings, account (sections 5, settings 1)

| Legacy | Now | Android | Left |
|---|---|---|---|
| Profile view and editor | RN `profile/` over the core | Phase 3 pass | |
| Settings (graphics, audio, gameplay, cache, language) | RN `settings/` + `dcl_core::settings` | Ran: nothing in Settings this phase | Persistence across relaunch not run; es/pt_BR switch not run |
| Engine-owned state (last realm, benchmark, flags) | Hidden core settings keys | Ran: world loads | |
| Account deletion, sign out | RN `AccountSection` | Phase 3 pass | |
| Developer tab | RN `DevToolsSection` | Phase 3 pass | |

## Chat, notifications, bug report, credits (settings 3-6)

| Legacy | Now | Android | Left |
|---|---|---|---|
| Chat, mentions | RN `hud/ChatPanel` | Ran: opens | Sending and mentions not run |
| Notification list, mark read | RN `menu/` + core | Phase 3 pass | In-world toasts and polling not done |
| Push (token, permission, remote push) | `DclPush` + core registration | Ran: permission prompt, grant | Identify delivery, push with the app killed |
| Bug report | RN form + `places::bug_report` | Not run | Ticket creation not run |
| Report content | External form | Phase 3 pass | |
| Credits balance | Core `credits_balance` | Phase 3 pass | Shop and purchase: deferred (4.8, 6/7) |
| Voice chat | Engine (unchanged) | Not run | |

## In-world HUD and modals (explorer section 2-4)

| Legacy | Now | Android | Left |
|---|---|---|---|
| Joystick, joypad, crosshair, pointer tooltip | Engine controls (kept) | Ran: shown; movement not driven | Movement via touch not run |
| Menu, chat, emote buttons | RN `hud/` | Ran | |
| Version and FPS line | RN `VersionFps` | Not run | |
| Hide UI | RN `HideUi` control | Not run | The sub-toggles were dropped |
| Coach marks | RN `CoachMarks` | Ran: shown | |
| Avatar tap opens a profile | Not done | | Needs a player-profile screen |
| Scene request (link, teleport, realm change, NFT) | `CoreModals` + `AnswerSceneRequest` | Not run | Needs a scene that asks; NFT card not ported |
| Private world / place not found | Destination check + engine notice | Ran: not found (deep link) | Private and password not run |
| World password | `ModalProvider` password dialog + `SetWorldSecret` | Not run | |
| Scene crash | `CoreModals` | Not run | Reload re-enters the same place |
| Disconnect, session ended, room closed, kicked | `SessionWatch` + engine notice | Not run | Ban pre-check not done |
| Low memory | Engine notice + modal | Not run | |
| Connection lost / poor connection | `dcl_core::connection` + `useConnectionQuality` | Not run | iOS buttonless second dialog dropped |
| Review prompt | `dcl_core::review` + Play card | Not run | Wearable-claimed trigger not wired |
| Back button leaves the world | `confirmLeave` | Ran: confirm shown, LEAVE returns to Discover, re-enter loads | |
| Orientation (portrait menus, landscape world) | `setOrientation` per screen | Ran: world in landscape | |
| Creator preview HUD, multiplayer debug, interactable overlay | Dropped | | Dev tools; rebuild if wanted |

## Persisted keys

Every `ConfigData` key is either a core setting (shown or hidden, `dcl_core::settings`), part of the session store, or
dropped with the code that read it (`guest_profile`, `session_account`, campaign counters, upgrade-modal and review
counters now in `review_state.json`, last-places list). The old `[user]` realm and parcel are not imported (one resume
goes to the default).

## Metrics

`App Opened`, `Push Opened`, `review_prompted` and the push identify now come from the core. The screen and button
names of the deleted GDScript screens disappeared with them; the RN screens report through `trackScreen` and
`track`. A full diff against the 106 legacy names was not done.

## Deferred on purpose

IAP and credits shop (4.8, phase 6/7), native wallets and real logins (6.7, 6.8), friends (6.9), iOS builds of every new
library (written, never built), `plugins/` deletion (5.11), emote wheel and avatar preview port (5.2).
