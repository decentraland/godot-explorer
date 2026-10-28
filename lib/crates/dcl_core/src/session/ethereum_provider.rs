use ethers_providers::{Provider, Ws};
use tokio::sync::Mutex;

use crate::urls;

pub struct EthereumProvider {
    provider: Mutex<Option<Provider<Ws>>>,
}

impl Default for EthereumProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl EthereumProvider {
    pub fn new() -> Self {
        Self {
            provider: Mutex::new(None),
        }
    }

    pub async fn send_async(
        &self,
        method: &str,
        params: &[serde_json::Value],
    ) -> Result<serde_json::Value, anyhow::Error> {
        let mut this_provider = self.provider.lock().await;

        let provider = match this_provider.take() {
            Some(provider) => provider,
            None => {
                let provider_url = urls::ethereum_rpc_with_project("kernel-local");
                Provider::<Ws>::connect(&provider_url).await?
            }
        };
        let result = provider.request(method, params).await;

        match result {
            Err(e) => Err(anyhow::Error::new(e)),
            Ok(result) => {
                *this_provider = Some(provider);
                Ok(result)
            }
        }
    }
}
