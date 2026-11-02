//! The network seam of the gates, so every decision is testable offline.

use std::time::Duration;

use async_trait::async_trait;
use serde_json::Value;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Why a signed request did not succeed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignedFetchError {
    /// Nobody is signed in, so there is nothing to sign with.
    NoIdentity,
    /// The server (or the network) refused; the message is all that survives.
    Rejected(String),
}

#[async_trait]
pub trait Fetch: Send + Sync {
    /// GETs `url` and returns the parsed JSON of a 2xx response.
    async fn get_json(&self, url: &str) -> Result<Value, String>;
    /// POSTs a JSON `body` and returns the parsed JSON of a 2xx response.
    async fn post_json(&self, url: &str, body: String) -> Result<Value, String>;
    /// POSTs to `url` signed with the session's identity, `metadata` being the signed-fetch
    /// metadata JSON. Succeeds on any 2xx.
    async fn signed_post(&self, url: &str, metadata: &str) -> Result<(), SignedFetchError>;
}

pub struct ReqwestFetch {
    client: reqwest::Client,
}

impl ReqwestFetch {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent(crate::USER_AGENT)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .unwrap_or_default();
        Self { client }
    }
}

impl Default for ReqwestFetch {
    fn default() -> Self {
        Self::new()
    }
}

async fn json_of(response: reqwest::Response) -> Result<Value, String> {
    let status = response.status();
    if !status.is_success() {
        return Err(format!("HTTP {status}"));
    }
    response.json::<Value>().await.map_err(|e| e.to_string())
}

#[async_trait]
impl Fetch for ReqwestFetch {
    async fn get_json(&self, url: &str) -> Result<Value, String> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        json_of(response).await
    }

    async fn post_json(&self, url: &str, body: String) -> Result<Value, String> {
        let response = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        json_of(response).await
    }

    async fn signed_post(&self, url: &str, metadata: &str) -> Result<(), SignedFetchError> {
        let chain = crate::Core::try_get()
            .and_then(|core| core.with_session(|s| s.ephemeral_auth_chain().cloned()))
            .ok_or(SignedFetchError::NoIdentity)?;
        let uri: http::Uri = url
            .parse()
            .map_err(|e: http::uri::InvalidUri| SignedFetchError::Rejected(e.to_string()))?;
        let meta: Value = serde_json::from_str(metadata)
            .map_err(|e| SignedFetchError::Rejected(e.to_string()))?;
        let headers = crate::session::wallet::sign_request("post", &uri, &chain, meta).await;

        let mut request = self.client.post(url);
        for (name, value) in headers {
            request = request.header(name, value);
        }
        let response = request
            .send()
            .await
            .map_err(|e| SignedFetchError::Rejected(e.to_string()))?;
        let status = response.status();
        if status.is_success() {
            return Ok(());
        }
        let body = response.text().await.unwrap_or_default();
        Err(SignedFetchError::Rejected(format!("HTTP {status}: {body}")))
    }
}
