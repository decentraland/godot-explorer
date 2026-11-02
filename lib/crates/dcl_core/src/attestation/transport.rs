//! HTTP seam for the mobile-bff calls, so tests run offline.

use std::time::Duration;

use async_trait::async_trait;
use serde_json::Value;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

/// A failed request; `code` is the server's `code` field when the body carried one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpFailure {
    pub code: Option<String>,
    pub message: String,
}

#[async_trait]
pub trait Transport: Send + Sync {
    /// POSTs `body` and returns the parsed JSON object of a 2xx response.
    async fn post_json(
        &self,
        url: &str,
        body: String,
        headers: &[(&str, &str)],
    ) -> Result<Value, HttpFailure>;
}

pub struct ReqwestTransport {
    client: reqwest::Client,
}

impl ReqwestTransport {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent(crate::USER_AGENT)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .unwrap_or_default();
        Self { client }
    }
}

impl Default for ReqwestTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Transport for ReqwestTransport {
    async fn post_json(
        &self,
        url: &str,
        body: String,
        headers: &[(&str, &str)],
    ) -> Result<Value, HttpFailure> {
        let mut request = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .body(body);
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        let response = request.send().await.map_err(|e| HttpFailure {
            code: None,
            message: e.to_string(),
        })?;
        let status = response.status();
        let text = response.text().await.map_err(|e| HttpFailure {
            code: None,
            message: e.to_string(),
        })?;
        let parsed: Option<Value> = serde_json::from_str(&text).ok();
        if !status.is_success() {
            let code = parsed
                .as_ref()
                .and_then(|v| v.get("code"))
                .and_then(Value::as_str)
                .map(str::to_owned);
            return Err(HttpFailure {
                code,
                message: format!("status {status}: {text}"),
            });
        }
        match parsed {
            Some(v) if v.is_object() => Ok(v),
            _ => Err(HttpFailure {
                code: None,
                message: "unparseable response".into(),
            }),
        }
    }
}
