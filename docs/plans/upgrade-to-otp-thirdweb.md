# Plan: "Upgrade to OTP" — link email to a thirdweb guest wallet

> Working plan for a fresh `/clear` session. Self-contained: includes all the
> context discovered while reviewing PR #2179, the exact insertion points, the
> thirdweb REST flow, and the open questions to verify against the live API.
>
> Branch: `feat/add-guest-thirdweb` (the PR #2179 branch). Build on top of it.

---

## 0. Goal (what the user asked for)

In **Settings → Account**, add a button **"Upgrade to OTP"** as the **very first
element at the top of the Account section** (above the current top section
`SectionReport`). The button:

- Is shown **only when the active account is a thirdweb guest** wallet.
- Is **hidden** for every other account type (real WalletConnect/social wallet,
  disposable LocalWallet, or no identity).
- On press, opens a **modal** that invites the user to enter their **email**,
  then verify a **one-time code (OTP)**.
- The flow calls the **thirdweb API using the stored guest JWT** to **link** the
  email to the existing guest wallet — `/link` — so the **same wallet address**
  becomes recoverable via email afterwards.

---

## 1. Background — what PR #2179 already built (read this first)

PR #2179 ("feat: add continue as guest with thirdweb") introduced a silent,
persistent guest wallet backed by thirdweb In-App Wallets. Key facts that this
feature depends on:

- **Two distinct concepts** (do NOT conflate — there's a project rule on this):
  - **Disposable account** = random throwaway `LocalWallet`, `is_guest = true`,
    minted fresh every cold start. Created by
    `DclPlayerIdentity::create_disposable_account()`. Dev-only (double-tap).
  - **Thirdweb guest** = silent persistent wallet, deterministic per device
    anchor. Created by `async_create_guest_account(device_anchor_id)`. It is
    treated as a **real remote wallet**: `is_guest = false`. THIS is what
    "Upgrade to OTP" targets.

- **The login flow** (`lib/src/auth/dcl_player_identity.rs`,
  `perform_thirdweb_guest_login`, ~line 843):
  1. resolve device anchor (Android SSAID / iOS Keychain UUID / desktop UUID
     file) — `lib/src/auth/device_anchor.rs`.
  2. `keccak256(anchor)` → opaque thirdweb `sessionId`
     (`compute_session_id`, `device_anchor.rs:73`).
  3. `thirdweb_guest::guest_login(session_id)` →
     `POST https://api.thirdweb.com/v1/auth/complete {method:"guest", sessionId}`
     → returns `{ token (JWT), user_id, wallet_address, is_new_user }`
     (`thirdweb_guest.rs:138`).
  4. mint local ephemeral keypair + Decentraland delegation message.
  5. `thirdweb_guest::sign_message(token, addr, 1, msg)` →
     `POST https://embedded-wallet.thirdweb.com/api/v1/enclave-wallet/sign-message`
     with `Authorization: Bearer embedded-wallet-token:<JWT>` (`thirdweb_guest.rs:200`).
  6. assemble `EphemeralAuthChain`, then `try_set_remote_wallet` (deferred) →
     `_update_remote_wallet` → `is_guest = false`, emits `wallet_connected`.

- **The JWT is persisted** to `user://thirdweb_session.json` via
  `save_session_to_disk` (`thirdweb_guest.rs:94`), shape `PersistedSession {
  token, user_id, wallet_address, saved_at_unix }`. **But** `load_session_from_disk`
  (`thirdweb_guest.rs:118`) currently has **zero callers** — it's dead code we
  will give a purpose to. The JWT is **dropped from memory** after login
  (`dcl_player_identity.rs:853` comment) — so at Upgrade time we must re-read it
  from disk.

- **Constants** (`thirdweb_guest.rs:17-30`):
  - `THIRDWEB_CLIENT_ID = "e1adce863fe287bb6cf0e3fd90bdb77f"`
  - `THIRDWEB_API_BASE  = "https://api.thirdweb.com"`
  - `THIRDWEB_IAW_BASE  = "https://embedded-wallet.thirdweb.com"`
  - `THIRDWEB_ALLOWED_ORIGIN = "https://decentraland.org"` (sent as `Origin`
    header to satisfy the dashboard allowlist; bundle-id migration is a TODO)
  - `REQUEST_TIMEOUT = 20s`
  - `SESSION_PATH = "user://thirdweb_session.json"`

- **Review findings relevant here** (from the PR review):
  - The persisted JWT is **plaintext** (follow-up to move to Keychain/Keystore).
  - `load_session_from_disk` is unused; we will now use it.
  - There is currently **no way for GDScript to tell a thirdweb guest apart** from
    a normal remote wallet — `is_guest` is `false` for both. **A new marker is
    required** (see §3).

---

## 2. The thirdweb "link email" REST flow (✅ verified against live openapi.json, 2026-06-01)

Use the **unified v1 API at `api.thirdweb.com`** (same host already used by
`guest_login`). Three calls, two tokens. **All three endpoints/shapes below were
verified against `https://api.thirdweb.com/openapi.json` on 2026-06-01** — they
match. Re-diff before shipping only if much time has passed.

### Call A — initiate OTP (no auth token needed)
```
POST https://api.thirdweb.com/v1/auth/initiate
Headers: x-client-id: <THIRDWEB_CLIENT_ID>
         Origin: https://decentraland.org      (keep parity with existing calls)
         Content-Type: application/json
Body:    { "method": "email", "email": "user@example.com" }
→ 200 { "method": "email", "success": true }     (429 = rate-limited)
```

### Call B — complete OTP → get the EMAIL identity JWT
```
POST https://api.thirdweb.com/v1/auth/complete
Headers: x-client-id, Origin, Content-Type
Body:    { "method": "email", "email": "...", "code": "123456" }
→ 200 { "isNewUser": bool, "token": "<EMAIL_JWT>", "type": "email",
        "userId": "...", "walletAddress": "0x..." }
```
> ⚠️ The `walletAddress` returned here is the **email identity's own** wallet (a
> different address if the email is new). It is NOT the final address. Do not use
> it. We only need `token` (the EMAIL_JWT) for Call C.

### Call C — link email to the existing GUEST wallet (preserves address)
```
POST https://api.thirdweb.com/v1/auth/link
Headers: x-client-id, Origin, Content-Type
         Authorization: Bearer <GUEST_JWT>        ← the guest's token (from disk/refresh)
Body:    { "accountAuthTokenToConnect": "<EMAIL_JWT>" }   ← from Call B
→ 200 { "linkedAccounts": [ {type:"guest", walletAddress:"0x..."},
                            {type:"email", walletAddress:"0x..."} ] }
```
- The **bearer token identifies the surviving account** — i.e. the GUEST whose
  address is kept. The email gets merged into the guest user.
- Address preservation holds when the email is **new** (not already its own
  thirdweb user). If the email already owns a wallet, link is rejected with a
  `message` — surface that error, don't retry.

**Two tokens, don't mix them:** GUEST_JWT → `Authorization` header; EMAIL_JWT →
request **body** `accountAuthTokenToConnect`.

**Sources:** `https://api.thirdweb.com/openapi.json` (authoritative, always
current — diff against it before implementing), `https://portal.thirdweb.com/wallets/users`,
`https://portal.thirdweb.com/connect/wallet/user-management/link-multiple-identity`,
TS SDK `linkAccount.ts`.

### ✅ Verified against live openapi.json (2026-06-01)
- `/v1/auth/initiate` (method=email) → body `{method:"email", email}` (both
  required) → `{method, success}`. Security: `x-client-id` **or** `x-secret-key`.
- `/v1/auth/complete` (method=email) → body `{method:"email", email, code}` (all
  required) → `{isNewUser, token, type, userId, walletAddress}` — **`token` is
  present** (this is the EMAIL_JWT for Call C). Guest variant → `{method:"guest",
  sessionId?}` (sessionId optional). Security: `x-client-id`/`x-secret-key`.
- `/v1/auth/link` → body `{accountAuthTokenToConnect}` (required, minLength 1) →
  `{linkedAccounts: [{id, type, walletAddress}, ...]}`. Security: **BOTH**
  `x-client-id`/`x-secret-key` **AND** `Authorization: Bearer <jwt>` (the guest's
  user token — plain JWT, no enclave prefix). ✓ Plan's plain-bearer assumption
  holds.

### ⚠️ Remaining items to verify empirically (need a live guest token; not in openapi)
1. **Guest JWT freshness:** the persisted token may be expired by Upgrade time.
   Mitigation: before Call C, **refresh** the guest token by re-running
   `guest_login(session_id)` (idempotent — same anchor → same wallet → fresh
   token). This requires the device anchor at Upgrade time (pass it from GDScript
   like the lobby does, or recompute in Rust).
2. **Post-upgrade login:** confirm that future `guest_login(sessionId)` still
   returns the **same** `wallet_address` after the email is linked (it should —
   linking is additive). If thirdweb ever rebinds, the user would lose their
   wallet — this is the standing determinism risk of the whole guest design.
3. **OTP TTL** (assume ~5–10 min) and rate-limit (`429`) handling.

---

## 3. Detecting "is this a thirdweb guest?" (NEW marker — required)

`is_guest` is `false` for thirdweb guests, so it cannot gate the button. Add a
new flag + getter on `DclPlayerIdentity`.

**`lib/src/auth/dcl_player_identity.rs`:**
- Add field next to `is_guest` (~line 46): `is_thirdweb_guest: bool` (init
  `false` in `INode::init`).
- **Set it `true`** in the `async_create_guest_account` success branch — alongside
  the deferred `try_set_remote_wallet` call (~line 229), via a small deferred
  setter so it runs on the main thread (e.g. `call_deferred("_set_thirdweb_guest_flag", &[true.to_variant()])`).
- **Clear it `false`** in `_update_local_wallet` (disposable), and on logout, so
  the flag never leaks across an account switch. (In `_update_remote_wallet`,
  leave the flag controlled by its callers — a normal WalletConnect connect must
  result in `false`; the simplest robust rule is: default `_update_remote_wallet`
  sets `false`, and the thirdweb path re-sets `true` right after.)
- **Rehydrate across cold starts:** the wallet is re-derived each launch via
  `try_recover_account`, which doesn't know it was thirdweb. After a successful
  recover, set `is_thirdweb_guest = true` **iff** `load_session_from_disk()`
  exists AND its `wallet_address` matches the recovered address. This:
  (a) gives `load_session_from_disk` a real caller, and
  (b) avoids false positives if the user later upgraded to a real WalletConnect
      wallet (addresses won't match).
- Expose:
  ```rust
  #[func]
  fn is_thirdweb_guest(&self) -> bool { self.is_thirdweb_guest }
  ```

GDScript reads it as `Global.player_identity.is_thirdweb_guest()`.

---

## 4. Rust: the link API client + the two `#[func]` entry points

### 4a. `lib/src/auth/thirdweb_guest.rs` — add three REST helpers
Mirror the existing `guest_login` / `sign_message` style (reqwest client with
`REQUEST_TIMEOUT`, `x-client-id` + `Origin` headers, anyhow errors, tracing):

```rust
// Call A
pub async fn email_initiate(email: &str) -> Result<(), anyhow::Error>;
// POST {API_BASE}/v1/auth/initiate  body {method:"email", email}

// Call B → returns the EMAIL_JWT
pub async fn email_complete(email: &str, code: &str) -> Result<String, anyhow::Error>;
// POST {API_BASE}/v1/auth/complete  body {method:"email", email, code}  → .token

// Call C → links email into the guest account identified by guest_jwt
pub async fn link_email(guest_jwt: &str, email_jwt: &str) -> Result<(), anyhow::Error>;
// POST {API_BASE}/v1/auth/link
//   header Authorization: Bearer {guest_jwt}
//   body {accountAuthTokenToConnect: email_jwt}
```
Add request/response structs with `#[serde(rename = "...")]` for camelCase
fields (`accountAuthTokenToConnect`, `isNewUser`, `walletAddress`). On non-2xx,
return `anyhow!("... status={}, body={}")` like the existing helpers.

Optionally add a guest-token refresh helper that re-derives `session_id` and
calls `guest_login` to get a fresh `token` (for the freshness mitigation, §2 Q3).

### 4b. `lib/src/auth/dcl_player_identity.rs` — two Promise-returning `#[func]`s
Follow the **exact** shape of `async_create_guest_account` (line ~193):
`Promise::make_to_async()` → guard `TokioRuntime::static_clone_handle()` → reject
if missing → `handle.spawn(async move { ... let Some(mut promise) = get_promise()
else {return}; match result { Ok => resolve_with_data, Err => reject } })` →
`return promise`.

```rust
// Step 1: send the OTP to the email. No token needed.
#[func]
fn async_link_email_start(&mut self, email: GString) -> Gd<Promise>;
//   spawn → thirdweb_guest::email_initiate(&email).await
//   Ok  → resolve_with_data(true.to_variant())
//   Err → reject("Could not send code: {e}")

// Step 2: verify the code AND link to the guest wallet.
#[func]
fn async_link_email_verify(
    &mut self,
    email: GString,
    code: GString,
    device_anchor_id: GString,   // for guest-JWT refresh (§2 Q3)
) -> Gd<Promise>;
//   spawn:
//     email_jwt = thirdweb_guest::email_complete(&email, &code).await?
//     guest_jwt = load_session_from_disk().token   (refresh via guest_login if needed,
//                 using device_anchor_id → resolve_anchor → compute_session_id)
//     thirdweb_guest::link_email(&guest_jwt, &email_jwt).await?
//     // optional: persist updated session token; address is unchanged
//   Ok  → resolve_with_data(address_str.to_variant())   // unchanged guest address
//   Err → reject("Could not verify code: {e}")
```

> The device anchor is obtained in GDScript exactly like the lobby does
> (`lobby.gd:_get_device_anchor_id()` — Android `plugin.getDeviceAnchorId()`,
> iOS `plugin.get_device_anchor_id()`, desktop `""`). Reuse that helper (consider
> hoisting it to a shared util so both lobby and the modal can call it).

---

## 5. GDScript / scene work

### 5a. The Settings button (`godot/src/ui/pages/settings/`)
- **Scene** `settings.tscn`, section `VBoxContainer_Account` (node at line **1174**,
  `%`-unique). Current first child is `SectionReport` (line **1184**). Insert the
  new button as the **first child of `VBoxContainer_Account`**, above
  `SectionReport`.
  - Either a bare `Button` styled like the help/report buttons, or a `CustomButton`
    instance (`res://src/ui/components/atoms/buttons/custom_button/custom_button.tscn`,
    used by `CustomButton_SignOut` at line 1363) with `custom_text = "UPGRADE TO OTP"`.
    Prefer `CustomButton` with a `SecondaryButton`/`SecondaryOutlinedButton`
    variation for visual consistency with SignOut. Give it `unique_name_in_owner`
    (e.g. `%Button_UpgradeToOtp`).
  - Wire `pressed` → `_on_button_upgrade_to_otp_pressed` (add a `[connection]`
    near line 1409).
- **Script** `settings.gd`:
  - `@onready var button_upgrade_to_otp := %Button_UpgradeToOtp` (near line 33,
    next to `container_account`).
  - In `_ready()` (or when the account tab opens, see `_on_button_account_pressed`
    line 563), set visibility:
    ```gdscript
    button_upgrade_to_otp.visible = (
        Global.player_identity != null
        and Global.player_identity.is_thirdweb_guest()
    )
    ```
  - Handler opens the modal:
    ```gdscript
    func _on_button_upgrade_to_otp_pressed() -> void:
        Global.metrics.track_click_button("upgrade_to_otp", "settings_account", "")
        # instantiate & show the modal (5b)
    ```

### 5b. The Upgrade-to-OTP modal (new component)
Create a new multi-step modal, modeled on the **account deletion popup**
(`godot/src/ui/components/organisms/menu/account_deletion_popup.{tscn,gd}`) which
already implements the multi-screen pattern (confirm → processing → done → fail).
The simple `modal` component
(`godot/src/ui/components/organisms/modal/modal.{tscn,gd}`) is the fallback if a
lighter dialog is enough, but we need text input + two steps, so the deletion
popup pattern fits better.

Suggested location: `godot/src/ui/components/organisms/menu/upgrade_otp_popup.{tscn,gd}`.

Steps / sub-screens:
1. **Email entry** — a `DclLineEdit`
   (`res://src/ui/components/atoms/inputs/dcl_line_edit.tscn`, has built-in error
   label) for the email + a primary "SEND CODE" button + cancel.
2. **Processing** — reuse the loading spinner from the deletion popup.
3. **Code entry** — a `DclLineEdit` for the 6-digit OTP + "VERIFY" button +
   a "resend code" affordance (re-calls step 1's start).
4. **Success** — "Your email is linked. You can now recover this account." Close.
5. **Error** — show `result.get_error()`, allow retry / back.

Submit handlers use the **PromiseUtils idiom** (same as
`lobby.gd:_on_button_continue_as_guest_pressed`, line 743):
```gdscript
# gdlint:ignore = async-function-name
func _on_send_code_pressed() -> void:
    _set_busy(true)
    var promise: Promise = Global.player_identity.async_link_email_start(email_field.get_text())
    var result = await PromiseUtils.async_awaiter(promise)
    _set_busy(false)
    if result is PromiseError:
        _show_error(result.get_error()); return
    _show_code_step()

# gdlint:ignore = async-function-name
func _on_verify_pressed() -> void:
    _set_busy(true)
    var anchor: String = _get_device_anchor_id()   # shared helper (see §4b)
    var promise: Promise = Global.player_identity.async_link_email_verify(
        email_field.get_text(), code_field.get_text(), anchor)
    var result = await PromiseUtils.async_awaiter(promise)
    _set_busy(false)
    if result is PromiseError:
        _show_error(result.get_error()); return
    _show_success()   # address unchanged; optionally refresh profile UI
```

> Note: unlike the lobby (which only disables a button), this modal MUST show a
> busy/spinner state and keep the submit disabled until the promise resolves, and
> should guard against the user closing mid-request. (This mirrors a MUST-FIX the
> PR review raised about the lobby's missing loading UI — don't repeat it here.)

---

## 6. Edge cases & decisions to confirm with the user / product
- **Already-linked guest:** if the guest already linked an email (re-upgrade),
  Call C returns an error. Detect and show "this account already has an email
  linked" instead of a raw error. (Optionally hide/disable the button if we can
  know it's already linked — but we have no local flag for that yet; out of scope
  unless desired.)
- **Email belongs to another thirdweb user:** link rejected → friendly error.
- **Token expired / offline:** surface a retryable error; the refresh-via-
  `guest_login` step covers the expiry case if the anchor is available.
- **Desktop:** anchor is the `user://device_anchor.txt` UUID; flow works the same.
- **Wording:** keep "guest" = thirdweb-persistent (consistent with PR #2179). The
  button says "UPGRADE TO OTP" (or "SECURE YOUR ACCOUNT" / "ADD EMAIL RECOVERY" —
  confirm copy with the user).
- **Analytics:** add a `track_click_button("upgrade_to_otp", ...)` and consider
  success/failure events.

---

## 7. Suggested implementation order
1. **Rust marker** (§3): `is_thirdweb_guest` field + setter + getter + rehydrate
   from `load_session_from_disk`. Build, confirm `is_thirdweb_guest()` is bindable.
2. **Verify thirdweb endpoints live** (§2 open questions) — curl/openapi.json
   against the real API with a test guest token BEFORE writing the client.
3. **Rust link client** (§4a) + the two `async_link_email_*` `#[func]`s (§4b).
   Add `#[ignore]` integration tests like the existing thirdweb ones.
4. **Settings button** (§5a) — visibility gated on `is_thirdweb_guest()`.
5. **Modal** (§5b) — multi-step, PromiseUtils idiom, busy state.
6. **End-to-end test** on a thirdweb-guest session: button appears → email →
   OTP → success → confirm wallet address unchanged and email-recovery works on a
   fresh install.
7. `cd lib && cargo fmt --all && cargo clippy -- -D warnings`; `gdformat godot/`
   and `gdlint godot/` (whole folder).

---

## 8. Key file references (cheat sheet)
| What | File:line |
|---|---|
| thirdweb REST client (extend here) | `lib/src/auth/thirdweb_guest.rs` (constants 17-30, `guest_login` 138, `sign_message` 200, `load_session_from_disk` 118 — unused) |
| identity, add marker + `#[func]`s | `lib/src/auth/dcl_player_identity.rs` (`is_guest` 46, `async_create_guest_account` 193, `_update_remote_wallet` 113, `perform_thirdweb_guest_login` 843) |
| device anchor | `lib/src/auth/device_anchor.rs` (`resolve_anchor` 60, `compute_session_id` 73) |
| Promise pattern (Rust) | `lib/src/godot_classes/promise.rs` (`make_to_async` 87, `resolve_with_data`, `reject` 60) |
| Promise idiom (GDScript) | `godot/src/utils/promise.gd` (`async_awaiter` 30); example `godot/src/ui/pages/auth/lobby.gd:743` |
| device anchor in GDScript | `godot/src/ui/pages/auth/lobby.gd:_get_device_anchor_id` ~730 |
| Settings scene — Account section | `godot/src/ui/pages/settings/settings.tscn` (`VBoxContainer_Account` 1174, top child `SectionReport` 1184, `CustomButton_SignOut` 1363, connections ~1409) |
| Settings script | `godot/src/ui/pages/settings/settings.gd` (`container_account` 33, `_on_button_account_pressed` 563, `_on_button_delete_account_pressed` 568) |
| Modal patterns | `godot/src/ui/components/organisms/modal/modal.{tscn,gd}`; multi-step: `godot/src/ui/components/organisms/menu/account_deletion_popup.{tscn,gd}` |
| Inputs | `godot/src/ui/components/atoms/inputs/dcl_line_edit.tscn` (has error label) |
| Button component | `godot/src/ui/components/atoms/buttons/custom_button/custom_button.tscn` |
