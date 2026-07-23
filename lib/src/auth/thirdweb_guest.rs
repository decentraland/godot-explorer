//! Thirdweb In-App Wallet — Guest mode REST client.
//!
//! Issues a deterministic wallet keyed by an opaque `sessionId` and signs
//! messages on its behalf. Used by the silent guest-login flow that anchors
//! the `sessionId` to a stable device identifier.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ethers_core::types::H160;
use godot::classes::file_access::ModeFlags;
use godot::classes::FileAccess;
use godot::prelude::GString;
use serde::{Deserialize, Serialize};

use super::wallet::AsH160;

const THIRDWEB_CLIENT_ID: &str = "e1adce863fe287bb6cf0e3fd90bdb77f";
const THIRDWEB_API_BASE: &str = "https://api.thirdweb.com";
/// In-app-wallet host. Different from `api.thirdweb.com` — that one is the
/// server-side Engine API that requires `x-secret-key`; this one is the
/// enclave-wallet service the client SDKs talk to with just the user JWT.
const THIRDWEB_IAW_BASE: &str = "https://embedded-wallet.thirdweb.com";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

/// Origin allowlisted in the thirdweb dashboard project
/// (https://thirdweb.com/dcl/POC-Explorer-e1adce/settings). Sent as the
/// `Origin` header so requests are accepted without registering each
/// platform's bundle ID separately. Switch to `x-bundle-id` once the bundle
/// IDs are added to the dashboard allowlist.
const THIRDWEB_ALLOWED_ORIGIN: &str = "https://decentraland.org";

#[derive(Debug, Serialize)]
struct GuestLoginRequest<'a> {
    method: &'a str,
    #[serde(rename = "sessionId")]
    session_id: &'a str,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GuestLoginResponse {
    token: String,
    user_id: String,
    wallet_address: String,
    #[serde(default)]
    is_new_user: bool,
}

#[derive(Debug, Serialize)]
struct SignMessagePayload<'a> {
    message: &'a str,
    #[serde(rename = "isRaw")]
    is_raw: bool,
    #[serde(rename = "chainId")]
    chain_id: u64,
}

#[derive(Debug, Serialize)]
struct SignMessageRequest<'a> {
    #[serde(rename = "messagePayload")]
    message_payload: SignMessagePayload<'a>,
}

#[derive(Debug, Deserialize)]
struct SignMessageResponse {
    signature: String,
}

#[derive(Debug, Serialize)]
struct EmailInitiateRequest<'a> {
    method: &'a str,
    email: &'a str,
}

#[derive(Debug, Serialize)]
struct EmailCompleteRequest<'a> {
    method: &'a str,
    email: &'a str,
    code: &'a str,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EmailCompleteResponse {
    token: String,
    /// The email identity's own wallet address. Used for login; ignored for
    /// the upgrade/link flow (the guest address is preserved instead).
    wallet_address: String,
}

#[derive(Debug, Serialize)]
struct LinkAccountRequest<'a> {
    #[serde(rename = "accountAuthTokenToConnect")]
    account_auth_token_to_connect: &'a str,
}

#[derive(Debug, Clone)]
pub struct ThirdwebGuestSession {
    pub token: String,
    pub user_id: String,
    pub wallet_address: H160,
    pub is_new_user: bool,
}

/// Typed failure for `guest_login`, carrying a machine-stable `reason` code so
/// analytics can bucket outcomes without parsing free-form error strings (the
/// `?`/`anyhow` boundary erases whether a failure was a timeout, a network
/// error, or an HTTP status). Implements `std::error::Error` so existing
/// `anyhow`-based callers keep working via `?` / `Into`.
#[derive(Debug)]
pub struct GuestLoginError {
    /// A machine-stable code. This function produces "http_4xx", "http_5xx",
    /// "timeout", "network", "bad_response" or "invalid_address"; later steps
    /// of the guest-login flow (see `perform_thirdweb_guest_login`) may also
    /// construct this error with "sign_message" or "ephemeral". Kept as a
    /// `&'static str` so the vocabulary is fixed at the call site.
    pub reason: &'static str,
    /// Upstream HTTP status when `reason` is "http_4xx"/"http_5xx". None otherwise.
    pub http_status: Option<u16>,
    /// Human-readable detail for logs (may include the upstream body). Not for
    /// analytics grouping — use `reason`.
    pub message: String,
}

impl std::fmt::Display for GuestLoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.http_status {
            Some(status) => write!(f, "{} (status={}): {}", self.reason, status, self.message),
            None => write!(f, "{}: {}", self.reason, self.message),
        }
    }
}

impl std::error::Error for GuestLoginError {}

/// Classifies a reqwest transport error (no HTTP response was produced) into a
/// stable `reason` code. Status-based codes are handled separately by the
/// caller, which has the response in hand.
fn classify_reqwest_error(e: &reqwest::Error) -> &'static str {
    if e.is_timeout() {
        "timeout"
    } else {
        "network"
    }
}

const SESSION_PATH: &str = "user://thirdweb_session.json";

#[derive(Debug, Serialize, Deserialize)]
struct PersistedSession {
    token: String,
    user_id: String,
    wallet_address: String,
    saved_at_unix: u64,
}

/// Persists the thirdweb JWT alongside the wallet address so subsequent
/// launches can renew the local ephemeral delegation by calling
/// `sign_message` again, without paying the round trip to `/v1/auth/complete`.
/// The JWT lives in the user data dir as plaintext JSON — fine for V1 (same
/// trust level as the rest of `user://settings.ini`); a follow-up should
/// move this into Keychain (iOS) / Keystore (Android) for parity with the
/// platform-secure stores we already use for the device anchor.
pub fn save_session_to_disk(session: &ThirdwebGuestSession) -> Result<(), anyhow::Error> {
    let payload = PersistedSession {
        token: session.token.clone(),
        user_id: session.user_id.clone(),
        wallet_address: format!("{:#x}", session.wallet_address),
        saved_at_unix: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
    };
    let json = serde_json::to_string(&payload)?;

    let path = GString::from(SESSION_PATH);
    let mut file = FileAccess::open(&path, ModeFlags::WRITE)
        .ok_or_else(|| anyhow::anyhow!("failed to open {} for write", SESSION_PATH))?;
    file.store_string(&GString::from(&json));
    file.close();
    Ok(())
}

/// Reads the previously persisted JWT. Returned `None` means "no session
/// saved yet" — the caller should kick off a fresh `guest_login`. The token
/// is not validated against thirdweb here; expiration is enforced by the
/// API when it's used.
pub fn load_session_from_disk() -> Option<ThirdwebGuestSession> {
    let path = GString::from(SESSION_PATH);
    if !FileAccess::file_exists(&path) {
        return None;
    }
    let mut file = FileAccess::open(&path, ModeFlags::READ)?;
    let content = file.get_as_text().to_string();
    file.close();
    let payload: PersistedSession = serde_json::from_str(&content).ok()?;
    let wallet_address = payload.wallet_address.as_str().as_h160()?;
    Some(ThirdwebGuestSession {
        token: payload.token,
        user_id: payload.user_id,
        wallet_address,
        is_new_user: false,
    })
}

/// Logs in as a guest with a deterministic session id. The same session id
/// always returns the same wallet address (server-side, custodial).
pub async fn guest_login(session_id: &str) -> Result<ThirdwebGuestSession, GuestLoginError> {
    let url = format!("{}/v1/auth/complete", THIRDWEB_API_BASE);
    let body = GuestLoginRequest {
        method: "guest",
        session_id,
    };

    tracing::debug!(
        "thirdweb guest_login: session_id_len={}, url={}",
        session_id.len(),
        url
    );

    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|e| GuestLoginError {
            reason: "network",
            http_status: None,
            message: format!("client build failed: {e}"),
        })?;

    let response = client
        .post(&url)
        .header("x-client-id", THIRDWEB_CLIENT_ID)
        .header("Origin", THIRDWEB_ALLOWED_ORIGIN)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| GuestLoginError {
            reason: classify_reqwest_error(&e),
            http_status: None,
            message: e.to_string(),
        })?;

    let status = response.status();
    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(GuestLoginError {
            reason: if status.is_server_error() {
                "http_5xx"
            } else {
                "http_4xx"
            },
            http_status: Some(status.as_u16()),
            message: format!("status={status}, body={text}"),
        });
    }

    let parsed: GuestLoginResponse = response.json().await.map_err(|e| GuestLoginError {
        reason: "bad_response",
        http_status: Some(status.as_u16()),
        message: format!("failed to parse response body: {e}"),
    })?;
    let address = parsed
        .wallet_address
        .as_str()
        .as_h160()
        .ok_or_else(|| GuestLoginError {
            reason: "invalid_address",
            http_status: None,
            message: "thirdweb returned invalid wallet address".into(),
        })?;

    tracing::info!(
        "thirdweb guest_login: success, address={:#x}, is_new_user={}",
        address,
        parsed.is_new_user
    );

    Ok(ThirdwebGuestSession {
        token: parsed.token,
        user_id: parsed.user_id,
        wallet_address: address,
        is_new_user: parsed.is_new_user,
    })
}

/// Signs an arbitrary plain-text message using the guest enclave wallet. The
/// signature is EIP-191 (personal_sign) and verifiable against the wallet
/// address. Hits the in-app-wallet enclave service (different host from
/// `api.thirdweb.com`) which accepts the user JWT directly, prefixed with
/// `embedded-wallet-token:` inside the Bearer scheme — this prefix is what
/// distinguishes the client-side path from the server-side Engine API that
/// requires `x-secret-key`.
pub async fn sign_message(
    token: &str,
    from: H160,
    chain_id: u64,
    message: &str,
) -> Result<String, anyhow::Error> {
    let url = format!("{}/api/v1/enclave-wallet/sign-message", THIRDWEB_IAW_BASE);
    let body = SignMessageRequest {
        message_payload: SignMessagePayload {
            message,
            is_raw: false,
            chain_id,
        },
    };

    tracing::debug!(
        "thirdweb sign_message: from={:#x}, chain_id={}, message_len={}",
        from,
        chain_id,
        message.len()
    );

    let response = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()?
        .post(&url)
        .header("x-thirdweb-client-id", THIRDWEB_CLIENT_ID)
        .header("Origin", THIRDWEB_ALLOWED_ORIGIN)
        .header(
            "Authorization",
            format!("Bearer embedded-wallet-token:{}", token),
        )
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!(
            "thirdweb sign_message failed: status={}, body={}",
            status,
            text
        ));
    }

    let parsed: SignMessageResponse = response.json().await?;
    tracing::debug!(
        "thirdweb sign_message: signature_len={}",
        parsed.signature.len()
    );
    Ok(parsed.signature)
}

/// Refreshes the guest JWT by re-deriving the `sessionId` from the device
/// anchor and re-running `guest_login`. This is idempotent — the same anchor
/// always yields the same wallet — so it's safe to call at Upgrade time to
/// guarantee a non-expired token for the `/link` call (the persisted one may
/// have aged out). Returns the full session so the caller can persist the
/// fresh token.
pub async fn refresh_guest_session(
    device_anchor_id: &str,
) -> Result<ThirdwebGuestSession, anyhow::Error> {
    let anchor = super::device_anchor::resolve_anchor(device_anchor_id);
    let session_id = super::device_anchor::compute_session_id(&anchor);
    guest_login(&session_id).await.map_err(anyhow::Error::from)
}

/// Call A — sends a one-time code to `email`. No auth token required; the
/// project is identified by `x-client-id` alone. A `429` means the address is
/// rate-limited (surface it; don't auto-retry).
pub async fn email_initiate(email: &str) -> Result<(), anyhow::Error> {
    let url = format!("{}/v1/auth/initiate", THIRDWEB_API_BASE);
    let body = EmailInitiateRequest {
        method: "email",
        email,
    };

    tracing::debug!("thirdweb email_initiate: url={}", url);

    let response = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()?
        .post(&url)
        .header("x-client-id", THIRDWEB_CLIENT_ID)
        .header("Origin", THIRDWEB_ALLOWED_ORIGIN)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!(
            "thirdweb email_initiate failed: status={}, body={}",
            status,
            text
        ));
    }

    tracing::info!("thirdweb email_initiate: code sent");
    Ok(())
}

/// Call B — verifies the OTP and returns the EMAIL identity JWT. That token is
/// Verifies the OTP code and returns `(email_jwt, email_wallet_address)`.
/// The JWT is used either to link the email to an existing guest account
/// (`link_email`) or directly as the auth token for a native email login.
pub async fn email_complete(email: &str, code: &str) -> Result<(String, H160), anyhow::Error> {
    let url = format!("{}/v1/auth/complete", THIRDWEB_API_BASE);
    let body = EmailCompleteRequest {
        method: "email",
        email,
        code,
    };

    tracing::debug!("thirdweb email_complete: url={}", url);

    let response = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()?
        .post(&url)
        .header("x-client-id", THIRDWEB_CLIENT_ID)
        .header("Origin", THIRDWEB_ALLOWED_ORIGIN)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!(
            "thirdweb email_complete failed: status={}, body={}",
            status,
            text
        ));
    }

    let parsed: EmailCompleteResponse = response.json().await?;
    let address = parsed
        .wallet_address
        .as_str()
        .as_h160()
        .ok_or_else(|| anyhow::anyhow!("thirdweb email_complete: invalid wallet address"))?;
    tracing::info!(
        "thirdweb email_complete: success, address={:#x}, email_jwt_len={}",
        address,
        parsed.token.len()
    );
    Ok((parsed.token, address))
}

/// Call C — links the email identity (`email_jwt`) into the existing guest
/// account identified by `guest_jwt`. The bearer token identifies the
/// surviving account, so the guest's wallet address is preserved. Address
/// preservation only holds when the email is new; if it already owns a
/// thirdweb wallet the API rejects the link with a message — surfaced here as
/// an error, not retried.
pub async fn link_email(guest_jwt: &str, email_jwt: &str) -> Result<(), anyhow::Error> {
    let url = format!("{}/v1/auth/link", THIRDWEB_API_BASE);
    let body = LinkAccountRequest {
        account_auth_token_to_connect: email_jwt,
    };

    tracing::debug!("thirdweb link_email: url={}", url);

    let response = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()?
        .post(&url)
        .header("x-client-id", THIRDWEB_CLIENT_ID)
        .header("Origin", THIRDWEB_ALLOWED_ORIGIN)
        .header("Authorization", format!("Bearer {}", guest_jwt))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!(
            "thirdweb link_email failed: status={}, body={}",
            status,
            text
        ));
    }

    tracing::info!("thirdweb link_email: email linked to guest wallet");
    Ok(())
}

#[derive(Debug, Deserialize)]
struct WalletsMeResponse {
    result: WalletsMeResult,
}

#[derive(Debug, Deserialize)]
struct WalletsMeResult {
    #[serde(default)]
    profiles: Vec<LinkedProfile>,
}

#[derive(Debug, Deserialize, Clone)]
struct LinkedProfile {
    #[serde(rename = "type")]
    profile_type: String,
    /// Opaque per-profile id (e.g. the guest profile's derived id). Present on
    /// the `GET /v1/wallets/me` payload and required to target an unlink.
    #[serde(default)]
    id: Option<String>,
    /// Email address for `email` profiles — the `/v1/auth/unlink` schema targets
    /// an email identity by `details.email` (not by `id`).
    #[serde(default)]
    email: Option<String>,
}

/// Fetches the account's linked auth profiles from the unified v1 API
/// `GET /v1/wallets/me`, which returns `{ result: { profiles: [{ type, id, .. }] } }`.
/// Auth is the plain guest JWT as a Bearer (no enclave prefix), same scheme as
/// `link_email`.
async fn get_linked_profiles(token: &str) -> Result<Vec<LinkedProfile>, anyhow::Error> {
    let url = format!("{}/v1/wallets/me", THIRDWEB_API_BASE);

    tracing::debug!("thirdweb get_linked_profiles: url={}", url);

    let response = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()?
        .get(&url)
        .header("x-client-id", THIRDWEB_CLIENT_ID)
        .header("Origin", THIRDWEB_ALLOWED_ORIGIN)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!(
            "thirdweb get_linked_profiles failed: status={}, body={}",
            status,
            text
        ));
    }

    let parsed: WalletsMeResponse = response.json().await?;
    Ok(parsed.result.profiles)
}

/// Lists the auth-method types linked to the account behind `token` — e.g.
/// `["guest"]` for a never-upgraded guest, `["guest", "email"]` after an OTP
/// upgrade. Lets the client detect whether a guest has anything linked beyond
/// the silent id-login.
pub async fn get_linked_profile_types(token: &str) -> Result<Vec<String>, anyhow::Error> {
    let types: Vec<String> = get_linked_profiles(token)
        .await?
        .into_iter()
        .map(|p| p.profile_type)
        .collect();
    tracing::info!("thirdweb get_linked_profile_types: {:?}", types);
    Ok(types)
}

/// Identifier the `/v1/auth/unlink` schema expects inside `details`. It differs
/// by auth method: a `guest` (and social/passkey) identity is targeted by its
/// opaque `id`, an `email` identity by its `email` address. Only the relevant
/// field is serialized.
#[derive(Debug, Serialize, Default)]
struct UnlinkDetails<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<&'a str>,
}

#[derive(Debug, Serialize)]
struct UnlinkRequest<'a> {
    #[serde(rename = "type")]
    profile_type: &'a str,
    details: UnlinkDetails<'a>,
    /// thirdweb guards against orphaning a wallet: unlinking the LAST auth method
    /// is refused with `400 "user must have at least one account …"` unless this
    /// is set. We always want the delete semantics, so it's `true`. It is a no-op
    /// when the profile being unlinked is not the last one.
    #[serde(rename = "allowAccountDeletion")]
    allow_account_deletion: bool,
}

/// Builds the `details` identifier `/v1/auth/unlink` needs for a given profile:
/// `email` is targeted by its address, everything else (`guest`, social, …) by
/// its opaque `id`. Returns `None` when the profile carries no usable
/// identifier (the caller skips it).
fn unlink_details_for(profile: &LinkedProfile) -> Option<UnlinkDetails<'_>> {
    let by_id = || {
        profile.id.as_deref().map(|id| UnlinkDetails {
            id: Some(id),
            email: None,
        })
    };
    match profile.profile_type.as_str() {
        "email" => profile
            .email
            .as_deref()
            .map(|email| UnlinkDetails {
                id: None,
                email: Some(email),
            })
            .or_else(by_id),
        _ => by_id(),
    }
}

/// POSTs a single `/v1/auth/unlink` and normalizes the response. A 2xx is
/// success. The **last-profile quirk** also counts as success: unlinking the
/// final identity makes the API delete the user and then answer `500` with
/// "User not found" / "Failed to unlink authentication account" (it can't
/// re-fetch the just-deleted user). Any other status is an error.
async fn post_unlink(
    token: &str,
    profile_type: &str,
    details: UnlinkDetails<'_>,
) -> Result<(), anyhow::Error> {
    let url = format!("{}/v1/auth/unlink", THIRDWEB_API_BASE);
    let body = UnlinkRequest {
        profile_type,
        details,
        allow_account_deletion: true,
    };

    let response = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()?
        .post(&url)
        .header("x-client-id", THIRDWEB_CLIENT_ID)
        .header("Origin", THIRDWEB_ALLOWED_ORIGIN)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    let status = response.status();
    let text = response.text().await.unwrap_or_default();

    if status.is_success() {
        tracing::info!(
            "thirdweb unlink: {} unlinked (status={})",
            profile_type,
            status
        );
        return Ok(());
    }

    let lower = text.to_lowercase();
    if status.as_u16() == 500 && (lower.contains("not found") || lower.contains("failed to unlink"))
    {
        tracing::info!(
            "thirdweb unlink: {} last-profile deletion (500 treated as success)",
            profile_type
        );
        return Ok(());
    }

    Err(anyhow::anyhow!(
        "thirdweb unlink {} failed: status={}, body={}",
        profile_type,
        status,
        text
    ))
}

/// Deletes a non-upgraded guest account by unlinking its sole `guest` profile
/// (issue #2335). For such a guest the silent id-login is the only identity, so
/// removing it deletes the thirdweb user — which frees the deterministic
/// `sessionId`, so the next `guest_login` from the same device anchor mints a
/// BRAND-NEW wallet (verified against the live API).
///
/// `token` must be a fresh guest JWT (see `refresh_guest_session`).
///
/// Two behaviours to be aware of, both handled here:
/// - **Safety refusal:** if the account has ANY non-`guest` profile it is an
///   *upgraded* guest — we refuse, so an email/social-recoverable account is
///   never silently stripped. Those go through the manual `/deletion` flow.
/// - **Last-profile quirk:** unlinking the final profile makes the API delete
///   the user and then answer `500` with "User not found" / "Failed to unlink
///   authentication account" (it can't re-fetch the just-deleted user). That
///   specific response means the delete succeeded, so we treat it as success.
pub async fn unlink_guest_profile(token: &str) -> Result<(), anyhow::Error> {
    let profiles = get_linked_profiles(token).await?;

    if profiles.iter().any(|p| p.profile_type != "guest") {
        return Err(anyhow::anyhow!(
            "refusing to unlink: account has non-guest profiles (upgraded guest)"
        ));
    }

    let Some(guest) = profiles.iter().find(|p| p.profile_type == "guest") else {
        // Nothing to unlink — already deleted / not a guest. Idempotent success.
        tracing::info!("thirdweb unlink_guest_profile: no guest profile — nothing to do");
        return Ok(());
    };
    let Some(id) = guest.id.as_deref() else {
        return Err(anyhow::anyhow!("guest profile missing id — cannot unlink"));
    };

    post_unlink(
        token,
        "guest",
        UnlinkDetails {
            id: Some(id),
            email: None,
        },
    )
    .await
}

/// Fully deletes an **upgraded** guest by unlinking EVERY linked profile — the
/// "double unlink" of the `guest` identity AND the linked `email` (plus any
/// other) identity. Removing all profiles deletes the thirdweb user outright, so
/// the account is gone (not merely stripped back to a bare guest) and the
/// deterministic `sessionId` is freed for a brand-new wallet on the next
/// `guest_login`.
///
/// Non-`guest` profiles are unlinked first and `guest` LAST, so the last-profile
/// 500-as-success quirk lands on the known guest request. It is best-effort per
/// profile: a failure on one is logged and the rest still run; the call only
/// returns an error if at least one unlink failed for real.
///
/// Unlike `unlink_guest_profile` this deliberately does NOT refuse an upgraded
/// account — deleting the recoverable email account is the whole point. It is
/// gated upstream (non-prod + the `enable-upgraded-deletion` deeplink).
///
/// `token` must be a fresh guest JWT (see `refresh_guest_session`); a guest
/// login on an already-upgraded account returns that same user's token.
pub async fn unlink_upgraded_account(token: &str) -> Result<(), anyhow::Error> {
    let profiles = get_linked_profiles(token).await?;
    if profiles.is_empty() {
        tracing::info!("thirdweb unlink_upgraded_account: no profiles — nothing to do");
        return Ok(());
    }

    // Unlink the guest identity LAST so the user-deletion 500 lands on it.
    let mut ordered: Vec<&LinkedProfile> = profiles.iter().collect();
    ordered.sort_by_key(|p| u8::from(p.profile_type == "guest"));

    let mut last_err: Option<anyhow::Error> = None;
    for profile in ordered {
        let Some(details) = unlink_details_for(profile) else {
            tracing::warn!(
                "thirdweb unlink_upgraded_account: profile {} has no identifier — skipping",
                profile.profile_type
            );
            continue;
        };
        if let Err(e) = post_unlink(token, &profile.profile_type, details).await {
            tracing::error!(
                "thirdweb unlink_upgraded_account: {} unlink failed: {:?}",
                profile.profile_type,
                e
            );
            last_err = Some(e);
        }
    }

    if let Some(e) = last_err {
        return Err(e);
    }
    tracing::info!("thirdweb unlink_upgraded_account: all profiles unlinked");
    Ok(())
}

/// `true` when the account has any auth method beyond the silent `guest` login —
/// i.e. an email/social/passkey identity is linked, so it has already been
/// "upgraded" and the Upgrade affordance should be hidden.
pub fn account_is_upgraded(profile_types: &[String]) -> bool {
    profile_types.iter().any(|t| t != "guest")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethers_core::utils::{hex, keccak256};

    fn make_session_id(seed: &str) -> String {
        format!(
            "dcl-godot-itest-{}",
            hex::encode(keccak256(seed.as_bytes()))
        )
    }

    #[test]
    fn account_is_upgraded_detects_non_guest_profiles() {
        // Never-upgraded guest: only the silent id-login.
        assert!(!account_is_upgraded(&["guest".to_string()]));
        // No profiles at all (e.g. query couldn't enumerate) → treat as not upgraded.
        assert!(!account_is_upgraded(&[]));
        // Any linked email/social means it has been upgraded.
        assert!(account_is_upgraded(&[
            "guest".to_string(),
            "email".to_string()
        ]));
        assert!(account_is_upgraded(&["google".to_string()]));
    }

    #[tokio::test]
    #[ignore = "hits live thirdweb API; run manually with --ignored"]
    async fn guest_login_returns_same_address_for_same_session_id() {
        let session_id = make_session_id("stable-itest-seed-1");
        let a = guest_login(&session_id).await.expect("first login");
        let b = guest_login(&session_id).await.expect("second login");
        assert_eq!(a.wallet_address, b.wallet_address);
    }

    #[tokio::test]
    #[ignore = "hits live thirdweb API; run manually with --ignored"]
    async fn guest_login_different_session_id_different_address() {
        let a = guest_login(&make_session_id("seed-a"))
            .await
            .expect("login a");
        let b = guest_login(&make_session_id("seed-b"))
            .await
            .expect("login b");
        assert_ne!(a.wallet_address, b.wallet_address);
    }

    #[tokio::test]
    #[ignore = "hits live thirdweb API; run manually with --ignored"]
    async fn sign_message_returns_verifiable_signature() {
        use ethers_core::types::Signature;
        use std::str::FromStr;

        let session = guest_login(&make_session_id("sign-itest-seed"))
            .await
            .expect("login");
        let message = "hello from godot-explorer itest";
        let signature_hex = sign_message(&session.token, session.wallet_address, 1, message)
            .await
            .expect("sign");

        let sig = Signature::from_str(signature_hex.strip_prefix("0x").unwrap_or(&signature_hex))
            .expect("parse signature");
        let recovered = sig
            .recover(message.as_bytes())
            .expect("recover signer from signature");
        assert_eq!(recovered, session.wallet_address);
    }

    /// Issue #2335: unlinking the sole `guest` profile deletes the thirdweb user
    /// so the SAME device anchor (sessionId) mints a BRAND-NEW wallet on the next
    /// login — i.e. the guest account is effectively reset. Uses a fresh,
    /// time-seeded sessionId so it always starts from `is_new_user=true` and never
    /// touches a real user's wallet.
    #[tokio::test]
    #[ignore = "hits live thirdweb API; run manually with --ignored"]
    async fn unlink_guest_profile_frees_the_session() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let session_id = make_session_id(&format!("delete-unlink-itest-{nanos}"));

        let s1 = guest_login(&session_id).await.expect("first login");
        assert!(s1.is_new_user, "fresh sessionId should mint a new wallet");

        unlink_guest_profile(&s1.token)
            .await
            .expect("unlink sole guest profile");

        let s2 = guest_login(&session_id).await.expect("second login");
        assert_ne!(
            s1.wallet_address, s2.wallet_address,
            "same sessionId must mint a NEW wallet after the guest profile is unlinked"
        );
        assert!(s2.is_new_user, "re-login after unlink should be a new user");
    }
}
