use std::str::FromStr;

use crate::godot_classes::dcl_tokio_rpc::GodotTokioCall;

use super::{
    decentraland_auth_server::{do_request, CreateRequest},
    ephemeral_auth_chain::EphemeralAuthChain,
    wallet::{AsH160, SimpleAuthChain, Wallet, WalletType},
};
use chrono::{DateTime, Utc};
use ethers_core::types::Signature;
use ethers_signers::{LocalWallet, Signer};
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
    let ephemeral_wallet = Wallet::new_from_inner(WalletType::Local(local_wallet));
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

pub fn create_local_ephemeral(signer_wallet: &LocalWallet) -> EphemeralAuthChain {
    let local_wallet = LocalWallet::new(&mut thread_rng());
    let signing_key_bytes = local_wallet.signer().to_bytes().to_vec();
    let ephemeral_wallet = Wallet::new_from_inner(WalletType::Local(local_wallet));
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
