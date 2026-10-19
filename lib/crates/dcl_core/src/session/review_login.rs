//! App Store review sign-in — the escape hatch for Apple's reviewers.
//!
//! The regular "Sign in with email" flow mints its wallet through thirdweb,
//! which emails a fresh OTP that expires in 15 minutes. A reviewer can never
//! complete it: the code we publish in App Review Information is dead by the
//! time they read it, and the mailbox that receives the live one isn't theirs.
//!
//! So one address — and only one — is served by mobile-bff instead. It answers
//! `/test-auth/send-code` and `/test-auth/verify-code`, checks the submitted
//! code against a fixed value it holds in config, and mints a full
//! `AuthIdentity` (server-signed auth chain + ephemeral key) for the review
//! wallet. The shape is the same one the auth-server deep link returns, so the
//! client reuses `ephemeral_from_auth_identity` verbatim.
//!
//! Both endpoints reject every other address with a 403, and 404 when the
//! deployment has no review account configured — nothing here weakens sign-in
//! for real users.

use std::time::Duration;

use ethers_core::types::H160;
use serde::{Deserialize, Serialize};

use super::auth_identity::ephemeral_from_auth_identity;
use super::decentraland_auth_server::AuthIdentity;
use super::ephemeral_auth_chain::EphemeralAuthChain;
use super::wallet::AsH160;
use crate::urls;

/// The single address mobile-bff will answer for. Kept client-side so a normal
/// sign-in never round-trips to the review endpoint: everyone else goes
/// straight to thirdweb as before.
const REVIEW_LOGIN_EMAIL: &str = "appletesting@dclregenesislabs.xyz";

const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug, Serialize)]
struct SendCodeRequest<'a> {
    email: &'a str,
}

#[derive(Debug, Serialize)]
struct VerifyCodeRequest<'a> {
    email: &'a str,
    code: &'a str,
}

#[derive(Debug, Deserialize)]
struct ErrorResponse {
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VerifyCodeResponse {
    data: Option<VerifyCodeData>,
}

#[derive(Debug, Deserialize)]
struct VerifyCodeData {
    identity: AuthIdentity,
    address: String,
}

/// True when `email` is the App Review account, ignoring case and surrounding
/// whitespace — a reviewer typing it by hand on a phone keyboard gets the same
/// treatment as a paste.
pub fn is_review_email(email: &str) -> bool {
    email.trim().eq_ignore_ascii_case(REVIEW_LOGIN_EMAIL)
}

/// Reads the server's `{ ok: false, error }` body so the UI can show why the
/// call was refused instead of a bare status code.
fn describe_failure(status: reqwest::StatusCode, body: &str) -> String {
    let detail = serde_json::from_str::<ErrorResponse>(body)
        .ok()
        .and_then(|parsed| parsed.error)
        .unwrap_or_else(|| body.trim().to_string());

    if detail.is_empty() {
        format!("status={}", status)
    } else {
        format!("status={}, {}", status, detail)
    }
}

/// Stands in for thirdweb's `email_initiate`. Nothing is actually sent — the
/// code is fixed — but the call still runs so a misconfigured deployment
/// (no review account) fails here rather than at the code screen.
pub async fn send_code(email: &str) -> Result<(), anyhow::Error> {
    let url = format!("{}/test-auth/send-code", urls::mobile_bff());

    let response = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()?
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&SendCodeRequest { email })
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!(
            "review send_code failed: {}",
            describe_failure(status, &text)
        ));
    }

    tracing::info!("review send_code: accepted");
    Ok(())
}

/// Stands in for `perform_email_login`: exchanges the fixed code for the review
/// wallet's identity and returns it in the same `(address, chain)` shape the
/// thirdweb path resolves with, so the caller is unchanged.
pub async fn verify_code(
    email: &str,
    code: &str,
) -> Result<(H160, EphemeralAuthChain), anyhow::Error> {
    let url = format!("{}/test-auth/verify-code", urls::mobile_bff());

    let response = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()?
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&VerifyCodeRequest { email, code })
        .send()
        .await?;

    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(anyhow::anyhow!(
            "review verify_code failed: {}",
            describe_failure(status, &text)
        ));
    }

    let (address, ephemeral_auth_chain) = identity_from_response(&text)?;
    tracing::info!("review verify_code: signed in as {:#x}", address);
    Ok((address, ephemeral_auth_chain))
}

/// Parses a `/test-auth/verify-code` success body into the wallet the client
/// signs in as. Split out from the request so the response contract — the
/// camelCase identity fields, the millisecond expiration, and the address
/// matching the chain it is rooted at — is checked without a network call.
fn identity_from_response(body: &str) -> Result<(H160, EphemeralAuthChain), anyhow::Error> {
    let parsed: VerifyCodeResponse = serde_json::from_str(body)
        .map_err(|e| anyhow::anyhow!("review verify_code: unreadable response: {}", e))?;
    let data = parsed
        .data
        .ok_or_else(|| anyhow::anyhow!("review verify_code: response carried no identity"))?;

    let address = data
        .address
        .as_str()
        .as_h160()
        .ok_or_else(|| anyhow::anyhow!("review verify_code: invalid wallet address"))?;

    let (ephemeral_auth_chain, _chain_id) = ephemeral_from_auth_identity(data.identity)?;

    // The address the caller signs in as has to be the one the auth chain is
    // rooted at, or every signed request would be attributed to a wallet that
    // never signed the delegation.
    if ephemeral_auth_chain.signer() != address {
        return Err(anyhow::anyhow!(
            "review verify_code: address {:#x} does not match auth chain signer {:#x}",
            address,
            ephemeral_auth_chain.signer()
        ));
    }

    Ok((address, ephemeral_auth_chain))
}

#[cfg(test)]
mod tests {
    use super::{identity_from_response, is_review_email};
    use ethers_core::utils::hex;
    use ethers_signers::{LocalWallet, Signer};
    use rand::thread_rng;

    #[test]
    fn only_the_review_address_is_diverted() {
        assert!(is_review_email("appletesting@dclregenesislabs.xyz"));
        assert!(is_review_email("  AppleTesting@DclRegenesisLabs.xyz  "));
        assert!(!is_review_email("appletesting@decentraland.org"));
        assert!(!is_review_email("someone@dclregenesislabs.xyz"));
        assert!(!is_review_email(""));
    }

    /// Builds the body mobile-bff answers `/test-auth/verify-code` with: a root
    /// wallet signs the ephemeral delegation, and the response hands the client
    /// the ephemeral private key plus the signed chain.
    fn minted_response(root: &LocalWallet, claimed_address: &str) -> String {
        let ephemeral = LocalWallet::new(&mut thread_rng());
        // Milliseconds on purpose: the server emits them and the signed payload
        // embeds the exact string, so the client must not re-render it.
        let expiration = "2099-01-01T00:00:00.123Z";
        let message = format!(
            "Decentraland Login\nEphemeral address: {:#x}\nExpiration: {}",
            ephemeral.address(),
            expiration
        );
        let signature =
            futures_lite::future::block_on(root.sign_message(message.as_bytes())).unwrap();

        serde_json::json!({
            "ok": true,
            "data": {
                "address": claimed_address,
                "identity": {
                    "ephemeralIdentity": {
                        "privateKey": hex::encode(ephemeral.signer().to_bytes()),
                        "publicKey": "",
                        "address": format!("{:#x}", ephemeral.address()),
                    },
                    "expiration": expiration,
                    "authChain": [
                        { "type": "SIGNER", "payload": format!("{:#x}", root.address()), "signature": "" },
                        {
                            "type": "ECDSA_EPHEMERAL",
                            "payload": message,
                            "signature": format!("0x{}", signature),
                        },
                    ],
                },
            },
        })
        .to_string()
    }

    #[test]
    fn accepts_a_server_minted_identity_and_rejects_a_mismatched_one() {
        let root = LocalWallet::new(&mut thread_rng());

        let body = minted_response(&root, &format!("{:#x}", root.address()));
        let (address, chain) = identity_from_response(&body).expect("identity should parse");
        assert_eq!(address, root.address());
        assert_eq!(chain.signer(), root.address());
        assert!(!chain.expired());

        // A body claiming an address the chain was not rooted at must not sign in:
        // every request would be attributed to a wallet that signed nothing.
        let impostor = LocalWallet::new(&mut thread_rng());
        let tampered = minted_response(&root, &format!("{:#x}", impostor.address()));
        assert!(identity_from_response(&tampered).is_err());
    }
}
