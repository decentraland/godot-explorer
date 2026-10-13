# The core API (shell ↔ core)

The shell talks to the core through a typed FFI generated from the Rust definitions (uniffi; see
decisions D-17 for the binding generator). Calls are synchronous or return futures the
TurboModule turns into promises; events arrive through one callback interface the host registers at
startup and forwards to JS as a single event emitter. Keys never cross into JS: the shell never sees
the ephemeral key, the wallet, the attestation token or a signed header.

## App state

One state machine, in the core, replacing `engine.ts` + `events.ts` on the RN side and
`rn_bridge.gd` mode switching on the Godot side.

```
Booting                      recovering a persisted session from the data dir
SignedOut
SigningIn { method, step }   Guest | Email { awaiting_code } | Browser { url_opened } | Wallet
SignedIn                     menus; no engine required
AvatarPreview { rect }       engine Idle with the preview; backpack, profile edit, avatar create
Loading { destination, progress }
InWorld { realm, parcel }
```

Transitions come from shell calls (`sign_in_*`, `sign_out`, `enter_world`, `exit_world`,
`show_avatar_preview`, `hide_avatar_preview`) and from engine events (`loading_finished` → InWorld,
`scene_crashed` stays InWorld and raises an event). `Booting` ends in `SignedOut` or `SignedIn`. The
state is the only thing the shell renders from; the HUD's `booting|loading|playing` phase and the
`engine.ts` `inWorld` flag disappear.

## Shell → core

| Domain | Calls | Replaces |
|---|---|---|
| State | `state()`, `on_state_changed` (event) | `engine.ts` snapshot, `world`/`ready` events |
| Session | `sign_in_guest()`, `sign_in_email_start(email)`, `sign_in_email_verify(code)`, `sign_in_browser(provider)` (opens via `open_url` event, completes on `handle_deep_link`), `sign_in_wallet(provider)` (the host runs the native wallet flow and hands the signature to the core, D-25), `sign_out()`, `delete_account()`, `profile()`, `accept_terms(version)` | `lobby.gd`, `flow/auth.ts` stub, `account.*` |
| Boot gates | `version_gate()`, `device_support()`, `pending_campaign()` | `version_gate.gd`, `device_support_coordinator.gd`, `campaigns.gd` |
| Destination | `resolve_destination(input) -> Destination` (place, world, parcel, access verdict), `enter_world(Destination)`, `exit_world()`, `teleport(parcel)` | `destination_resolver.gd`, `navigator.gd`, `enter_world`/`exit_world`/`teleport` commands |
| URLs | `urls() -> Urls` (places, events, catalyst, lambdas, marketplace, help, terms, …), `environment()` | hardcoded hosts in `rn/src`, `DclUrls` |
| Avatar | `show_avatar_preview(rect_px)`, `hide_avatar_preview()`, `set_preview_focus(target)`, `equip(urn) -> AvatarChange`, `unequip(urn)`, `set_body_shape(urn)`, `set_color(target, rgba)`, `set_emote_slot(slot, urn)`, `randomize_avatar()`, `play_emote(urn)`, `catalog(query)`, `base_emotes()`, `deploy_profile(Profile)` | `backpack_*`, `equip`, `rn_backpack.gd`, `wearables.gd` rules, `backpack/*.ts` copies, `profile.deploy` |
| Chat | `send_chat(text)`, `on_chat_message` (event), `unread_count()`, `mark_chat_read()` | `send_chat`, `chat`, `unread_chat` |
| Friends | `friends()`, `friend_requests()`, `nearby()`, `send_request`, `accept`, `reject`, `cancel`, `block`, `unblock`, `on_friends_changed` | `friends.*` |
| Notifications | `notifications(page)`, `mark_read(ids)`, `set_push_token(token)`, `on_notification` (event) | `notifications.*`, `notifications_manager.gd` |
| Places, events (signed part) | `favorite(place_id, bool)`, `vote(place_id, up)`, `set_event_reminder(id, bool)` | `places.*`, `fetch.signed` |
| Settings | `settings()`, `set_setting(key, value)`, `cache_size()`, `clear_cache()`, `version()`, `on_settings_changed` | `settings.*`, `config_data.gd` |
| Credits, IAP | `credits_balance()`, `quote(product)`, `verify_purchase(receipt)`, `marketplace_url()`; the store call itself runs in the host (StoreKit today, Play Billing with 5.4) | `iap_manager.gd` signed calls |
| Deep links | `handle_deep_link(url) -> Routed` | `deep_link_router.gd`, iOS `AppDelegate` forwarding |
| Analytics | `track(event, props)`, `track_screen(name, props)` | `Global.metrics.track_*` (~106 call sites), no Segment RN SDK |
| Reporting | `report_js_error(…)` is **not** offered; see Sentry in decisions | |
| Lifecycle (host only) | `on_background()`, `on_foreground()`, `on_low_memory()`, `attach_engine()` | Android/iOS activity callbacks |

Public read-only GETs (places list, events list, base catalog thumbnails) stay in the shell with a
typed client; the core gives the hosts. Anything that needs the identity key is a typed core call,
never a generic signed fetch.

## Core → shell (events)

`state_changed(state)`, `engine_required()` (host creates the Godot view and attaches the engine),
`avatar_preview_mode(bool)` (host flips surface order), `open_url(url, kind)` (browser, webview,
store), `scene_request(id, request)`, `chat_message`, `unread_count`, `notification`,
`friends_changed`, `settings_changed(key)`, `loading_progress`, `auth_error(code)`,
`report(event)` (Rust panic or engine error to forward to the crash reporter), `track` is not an
event (the core sends to Segment itself).

## Host responsibilities (Kotlin / Swift, not JS)

- Create `Core` at app start with the data dir the Godot-exported app used (risks R-6), the
  environment and the platform.
- Register the event callback and forward to JS.
- On `engine_required`: create the Godot view, call `Engine::start(core)`. On `avatar_preview_mode`:
  flip the surface order (`set_surface_on_top`; iOS equivalent still to be written).
- Provide the OS calls the core needs as callbacks: attestation (Play Integrity / App Attest), push
  token, store purchase result, wallet signature, clipboard, share sheet, calendar. For 2.0.0 these
  call today's Kotlin/Swift plugin code (D-21); RN libraries replace it in phase 7.
- Touch routing stays as the hosts do it today: RN when a real RN view is under the finger, Godot
  otherwise.

## No-engine builds

`cargo run -- run --target ios|android --no-godot` and `npm run android:no-godot` keep working: the
TurboModule binds to a mock core (`rn/src/core/mock.ts`) that implements the same TS interface,
replacing `mockBridge.ts` and `mockHandlers.ts`. The Expo Go workflow ends with the FFI: a custom
native module cannot load in Expo Go (decisions D-21).
