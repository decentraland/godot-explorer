# Handoff — Thirdweb Guest: UI wiring, upgrade detection, mobile debugging

Branch: `feat/add-guest-thirdweb-ui`
Status: working-tree changes, **not committed**. Verified end-to-end on Android device (Samsung, `R5CW33DFNBX`).

This documents the work done on top of the thirdweb guest feature (PR #2179 / branch
`feat/add-guest-thirdweb`). The base branch added the silent persistent thirdweb guest
wallet, the "Upgrade to OTP" REST flow, and the device anchor. The `-ui` branch added the
polished onboarding + modal UI. This session connected the two, added "already upgraded"
detection, fixed a hard Android blocker, and added a debug affordance for testing sessions.

Design background for the base feature lives in `docs/plans/upgrade-to-otp-thirdweb.md`.

---

## TL;DR — what changed and why

1. **GuestUpgradeCard was a front-end mock; now it calls the real backend.** The new modal
   UI (`guest_upgrade_card` → `input_modal` / `code_modal`) was wired to
   `async_link_email_start` / `async_link_email_verify`. The hardcoded `111111` / `222222`
   checks are gone.
2. **The app couldn't tell a guest was already upgraded.** Added a thirdweb query
   (`GET /v1/wallets/me`) that lists linked auth methods; the Upgrade card now hides for
   guests that already linked an email/social.
3. **Two latent compile/logic bugs in the `-ui` branch were fixed** (`lobby.gd` had a
   function-name mismatch that broke the whole lobby; `code_modal.gd` had the same).
4. **Guest login could hang forever on "Getting you ready…"** — added a screen-state
   watchdog that bails to a retry prompt.
5. **A debug constant** (`DEBUG_GUEST_ANCHOR_OVERRIDE`) lets you force a fixed `user_id`
   so you can generate / reuse a specific guest wallet on any device for testing.
6. **A hard Android blocker was diagnosed and fixed** (stale extracted Godot Android
   template → PCT2 texture self-check panic → app stuck on "Getting you ready").

---

## Changes by area

### A. `lib/src/auth/thirdweb_guest.rs` — upgrade-state REST client
- `get_linked_profile_types(token) -> Vec<String>`: `GET https://api.thirdweb.com/v1/wallets/me`
  with `x-client-id` + `Authorization: Bearer <guest_jwt>` (plain JWT, no enclave prefix).
  Response shape: `{ result: { profiles: [{ type, ... }] } }`. Returns the `type`s, e.g.
  `["guest"]` or `["guest", "email"]`. Logs the parsed types at INFO.
- `account_is_upgraded(&[String]) -> bool`: `true` if any `type != "guest"`.
- Added a pure unit test (`account_is_upgraded_detects_non_guest_profiles`).

### B. `lib/src/auth/dcl_player_identity.rs` — expose upgrade state
- New field `is_thirdweb_guest_upgraded: bool` (init `false`). Cleared everywhere
  `is_thirdweb_guest` is cleared (`_update_remote_wallet`, `_update_local_wallet`, logout)
  so it never leaks across an account switch.
- `#[func] is_thirdweb_guest_upgraded()` getter + `_set_thirdweb_guest_upgraded_flag(bool)`
  deferred setter (writes land on the main thread).
- `#[func] async_refresh_thirdweb_upgrade_state(device_anchor_id) -> Promise<bool>`:
  re-derives a fresh guest token (`refresh_guest_session`, idempotent — same anchor → same
  wallet), calls `get_linked_profile_types`, updates the cached flag, resolves the bool.
  Rejects on network error (caller keeps last-known state).
- On a successful email link (`async_link_email_verify` Ok branch) it sets the upgraded
  flag `true` immediately — no need to re-query.

### C. `godot/src/ui/components/molecules/guest_upgrade_card/guest_upgrade_card.gd`
- `_async_on_email_confirmed` now calls `Global.player_identity.async_link_email_start(email)`
  (real send-code); only opens the code modal on success, surfaces a friendly error
  otherwise (`_friendly_error`, ported from the legacy popup).
- Injects a real verifier into the code modal (`code_modal.set_verifier(...)`) which calls
  `async_link_email_verify(email, code, anchor)`.
- `_async_update_visibility()` (replaces the old synchronous `_update_visibility`): starts
  hidden, and only shows the card if the account is a thirdweb guest **and** thirdweb
  reports it is **not** upgraded (`async_refresh_thirdweb_upgrade_state`). On a successful
  upgrade in-session, hides itself.

### D. `godot/src/ui/components/organisms/code_modal/code_modal.gd`
- **Bug fix:** the submit function was defined as `_async_async_submit_code` but called as
  `_async_submit_code` (a gdlint-rename artifact) → the code step was broken. Renamed to
  match.
- Replaced the mock (2s timer + `code == "222222"`) with an injected `set_verifier(Callable)`
  that returns `""` on success or a friendly error string shown inline (keeps the spinner +
  red-border UX). Falls back to emitting `confirmed` if no verifier is set (generic reuse).

### E. `godot/src/ui/pages/settings/settings.gd`
- Removed the dead `Button_UpgradeToOtp` references (`@onready var button_upgrade_to_otp`,
  `_refresh_upgrade_to_otp_visibility()`, `_on_button_upgrade_to_otp_pressed()`). The node
  was removed from `settings.tscn` in the `-ui` branch, so these would null-crash. The
  "only show for thirdweb guests" gating now lives in the card itself (see C).

### F. `godot/src/ui/pages/auth/lobby.gd`
- **Bug fix (compile blocker):** `async_async_show_avatar_create_screen` (defined) vs
  `async_show_avatar_create_screen` (called in 5 places) — same rename artifact. Renamed
  the definition; this is why the lobby script failed to load.
- **Guest-login watchdog:** `GUEST_LOGIN_TIMEOUT_SEC = 20.0`. On "Play as guest", a
  `SceneTreeTimer` is armed; if we are still on `ACCOUNT_HOME_LOADING` after the timeout
  (i.e. nothing — guest creation, profile fetch, or avatar load — navigated us away), it
  returns to Account Home and shows a "Something went wrong / TRY AGAIN" modal. A
  per-attempt token (`_guest_login_attempt`) prevents a stale watchdog from clobbering a
  fresh attempt. This catches a hang **anywhere** in the chain, not just the create-guest
  promise (which is why a simple `async_race` on the promise was insufficient).

### G. `godot/src/global.gd` — debug anchor override
- `const DEBUG_GUEST_ANCHOR_OVERRIDE: String` (near `FORCE_TEST` / `FORCE_DEEPLINK`). When
  non-empty, `get_device_anchor_id()` returns it instead of the platform anchor (Android
  SSAID / iOS Keychain UUID / desktop `user://device_anchor.txt`), with a `push_warning`.
  Any string is hashed (`compute_session_id`) into a deterministic thirdweb wallet, so the
  same `user_id` yields the same guest wallet on any device/desktop/simulator.
- **⚠️ Currently set to `"dcl-debug-user-001"` for testing — must be reset to `""` before
  any shipped build** (otherwise every user shares one debug wallet). See "Before merge".

---

## Verified on device (2026-06-22)

- **Upgrade detection works.** With a fresh debug guest: `get_linked_profile_types: ["guest"]`
  → `account_is_upgraded` false → card visible. The real wallet that had an email linked
  earlier (`link_email: email linked to guest wallet`) returns `["guest","email"]` → card
  hidden. This confirms the `result.profiles[].type` shape inferred from the openapi.
- **Debug override works.** `WARNING: [guest] DEBUG_GUEST_ANCHOR_OVERRIDE active — using
  fixed anchor: dcl-debug-user-001` fired, and the wallet changed from `0x69cb…e6e3` (real
  device anchor) to `0xe46a…1f95` (override). Note: `is_new_user=false` for
  `dcl-debug-user-001` because that exact string was already registered — use a unique
  string for a brand-new user.
- **Guest login + boot works** (after the Android template fix below): pct2 self-check
  passes, login succeeds, onboarding flows.

Validation: `cargo check` + `cargo clippy` clean, unit test passes; `gdformat` +
`gdlint` clean; `cargo run -- check-gdscript` reports the edited scripts `✓ OK` (the 3
remaining errors are pre-existing, unrelated `android/build/**` instrumented test files).

---

## Environment gotchas discovered (not code bugs)

### 1. Stale Android template → PCT2 panic → "Getting you ready" hang
The Android build hung on "Getting you ready" because the **extracted** Godot Android
runtime `godot/android/build/libs/debug/godot-lib.template_debug.aar` was older than the
editor binary (`4.6.2.stable.gh.9ee6af7ab`) and lacked PR `decentraland/godotengine#14`.
The `pct2-selfcheck` in `lib/src/content/texture.rs:578` panicked, poisoned its `Once`, and
every later texture load re-panicked → avatar/profile never completed.

`extract_android_template()` (`src/export.rs`) **skips extraction if `godot/android/`
exists** (presence check, not version), so a freshly downloaded `android_source.zip` never
replaced the stale aar. Fix (safe — `godot/android/` is untracked + gitignored):
```bash
rm -rf godot/android/
cargo run -- run --target android   # re-extracts the current aar
```

### 2. `check-gdscript` false "Could not find type X" on stale class cache
After adding new `class_name` scripts, `cargo run -- check-gdscript` emits spurious
`Parse Error: Could not find type "X"` for the new classes (`InputModal`, `CodeModal`,
`GuestUpgradeCard`, etc.) and `Could not preload` for new assets, because the headless run
loads a stale `godot/.godot/global_script_class_cache.cfg`. Fix: **open the Godot editor
once** (or run an import) to regenerate the cache, then re-run.

---

## Before merge / TODO

- [ ] **Reset `DEBUG_GUEST_ANCHOR_OVERRIDE` to `""`** in `godot/src/global.gd`. Critical.
- [ ] Revert the editor-generated `godot/project.godot` change (it drops
      `memory/limits/message_queue/max_size_mb=64`, `occlusion_culling/*`,
      `run/main_run_args`): `git checkout -- godot/project.godot`.
- [ ] **Legacy `upgrade_otp_popup` is now orphaned.** Removing the Settings
      `Button_UpgradeToOtp` means nothing emits `Global.upgrade_to_otp` anymore, so the
      `upgrade_otp_popup` in the menu (`menu.gd._on_upgrade_to_otp`, the `upgrade_to_otp`
      signal in `global.gd`) is dead/unreachable. The `GuestUpgradeCard` replaces it.
      Decide whether to delete the legacy popup + signal + menu handler.
- [ ] **Persisted thirdweb token is plaintext** in `user://thirdweb_session.json` — move to
      Keychain (iOS) / Keystore (Android). (Pre-existing follow-up from the base plan.)
- [ ] **Pre-existing avatar bug, surfaced on device** (not from this work): repeated
      `SCRIPT ERROR: Invalid assignment of property 'name_claimed' ... on a base object of
      type 'previously freed'` at `avatar.gd:447`, fired from the `-ui` avatar/preset flow
      (`_on_preset_selected`, `_async_on_profile_changed`, `show_avatar_naming_screen`,
      `async_show_avatar_create_screen`). Looks like an `avatar_preview` lifecycle race
      (the Avatar's `nickname_ui` is freed while an async update is in flight). Also
      `backpack.gd:407` `'visible' on null instance`. Non-blocking but noisy.

---

## How to test the guest + upgrade flow

1. Ensure the Android template is current (`rm -rf godot/android/` if you hit the PCT2 panic).
2. (Optional) Set `DEBUG_GUEST_ANCHOR_OVERRIDE` to a unique string to mint a fresh guest.
3. `cargo run -- run --target android` (rebuilds, exports, installs, launches, streams logcat).
4. On device: if auto-logged-in, log out / clear app data so the Account Home screen with
   "Play as guest" appears, then tap **Play as guest**.
5. Watch logcat (`adb logcat | grep -iE "guest_login|get_linked_profile_types|using fixed anchor|pct2"`):
   - `guest_login: success, address=0x…, is_new_user=…`
   - On opening discover/settings: `get_linked_profile_types: ["guest"]` → Upgrade card visible.
   - Add an email via the card → `link_email: email linked to guest wallet` → next time
     `get_linked_profile_types` returns `["guest","email"]` → card hidden.

## Changed files
- `lib/src/auth/thirdweb_guest.rs`
- `lib/src/auth/dcl_player_identity.rs`
- `godot/src/global.gd`
- `godot/src/ui/components/molecules/guest_upgrade_card/guest_upgrade_card.gd`
- `godot/src/ui/components/organisms/code_modal/code_modal.gd`
- `godot/src/ui/pages/auth/lobby.gd`
- `godot/src/ui/pages/settings/settings.gd`
- `godot/project.godot` (editor artifact — recommend revert)
