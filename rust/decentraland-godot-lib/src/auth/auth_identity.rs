use super::{
    ephemeral_auth_chain::EphemeralAuthChain,
    wallet::{SimpleAuthChain, Wallet},
    with_browser_and_server::{remote_sign_message, RemoteReportState},
};
use chrono::{DateTime, Utc};
use ethers::{
    signers::LocalWallet,
    types::{Signature, H160},
};
use rand::thread_rng;

fn get_ephemeral_message(ephemeral_address: &str, expiration: std::time::SystemTime) -> String {
    let datetime: DateTime<Utc> = expiration.into();
    let formatted_time = datetime.format("%Y-%m-%dT%H:%M:%S%.3fZ");
    format!(
        "Decentraland Login\nEphemeral address: {ephemeral_address}\nExpiration: {formatted_time}",
    )
}

pub async fn try_create_ephemeral_with_account(
    signer: H160,
    sender: tokio::sync::mpsc::Sender<RemoteReportState>,
) -> Result<(H160, Wallet, Signature, u64), ()> {
    let ephemeral_wallet = Wallet::new_local_wallet();
    let ephemeral_address = format!("{:#x}", ephemeral_wallet.address());
    let expiration = std::time::SystemTime::now() + std::time::Duration::from_secs(30 * 24 * 3600);
    let message = get_ephemeral_message(ephemeral_address.as_str(), expiration);

    let (signer, signature, chain_id) =
        remote_sign_message(message.as_bytes(), Some(signer), sender).await?;
    Ok((signer, ephemeral_wallet, signature, chain_id))
}

pub async fn try_create_ephemeral(
    sender: tokio::sync::mpsc::Sender<RemoteReportState>,
) -> Result<(EphemeralAuthChain, u64), ()> {
    let local_wallet = LocalWallet::new(&mut thread_rng());
    let signing_key_bytes = local_wallet.signer().to_bytes().to_vec();
    let ephemeral_wallet = Wallet::new_from_inner(Box::new(local_wallet));
    let ephemeral_address = format!("{:#x}", ephemeral_wallet.address());
    let expiration = std::time::SystemTime::now() + std::time::Duration::from_secs(30 * 24 * 3600);
    let ephemeral_message = get_ephemeral_message(ephemeral_address.as_str(), expiration);

    let (signer, signature, chain_id) =
        remote_sign_message(ephemeral_message.as_bytes(), None, sender).await?;

    let auth_chain =
        SimpleAuthChain::new_ephemeral_identity_auth_chain(signer, ephemeral_message, signature);

    let ephemeral_auth_chain =
        EphemeralAuthChain::new(signer, signing_key_bytes, auth_chain, expiration);

    Ok((ephemeral_auth_chain, chain_id))
}

#[cfg(test)]
mod test {
    // #[traced_test]
    // #[tokio::test]
    // async fn test_try_create_ephemeral() {
    //     let (sx, rx) = tokio::sync::mpsc::channel(100);
    //     let Ok((signer, wallet, signature)) = try_create_ephemeral(sx).await else {
    //         return;
    //     };
    //     tracing::info!(
    //         "signer {:?} signature {:?} wallet {:?}",
    //         signer,
    //         signature,
    //         wallet.address()
    //     );
    // }
}
