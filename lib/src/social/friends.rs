use crate::auth::ephemeral_auth_chain::EphemeralAuthChain;
use std::collections::HashMap;

/// Builds the auth chain message for Social Service RPC connection
/// Following the same format as C# BuildAuthChain method
pub async fn build_auth_chain(wallet: &EphemeralAuthChain) -> anyhow::Result<String> {
    let unix_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis();

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

#[cfg(test)]
mod tests {
    use super::*;
    use dcl_rpc::{client::RpcClient, transports::web_sockets::WebSocketTransport};
    use crate::dcl::components::proto_components::social_service::v2::*;

    const SOCIAL_URL: &str = "wss://rpc-social-service-ea.decentraland.org";

    /// Helper function to test the social service client with a given wallet
    /// This function can be called from tests or integration code
    pub async fn test_social_service_with_wallet(
        wallet: &EphemeralAuthChain,
    ) -> anyhow::Result<()> {
        let service_connection =
            dcl_rpc::transports::web_sockets::tungstenite::WebSocketClient::connect(SOCIAL_URL)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to connect: {:?}", e))?;
        let service_transport = WebSocketTransport::new(service_connection);

        // Build and send auth chain
        let auth_chain_message = build_auth_chain(wallet).await?;
        service_transport
            .send(auth_chain_message.as_bytes())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send auth chain: {:?}", e))?;

        let mut service_client = RpcClient::new(service_transport)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create client: {:?}", e))?;
        let port = service_client
            .create_port("SocialService")
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create port: {:?}", e))?;
        let service_module = port
            .load_module::<SocialServiceClient<_>>("SocialService")
            .await
            .map_err(|e| anyhow::anyhow!("Failed to load module: {:?}", e))?;

        // gather and send initial data
        let _friends_response = service_module
            .get_friends(GetFriendsPayload {
                pagination: None,
                status: None,
            })
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get friends: {:?}", e))?;

        Ok(())
    }

    #[tokio::test]
    #[ignore] // Requires network connection and a valid wallet
    async fn test_social_service_client() -> anyhow::Result<()> {
        // NOTE: This test requires a valid EphemeralAuthChain wallet
        // In practice, you would get this from the global identity/wallet manager
        // For now, this test is ignored and serves as documentation

        // Example usage (requires actual wallet):
        // let wallet = get_wallet_from_somewhere();
        // test_social_service_with_wallet(&wallet).await?;

        Ok(())
    }
}
