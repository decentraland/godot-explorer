use std::time::Duration;

use crate::urls;

const RPC_TIMEOUT: Duration = Duration::from_secs(30);

/// JSON-RPC client for the Ethereum node behind `urls::ethereum_rpc`, used by scenes'
/// `sendAsync`. Each request is its own HTTPS POST, so scenes don't wait on each other.
pub struct EthereumProvider {
    client: reqwest::Client,
}

impl Default for EthereumProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl EthereumProvider {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(RPC_TIMEOUT)
            .build()
            .expect("fail trying to build reqwest client");
        Self { client }
    }

    pub async fn send_async(
        &self,
        method: &str,
        params: &[serde_json::Value],
    ) -> Result<serde_json::Value, anyhow::Error> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });
        let response = self
            .client
            .post(urls::ethereum_rpc_with_project("kernel-local"))
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    anyhow::anyhow!("{method}: Ethereum RPC request timed out")
                } else {
                    anyhow::anyhow!("{method} failed: {e}")
                }
            })?;
        let status = response.status();
        let text = response.text().await?;

        let Ok(mut response) = serde_json::from_str::<serde_json::Value>(&text) else {
            anyhow::bail!("{method} failed (HTTP {status}): {text}");
        };
        if let Some(error) = response.get("error").filter(|error| !error.is_null()) {
            if !error.is_object() {
                anyhow::bail!("{method} failed: {error}");
            }
            let mut message = format!(
                "{method} failed (code: {}, message: {}",
                error["code"],
                error["message"].as_str().unwrap_or_default()
            );
            if !error["data"].is_null() {
                message.push_str(&format!(", data: {}", error["data"]));
            }
            anyhow::bail!("{message})");
        }
        match response.get_mut("result") {
            Some(result) => Ok(result.take()),
            None => anyhow::bail!("{method} failed (HTTP {status}): {text}"),
        }
    }
}
