use dcl_rpc::{client::RpcClient, transports::web_sockets::WebSocketTransport};

use crate::dcl::components::proto_components::social_service::v2::*;

#[cfg(test)]
mod tests {
    use super::*;

    const SOCIAL_URL: &str = "wss://rpc-social-service-ea.decentraland.org";

    #[tokio::test]
    #[ignore] // Requires network connection, run with --ignored
    async fn test_social_service_client() -> anyhow::Result<()> {
        let service_connection =
            dcl_rpc::transports::web_sockets::tungstenite::WebSocketClient::connect(SOCIAL_URL)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to connect: {:?}", e))?;
        let service_transport = WebSocketTransport::new(service_connection);
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
}
