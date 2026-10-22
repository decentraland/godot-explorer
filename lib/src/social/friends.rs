use crate::auth::ephemeral_auth_chain::EphemeralAuthChain;
use std::collections::HashMap;

/// Builds the first frame of the social stream WebSocket: the auth chain headers, signed for
/// `get` and `/` (same format as the C# BuildAuthChain method).
pub async fn build_auth_chain(wallet: &EphemeralAuthChain) -> anyhow::Result<String> {
    let unix_time = crate::utils::clock::unix_time_ms();

    let metadata = "{}";
    let payload = format!("get:/:{}:{}", unix_time, metadata);

    // Sign the payload with the ephemeral wallet
    let signature = wallet
        .ephemeral_wallet()
        .sign_message(&payload)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to sign message: {:?}", e))?;

    // Build the auth chain
    let mut auth_chain = wallet.auth_chain().clone();
    auth_chain.add_signed_entity(payload, signature);

    // Build the auth chain buffer (similar to C# authChainBuffer)
    let mut auth_chain_buffer = HashMap::new();

    // Add auth chain links
    for (key, value) in auth_chain.headers() {
        auth_chain_buffer.insert(key, value);
    }

    // Add timestamp and metadata
    auth_chain_buffer.insert("x-identity-timestamp".to_string(), format!("{}", unix_time));
    auth_chain_buffer.insert("x-identity-metadata".to_string(), metadata.to_string());

    // Serialize to JSON
    Ok(serde_json::to_string(&auth_chain_buffer)?)
}
