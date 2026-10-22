use std::sync::atomic::{AtomicU64, Ordering};

use futures_util::{SinkExt, StreamExt};
use tokio::{net::TcpStream, sync::Mutex};
use tokio_tungstenite::{tungstenite::Message, MaybeTlsStream, WebSocketStream};

use crate::urls;

type RpcSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// JSON-RPC client for the Ethereum node behind `urls::ethereum_rpc`, used by scenes'
/// `sendAsync`. Requests are serialized on one lazily opened WebSocket.
pub struct EthereumProvider {
    socket: Mutex<Option<RpcSocket>>,
    next_id: AtomicU64,
}

impl Default for EthereumProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl EthereumProvider {
    pub fn new() -> Self {
        Self {
            socket: Mutex::new(None),
            next_id: AtomicU64::new(1),
        }
    }

    pub async fn send_async(
        &self,
        method: &str,
        params: &[serde_json::Value],
    ) -> Result<serde_json::Value, anyhow::Error> {
        let mut socket = self.socket.lock().await;

        if socket.is_none() {
            let provider_url = urls::ethereum_rpc_with_project("kernel-local");
            let (stream, _) = tokio_tungstenite::connect_async(provider_url).await?;
            socket.replace(stream);
        }

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let result = request(socket.as_mut().unwrap(), id, method, params).await;
        if result.is_err() {
            socket.take();
        }
        result
    }
}

async fn request(
    socket: &mut RpcSocket,
    id: u64,
    method: &str,
    params: &[serde_json::Value],
) -> Result<serde_json::Value, anyhow::Error> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params,
    });
    socket.send(Message::Text(body.to_string())).await?;

    while let Some(message) = socket.next().await {
        let Message::Text(text) = message? else {
            continue;
        };
        let mut response: serde_json::Value = serde_json::from_str(&text)?;
        if response["id"] != id {
            continue;
        }
        if let Some(error) = response.get("error") {
            anyhow::bail!(
                "(code: {}, message: {}, data: {})",
                error["code"],
                error["message"].as_str().unwrap_or_default(),
                error["data"]
            );
        }
        return Ok(response["result"].take());
    }

    anyhow::bail!("Ethereum RPC connection closed")
}
