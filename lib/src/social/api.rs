use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Result};
use reqwest::{Method, StatusCode};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::ephemeral_auth_chain::EphemeralAuthChain;
use crate::auth::wallet::sign_request;

use super::types::{
    BlockedUserEntry, BlockingStatus, Friend, FriendshipRequestEntry, FriendshipStatusResponse,
    Paginated,
};

const REQUEST_TIMEOUT_SECS: u64 = 15;

/// Every endpoint wraps its payload in `{ "data": ... }`.
#[derive(Deserialize)]
struct Envelope<T> {
    data: T,
}

/// Failed endpoints answer `{ "error": "...", "message": "..." }`.
#[derive(Deserialize, Default)]
struct ErrorBody {
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum RequestsDirection {
    Received,
    Sent,
}

impl RequestsDirection {
    fn as_str(self) -> &'static str {
        match self {
            RequestsDirection::Received => "received",
            RequestsDirection::Sent => "sent",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum FriendshipRequestIntention {
    Accepted,
    Rejected,
    Cancelled,
}

impl FriendshipRequestIntention {
    fn as_str(self) -> &'static str {
        match self {
            FriendshipRequestIntention::Accepted => "accepted",
            FriendshipRequestIntention::Rejected => "rejected",
            FriendshipRequestIntention::Cancelled => "cancelled",
        }
    }
}

/// `limit`/`offset` query parameters. `None` lets the server use its default page (100).
#[derive(Debug, Clone, Copy)]
pub struct Pagination {
    pub limit: i32,
    pub offset: i32,
}

/// Signed (ADR-44) HTTP client for the social service API.
pub struct SocialApiClient {
    base_url: String,
    wallet: Arc<EphemeralAuthChain>,
    http: reqwest::Client,
}

impl SocialApiClient {
    pub fn new(base_url: String, wallet: Arc<EphemeralAuthChain>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .build()
            .expect("reqwest client with a timeout always builds");

        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            wallet,
            http,
        }
    }

    pub async fn get_friends(&self, pagination: Option<Pagination>) -> Result<Paginated<Friend>> {
        self.get("/v2/friends", &pagination_query(pagination)).await
    }

    pub async fn get_mutual_friends(
        &self,
        address: &str,
        pagination: Option<Pagination>,
    ) -> Result<Paginated<Friend>> {
        self.get(
            &format!("/v2/users/{address}/mutual-friends"),
            &pagination_query(pagination),
        )
        .await
    }

    pub async fn get_friendship_requests(
        &self,
        direction: RequestsDirection,
        pagination: Option<Pagination>,
    ) -> Result<Paginated<FriendshipRequestEntry>> {
        let mut query = pagination_query(pagination);
        query.push(("direction", direction.as_str().to_string()));
        self.get("/v2/friendship-requests", &query).await
    }

    pub async fn get_friendship_status(&self, address: &str) -> Result<FriendshipStatusResponse> {
        self.get(&format!("/v1/users/{address}/friendship-status"), &[])
            .await
    }

    pub async fn send_friendship_request(
        &self,
        address: &str,
        message: Option<String>,
    ) -> Result<FriendshipRequestEntry> {
        let mut body = json!({ "address": address });
        if let Some(message) = message {
            body["message"] = Value::String(message);
        }
        self.send(Method::POST, "/v1/friendship-requests", body)
            .await?
            .ok_or_else(|| anyhow!("Empty response sending a friendship request"))
    }

    pub async fn update_friendship_request(
        &self,
        address: &str,
        intention: FriendshipRequestIntention,
    ) -> Result<()> {
        self.send::<Value>(
            Method::PATCH,
            &format!("/v1/friendship-requests/{address}"),
            json!({ "intention": intention.as_str() }),
        )
        .await
        .map(|_| ())
    }

    pub async fn remove_friend(&self, address: &str) -> Result<()> {
        self.request::<Value>(Method::DELETE, &format!("/v1/friends/{address}"), &[], None)
            .await
            .map(|_| ())
    }

    pub async fn get_blocked_users(
        &self,
        pagination: Option<Pagination>,
    ) -> Result<Paginated<BlockedUserEntry>> {
        self.get("/v2/blocks", &pagination_query(pagination)).await
    }

    pub async fn block_user(&self, address: &str) -> Result<()> {
        self.send::<Value>(Method::POST, "/v1/blocks", json!({ "address": address }))
            .await
            .map(|_| ())
    }

    pub async fn unblock_user(&self, address: &str) -> Result<()> {
        self.request::<Value>(Method::DELETE, &format!("/v1/blocks/{address}"), &[], None)
            .await
            .map(|_| ())
    }

    pub async fn get_blocking_status(&self) -> Result<BlockingStatus> {
        self.get("/v1/blocking-status", &[]).await
    }

    async fn get<T: DeserializeOwned>(&self, path: &str, query: &[(&str, String)]) -> Result<T> {
        self.request(Method::GET, path, query, None)
            .await?
            .ok_or_else(|| anyhow!("Empty response from GET {path}"))
    }

    async fn send<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Value,
    ) -> Result<Option<T>> {
        self.request(method, path, &[], Some(body)).await
    }

    /// Sends a signed request. Returns `None` for an empty (204) response.
    async fn request<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, String)],
        body: Option<Value>,
    ) -> Result<Option<T>> {
        let url = format!("{}{}", self.base_url, path);
        // The signature covers the method and the path, never the query string.
        let uri: http::Uri = url
            .parse()
            .map_err(|e| anyhow!("Invalid social API URL {url}: {e}"))?;
        let headers = sign_request(method.as_str(), &uri, &self.wallet, json!({})).await;

        let mut request = self.http.request(method.clone(), &url).query(query);
        for (key, value) in headers {
            request = request.header(key, value);
        }
        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request
            .send()
            .await
            .map_err(|e| anyhow!("{method} {path} failed: {e}"))?;
        let status = response.status();

        if status == StatusCode::NO_CONTENT {
            return Ok(None);
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| anyhow!("{method} {path} failed reading the response: {e}"))?;

        if !status.is_success() {
            let error: ErrorBody = serde_json::from_slice(&bytes).unwrap_or_default();
            let reason = error
                .message
                .or(error.error)
                .unwrap_or_else(|| String::from_utf8_lossy(&bytes).into_owned());
            return Err(anyhow!("{method} {path} answered {status}: {reason}"));
        }

        let envelope: Envelope<T> = serde_json::from_slice(&bytes)
            .map_err(|e| anyhow!("{method} {path} returned an unexpected body: {e}"))?;
        Ok(Some(envelope.data))
    }
}

fn pagination_query(pagination: Option<Pagination>) -> Vec<(&'static str, String)> {
    match pagination {
        Some(Pagination { limit, offset }) => {
            vec![("limit", limit.to_string()), ("offset", offset.to_string())]
        }
        None => Vec::new(),
    }
}
