use crate::godot_classes::dcl_tokio_rpc::GodotTokioCall;

use super::{
    auth_identity::get_ephemeral_message,
    ephemeral_auth_chain::EphemeralAuthChain,
    wallet::{AsH160, SimpleAuthChain, Wallet},
};
use ethers_core::types::Signature;
use ethers_signers::LocalWallet;
use rand::thread_rng;
use std::str::FromStr;

pub async fn try_create_magic_link_ephemeral(
    sender: tokio::sync::mpsc::Sender<GodotTokioCall>,
) -> Result<(EphemeralAuthChain, u64), anyhow::Error> {
    let local_wallet = LocalWallet::new(&mut thread_rng());
    let signing_key_bytes = local_wallet.signer().to_bytes().to_vec();
    let ephemeral_wallet = Wallet::new_from_inner(Box::new(local_wallet));
    let ephemeral_address = format!("{:#x}", ephemeral_wallet.address());
    let expiration = std::time::SystemTime::now() + std::time::Duration::from_secs(30 * 24 * 3600);
    let ephemeral_message = get_ephemeral_message(ephemeral_address.as_str(), expiration);

    let (sx, rx) = tokio::sync::oneshot::channel::<(String, String)>();

    sender
        .send(GodotTokioCall::MagicSignMessage {
            message: ephemeral_message.clone(),
            response: sx.into(),
        })
        .await?;

    let (signer, signature) = rx.await?;

    let signer = signer
        .as_str()
        .as_h160()
        .ok_or(anyhow::Error::msg("invalid address"))?;

    let signature = Signature::from_str(signature.as_str())?;
    let chain_id = 1;

    let auth_chain =
        SimpleAuthChain::new_ephemeral_identity_auth_chain(signer, ephemeral_message, signature);

    let ephemeral_auth_chain =
        EphemeralAuthChain::new(signer, signing_key_bytes, auth_chain, expiration);

    Ok((ephemeral_auth_chain, chain_id))
}
