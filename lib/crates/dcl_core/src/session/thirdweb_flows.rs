//! Thirdweb account flows: silent guest login, email link/login, account deletion.

use ethers_core::types::H160;

use super::{
    auth_identity::{create_ephemeral_from_external_signature, generate_ephemeral_for_signing},
    device_anchor,
    ephemeral_auth_chain::EphemeralAuthChain,
    thirdweb_guest,
};

/// Runs the silent guest-login flow end-to-end:
///   1. resolve the device anchor (native value or desktop fallback)
///   2. hash anchor → opaque thirdweb `sessionId`
///   3. POST /v1/auth/complete (guest) → wallet address + bearer token
///   4. mint a local ephemeral keypair + Decentraland delegation message
///   5. POST /v1/wallets/sign-message → external signature
///   6. assemble the EphemeralAuthChain
///
/// On success, the returned `EphemeralAuthChain` is signed by the thirdweb
/// wallet and delegates request signing to the local ephemeral key for the
/// usual ~30 day window. The thirdweb JWT itself is dropped after step 5 —
/// every cold start re-runs steps 1–6 (idempotent because the anchor is
/// stable, so thirdweb returns the same wallet address).
/// Successful outcome of the guest-login flow. Carries `is_new_user` so
/// analytics can tell a freshly minted wallet apart from a returning anchor
/// re-logging into its existing wallet ("wallets created" = new mints only).
pub struct GuestLoginOutcome {
    pub address: H160,
    pub chain: EphemeralAuthChain,
    pub is_new_user: bool,
}

pub async fn perform_thirdweb_guest_login(
    device_anchor_id: String,
) -> Result<GuestLoginOutcome, thirdweb_guest::GuestLoginError> {
    let anchor = device_anchor::resolve_anchor(&device_anchor_id);
    let session_id = device_anchor::compute_session_id(&anchor);

    let session = thirdweb_guest::guest_login(&session_id).await?;

    let (ephemeral_message, ephemeral_keys, expiration) = generate_ephemeral_for_signing();

    // The wallet already exists server-side at this point; a signing failure
    // still fails the login, but under its own reason so the dashboard doesn't
    // misattribute it to wallet creation.
    let signature_hex = thirdweb_guest::sign_message(
        &session.token,
        session.wallet_address,
        1,
        &ephemeral_message,
    )
    .await
    .map_err(|e| thirdweb_guest::GuestLoginError {
        reason: "sign_message",
        http_status: None,
        message: e.to_string(),
    })?;

    let signer_address_str = format!("{:#x}", session.wallet_address);
    let chain = create_ephemeral_from_external_signature(
        &signer_address_str,
        &signature_hex,
        &ephemeral_keys,
        expiration,
        &ephemeral_message,
    )
    .map_err(|e| thirdweb_guest::GuestLoginError {
        reason: "ephemeral",
        http_status: None,
        message: e.to_string(),
    })?;

    // Persist the JWT so future cold starts can renew the ephemeral
    // delegation without re-running `guest_login`. Failure is non-fatal:
    // we already have a valid in-memory ephemeral chain for this session.
    if let Err(e) = thirdweb_guest::save_session_to_disk(&session) {
        tracing::warn!(
            "thirdweb: failed to persist session to disk (non-fatal): {}",
            e
        );
    }

    Ok(GuestLoginOutcome {
        address: session.wallet_address,
        chain,
        is_new_user: session.is_new_user,
    })
}

/// Runs the "Upgrade to OTP" link end-to-end:
///   1. verify the OTP → EMAIL identity JWT (`email_complete`)
///   2. obtain a FRESH guest JWT — re-run `guest_login` from the anchor so the
///      `/link` bearer is guaranteed non-expired (the persisted one may have
///      aged out); fall back to the persisted token if the refresh fails
///   3. `link_email` merges the email into the guest user, preserving address
///   4. persist the refreshed session so the disk copy stays current
///
/// Returns the guest wallet address, which is unchanged by linking.
pub async fn perform_link_email(
    device_anchor_id: String,
    email: String,
    code: String,
) -> Result<H160, anyhow::Error> {
    let (email_jwt, _email_address) = thirdweb_guest::email_complete(&email, &code).await?;

    // Prefer a freshly minted guest session (idempotent: same anchor → same
    // wallet → fresh token). Fall back to the persisted token only if the
    // refresh round-trip fails (e.g. transient network), since the disk token
    // may be expired.
    let session = match thirdweb_guest::refresh_guest_session(&device_anchor_id).await {
        Ok(session) => session,
        Err(refresh_err) => {
            tracing::warn!(
                "thirdweb: guest session refresh failed ({}); falling back to persisted token",
                refresh_err
            );
            thirdweb_guest::load_session_from_disk().ok_or_else(|| {
                anyhow::anyhow!(
                    "no guest session available to authorize link (refresh failed: {})",
                    refresh_err
                )
            })?
        }
    };

    thirdweb_guest::link_email(&session.token, &email_jwt).await?;

    // Keep the disk copy current with the (possibly refreshed) token. The
    // address is unchanged by linking, so the rehydration match still holds.
    if let Err(e) = thirdweb_guest::save_session_to_disk(&session) {
        tracing::warn!(
            "thirdweb: failed to persist refreshed session after link (non-fatal): {}",
            e
        );
    }

    Ok(session.wallet_address)
}

/// Native email login — verifies the OTP and mints a DCL ephemeral auth chain
/// signed by the email identity's own wallet:
///   1. `email_complete` → email JWT + email wallet address
///   2. mint a local ephemeral keypair + Decentraland delegation message
///   3. `sign_message` with the email JWT to sign the delegation
///   4. assemble the EphemeralAuthChain
pub async fn perform_email_login(
    email: String,
    code: String,
) -> Result<(H160, EphemeralAuthChain), anyhow::Error> {
    let (email_jwt, email_address) = thirdweb_guest::email_complete(&email, &code).await?;

    let (ephemeral_message, ephemeral_keys, expiration) = generate_ephemeral_for_signing();

    let signature_hex =
        thirdweb_guest::sign_message(&email_jwt, email_address, 1, &ephemeral_message).await?;

    let signer_address_str = format!("{:#x}", email_address);
    let chain = create_ephemeral_from_external_signature(
        &signer_address_str,
        &signature_hex,
        &ephemeral_keys,
        expiration,
        &ephemeral_message,
    )?;

    Ok((email_address, chain))
}

/// Deletes the guest account server-side (issue #2335):
///   1. re-derive a FRESH guest JWT from the anchor (the persisted one may be
///      expired) — idempotent: same anchor → same wallet → fresh token
///   2. `unlink_guest_profile` removes the sole `guest` identity, deleting the
///      thirdweb user so the sessionId is freed for a brand-new wallet
pub async fn perform_delete_guest_account(device_anchor_id: String) -> Result<(), anyhow::Error> {
    let session = thirdweb_guest::refresh_guest_session(&device_anchor_id).await?;
    thirdweb_guest::unlink_guest_profile(&session.token).await
}

/// Fully deletes an UPGRADED guest account server-side (enable-upgraded-deletion):
///   1. re-derive a FRESH guest JWT from the anchor (a guest login on an
///      upgraded account returns that same user's token)
///   2. `unlink_upgraded_account` unlinks BOTH the `guest` and the `email` (plus
///      any other) profile, deleting the whole thirdweb user so the account is
///      gone and the sessionId is freed for a brand-new wallet.
pub async fn perform_delete_upgraded_account(
    device_anchor_id: String,
) -> Result<(), anyhow::Error> {
    let session = thirdweb_guest::refresh_guest_session(&device_anchor_id).await?;
    thirdweb_guest::unlink_upgraded_account(&session.token).await
}

/// Whether the guest behind this device anchor has linked a method beyond the silent login
/// (email, social). A fresh thirdweb query: the cached flag is cold outside the upgrade card.
pub async fn perform_guest_upgraded(device_anchor_id: String) -> Result<bool, anyhow::Error> {
    let session = thirdweb_guest::refresh_guest_session(&device_anchor_id).await?;
    let types = thirdweb_guest::get_linked_profile_types(&session.token).await?;
    Ok(types.iter().any(|kind| kind != "guest"))
}
