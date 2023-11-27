// fn get_ephemeral_message(ephemeral_address: &str, expiration: std::time::SystemTime) -> String {
//     let expiration_str = match expiration.duration_since(std::time::SystemTime::UNIX_EPOCH) {
//         Ok(duration) => duration.as_secs().to_string(),
//         Err(_) => "Invalid expiration time".to_string(),
//     };

//     format!(
//         "Decentraland Login\nEphemeral address: {}\nExpiration: {}",
//         ephemeral_address, expiration_str
//     )
// }

// pub async fn try_create_ephemeral(
//     sender: tokio::sync::mpsc::Sender<RemoteReportState>,
// ) -> Result<(H160, Wallet, String), ()> {
//     let ephemeral_wallet = Wallet::new_local_wallet();
//     let ephemeral_address = format!("{:#x}", ephemeral_wallet.address());
//     let expiration = std::time::SystemTime::now() + std::time::Duration::from_secs(30 * 24 * 3600);
//     let message = get_ephemeral_message(ephemeral_address.as_str(), expiration);

//     let (signer, signature) = remote_sign_message(message.as_bytes(), None, Some(sender)).await?;
//     Ok((signer, ephemeral_wallet, signature))
// }

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
