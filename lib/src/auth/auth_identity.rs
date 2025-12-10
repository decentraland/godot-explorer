use std::str::FromStr;

use crate::godot_classes::dcl_tokio_rpc::GodotTokioCall;

use super::{
    decentraland_auth_server::{
        do_request, do_request_mobile, fetch_identity_by_id, CreateRequest,
    },
    ephemeral_auth_chain::EphemeralAuthChain,
    wallet::{AsH160, ChainLink, ObjSafeWalletSigner, SimpleAuthChain, Wallet},
};
use chrono::{DateTime, Utc};
use ethers_core::{types::Signature, utils::hex};
use ethers_signers::LocalWallet;
use rand::thread_rng;

pub fn get_ephemeral_message(ephemeral_address: &str, expiration: std::time::SystemTime) -> String {
    let datetime: DateTime<Utc> = expiration.into();
    let formatted_time = datetime.format("%Y-%m-%dT%H:%M:%S%.3fZ");
    format!(
        "Decentraland Login\nEphemeral address: {ephemeral_address}\nExpiration: {formatted_time}",
    )
}

pub async fn try_create_remote_ephemeral(
    url_reporter_sender: tokio::sync::mpsc::Sender<GodotTokioCall>,
    target_config_id: Option<String>,
) -> Result<(EphemeralAuthChain, u64), anyhow::Error> {
    let local_wallet = LocalWallet::new(&mut thread_rng());
    let signing_key_bytes = local_wallet.signer().to_bytes().to_vec();
    let ephemeral_wallet = Wallet::new_from_inner(Box::new(local_wallet));
    let ephemeral_address = format!("{:#x}", ephemeral_wallet.address());
    let expiration = std::time::SystemTime::now() + std::time::Duration::from_secs(30 * 24 * 3600);
    let ephemeral_message = get_ephemeral_message(ephemeral_address.as_str(), expiration);

    let request = CreateRequest::from_new_ephemeral(ephemeral_message.as_str());
    let (owner_address, result) =
        do_request(request, url_reporter_sender, target_config_id).await?;

    let result = result
        .as_str()
        .ok_or(anyhow::Error::msg("response is not a string"))?;
    let signer = owner_address
        .as_str()
        .as_h160()
        .ok_or(anyhow::Error::msg("invalid address"))?;

    let signature = Signature::from_str(result)?;
    let chain_id = 1;

    let auth_chain =
        SimpleAuthChain::new_ephemeral_identity_auth_chain(signer, ephemeral_message, signature);

    let ephemeral_auth_chain =
        EphemeralAuthChain::new(signer, signing_key_bytes, auth_chain, expiration);

    Ok((ephemeral_auth_chain, chain_id))
}

/// Pending mobile auth state - just tracks whether mobile auth is in progress.
/// The actual identity data comes from the server via deep link.
pub struct PendingMobileAuth {
    pub request_id: String,
}

/// Starts mobile auth flow by opening the browser with auth URL.
/// Returns the pending auth state that should be saved to complete auth when deep link arrives.
/// Note: For mobile, the server generates the ephemeral identity, so we don't create it locally.
pub async fn start_mobile_auth(
    url_reporter_sender: tokio::sync::mpsc::Sender<GodotTokioCall>,
    target_config_id: Option<String>,
) -> Result<PendingMobileAuth, anyhow::Error> {
    // For mobile auth, we use an empty request since the server will generate everything
    let request = CreateRequest::from_new_ephemeral("");
    let mobile_request = do_request_mobile(request, url_reporter_sender, target_config_id).await?;

    Ok(PendingMobileAuth {
        request_id: mobile_request.request_id,
    })
}

/// Completes mobile auth flow by fetching the identity result using the ID from deep link.
/// The server provides the full ephemeral identity (including private key) and auth chain.
pub async fn complete_mobile_auth(
    identity_id: String,
) -> Result<(EphemeralAuthChain, u64), anyhow::Error> {
    let response = fetch_identity_by_id(identity_id).await?;
    let identity = response.identity;

    // Parse the signer address from the first element in the auth chain (SIGNER type)
    let signer = identity
        .auth_chain
        .first()
        .and_then(|link| link.payload.as_str().as_h160())
        .ok_or(anyhow::Error::msg("Invalid signer in auth chain"))?;

    // Parse the ephemeral private key (remove 0x prefix if present)
    let private_key_hex = identity
        .ephemeral_identity
        .private_key
        .strip_prefix("0x")
        .unwrap_or(&identity.ephemeral_identity.private_key);
    let ephemeral_keys = hex::decode(private_key_hex)
        .map_err(|e| anyhow::Error::msg(format!("Invalid ephemeral private key: {}", e)))?;

    // Parse expiration date
    let expiration = chrono::DateTime::parse_from_rfc3339(&identity.expiration)
        .map_err(|e| anyhow::Error::msg(format!("Invalid expiration date: {}", e)))?;
    let expiration_system_time = std::time::SystemTime::UNIX_EPOCH
        + std::time::Duration::from_secs(expiration.timestamp() as u64);

    // Convert auth chain from server format to our SimpleAuthChain
    let chain_links: Vec<ChainLink> = identity
        .auth_chain
        .into_iter()
        .map(|link| ChainLink::new(link.ty, link.payload, link.signature))
        .collect();
    let auth_chain = SimpleAuthChain::from_chain_links(chain_links);

    let chain_id = 1;

    let ephemeral_auth_chain =
        EphemeralAuthChain::new(signer, ephemeral_keys, auth_chain, expiration_system_time);

    Ok((ephemeral_auth_chain, chain_id))
}

pub fn create_local_ephemeral(signer_wallet: &LocalWallet) -> EphemeralAuthChain {
    let local_wallet = LocalWallet::new(&mut thread_rng());
    let signing_key_bytes = local_wallet.signer().to_bytes().to_vec();
    let ephemeral_wallet = Wallet::new_from_inner(Box::new(local_wallet));
    let ephemeral_address = format!("{:#x}", ephemeral_wallet.address());
    let expiration = std::time::SystemTime::now() + std::time::Duration::from_secs(30 * 24 * 3600);
    let ephemeral_message = get_ephemeral_message(ephemeral_address.as_str(), expiration);

    let signature =
        futures_lite::future::block_on(signer_wallet.sign_message(ephemeral_message.as_bytes()))
            .expect("signing with local wallet failed");

    let auth_chain = SimpleAuthChain::new_ephemeral_identity_auth_chain(
        signer_wallet.address(),
        ephemeral_message,
        signature,
    );

    EphemeralAuthChain::new(
        signer_wallet.address(),
        signing_key_bytes,
        auth_chain,
        expiration,
    )
}
