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
struct EmailCompleteResponse {
    /// The EMAIL identity JWT — this is the only field we consume; it becomes
    /// `accountAuthTokenToConnect` in the `/link` call. The `walletAddress`
    /// returned alongside is the email identity's OWN address (not the final
    /// one), so it is deliberately ignored.
    token: String,
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
pub async fn guest_login(session_id: &str) -> Result<ThirdwebGuestSession, anyhow::Error> {
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
            "thirdweb guest_login failed: status={}, body={}",
            status,
            text
        ));
    }

    let parsed: GuestLoginResponse = response.json().await?;
    let address = parsed
        .wallet_address
        .as_str()
        .as_h160()
        .ok_or_else(|| anyhow::anyhow!("thirdweb returned invalid wallet address"))?;

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
    guest_login(&session_id).await
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
/// fed to `link_email` as `accountAuthTokenToConnect`. The `walletAddress`
/// returned by this endpoint is the email identity's own address and is NOT
/// the final wallet — it is intentionally not parsed.
pub async fn email_complete(email: &str, code: &str) -> Result<String, anyhow::Error> {
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
    tracing::info!(
        "thirdweb email_complete: success, email_jwt_len={}",
        parsed.token.len()
    );
    Ok(parsed.token)
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

#[derive(Debug, Deserialize)]
struct LinkedProfile {
    #[serde(rename = "type")]
    profile_type: String,
}

/// Lists the auth-method types linked to the account behind `token` — e.g.
/// `["guest"]` for a never-upgraded guest, `["guest", "email"]` after an OTP
/// upgrade. Hits the unified v1 API `GET /v1/wallets/me`, which returns
/// `{ result: { profiles: [{ type, ... }] } }`. Auth is the plain guest JWT as a
/// Bearer (no enclave prefix), same scheme as `link_email`. Lets the client
/// detect whether a guest has anything linked beyond the silent id-login.
pub async fn get_linked_profile_types(token: &str) -> Result<Vec<String>, anyhow::Error> {
    let url = format!("{}/v1/wallets/me", THIRDWEB_API_BASE);

    tracing::debug!("thirdweb get_linked_profile_types: url={}", url);

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
            "thirdweb get_linked_profile_types failed: status={}, body={}",
            status,
            text
        ));
    }

    let parsed: WalletsMeResponse = response.json().await?;
    let types: Vec<String> = parsed
        .result
        .profiles
        .into_iter()
        .map(|p| p.profile_type)
        .collect();
    tracing::info!("thirdweb get_linked_profile_types: {:?}", types);
    Ok(types)
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
}
