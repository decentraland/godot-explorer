//! The Segment HTTP seam, so the service runs offline in tests.

use std::time::Duration;

use async_trait::async_trait;

use super::queue::SEGMENT_BATCH_URL;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

#[async_trait]
pub trait SegmentTransport: Send + Sync {
    /// Posts one `/v1/batch` body. `Err` is a transport failure (offline, timeout, DNS): the
    /// events are kept for the next flush. Any HTTP response is `Ok`; a rejected payload is not
    /// retried, so it cannot spin forever.
    async fn send_batch(&self, body: String) -> Result<(), String>;
}

pub struct ReqwestSegmentTransport {
    client: reqwest::Client,
}

impl ReqwestSegmentTransport {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent(crate::USER_AGENT)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .unwrap_or_default();
        Self { client }
    }
}

impl Default for ReqwestSegmentTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SegmentTransport for ReqwestSegmentTransport {
    async fn send_batch(&self, body: String) -> Result<(), String> {
        let response = self
            .client
            .post(SEGMENT_BATCH_URL)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !response.status().is_success() {
            tracing::warn!("Segment rejected a batch: {}", response.status());
        }
        Ok(())
    }
}
