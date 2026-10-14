//! The one place a request is signed with the identity key (ADR-44 signed fetch). Nothing here is
//! exported as a generic fetch: the typed calls of this module build their own URLs from `urls`
//! and call `SignedClient::request`, so the old `fetch.signed` allow-list is the type system.

use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use serde_json::Value;

use crate::{session::ephemeral_auth_chain::EphemeralAuthChain, session::wallet, Core};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignedError {
    /// No account is signed in (a guest has no identity to sign with).
    NotSignedIn,
    /// The URL a typed call built is not a valid URI.
    BadUrl(String),
    /// The request did not complete.
    Network(String),
    /// The server answered outside 2xx; `body` is its text.
    Status { status: u16, body: String },
    /// A 2xx answer that is not the shape the call expects.
    Malformed(String),
    /// The caller passed an argument the call cannot send.
    InvalidArgument(String),
}

impl std::fmt::Display for SignedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // Matches what the shell's guest detection looks for.
            Self::NotSignedIn => write!(f, "user not authenticated"),
            Self::BadUrl(url) => write!(f, "invalid url {url}"),
            Self::Network(detail) => write!(f, "network error: {detail}"),
            Self::Status { status, body } => write!(f, "status {status}: {body}"),
            Self::Malformed(detail) => write!(f, "invalid response format: {detail}"),
            Self::InvalidArgument(detail) => write!(f, "{detail}"),
        }
    }
}

impl std::error::Error for SignedError {}

/// What the server answered, whatever the status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    pub status: u16,
    pub body: String,
}

/// The HTTP seam, so the typed calls run offline in tests.
#[async_trait]
pub trait Transport: Send + Sync {
    async fn send(
        &self,
        method: &str,
        url: &str,
        headers: &[(String, String)],
        body: Option<&str>,
    ) -> Result<Reply, String>;
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
    async fn send(
        &self,
        method: &str,
        url: &str,
        headers: &[(String, String)],
        body: Option<&str>,
    ) -> Result<Reply, String> {
        let method = reqwest::Method::from_bytes(method.as_bytes()).map_err(|e| e.to_string())?;
        let mut request = self.client.request(method, url);
        for (name, value) in headers {
            request = request.header(name.as_str(), value.as_str());
        }
        if let Some(body) = body {
            request = request
                .header("Content-Type", "application/json")
                .body(body.to_owned());
        }
        let response = request.send().await.map_err(|e| e.to_string())?;
        let status = response.status().as_u16();
        let body = response.text().await.map_err(|e| e.to_string())?;
        Ok(Reply { status, body })
    }
}

/// A signed-in identity plus the way to reach the network.
pub struct SignedClient {
    transport: Arc<dyn Transport>,
    chain: EphemeralAuthChain,
    address: String,
}

impl SignedClient {
    pub fn new(transport: Arc<dyn Transport>, chain: EphemeralAuthChain, address: String) -> Self {
        Self {
            transport,
            chain,
            address,
        }
    }

    /// The client for the account signed in right now.
    pub fn live(core: &Core) -> Result<Self, SignedError> {
        let (chain, address) = core.with_session(|session| {
            (
                session.ephemeral_auth_chain().cloned(),
                session.address_string(),
            )
        });
        match chain {
            Some(chain) if !address.is_empty() => {
                Ok(Self::new(Arc::new(ReqwestTransport::new()), chain, address))
            }
            _ => Err(SignedError::NotSignedIn),
        }
    }

    /// The signed-in address, lowercase hex with `0x`.
    pub fn address(&self) -> &str {
        &self.address
    }

    /// Sends `method url` signed with the identity. The signed metadata is the JSON body, or `{}`
    /// when there is none (credits-server rejects a `null`); a non-2xx answer is an error.
    pub(super) async fn request(
        &self,
        method: &str,
        url: &str,
        body: Option<&Value>,
    ) -> Result<Reply, SignedError> {
        let uri = http::Uri::try_from(url).map_err(|_| SignedError::BadUrl(url.to_owned()))?;
        let body_text = body.map(Value::to_string);
        let metadata = body.cloned().unwrap_or_else(|| serde_json::json!({}));
        let headers = wallet::sign_request(method, &uri, &self.chain, metadata).await;
        let reply = self
            .transport
            .send(method, url, &headers, body_text.as_deref())
            .await
            .map_err(SignedError::Network)?;
        if (200..300).contains(&reply.status) {
            Ok(reply)
        } else {
            Err(SignedError::Status {
                status: reply.status,
                body: reply.body,
            })
        }
    }

    /// `request` whose answer must be a JSON value.
    pub(super) async fn request_json(
        &self,
        method: &str,
        url: &str,
        body: Option<&Value>,
    ) -> Result<Value, SignedError> {
        let reply = self.request(method, url, body).await?;
        serde_json::from_str(&reply.body).map_err(|e| SignedError::Malformed(e.to_string()))
    }
}

/// `base` with `segments` appended (each percent-encoded) and `query` pairs.
pub(super) fn build_url(
    base: &str,
    segments: &[&str],
    query: &[(&str, String)],
) -> Result<String, SignedError> {
    let mut url = url::Url::parse(base).map_err(|_| SignedError::BadUrl(base.to_owned()))?;
    {
        let mut path = url
            .path_segments_mut()
            .map_err(|_| SignedError::BadUrl(base.to_owned()))?;
        path.pop_if_empty();
        path.extend(segments);
    }
    if !query.is_empty() {
        url.query_pairs_mut().extend_pairs(query);
    }
    Ok(url.into())
}

#[cfg(test)]
pub(super) mod testing {
    use std::sync::Mutex;

    use ethers_signers::LocalWallet;
    use rand::thread_rng;

    use super::*;
    use crate::session::auth_identity::create_local_ephemeral;

    #[derive(Debug, Clone, PartialEq)]
    pub struct Sent {
        pub method: String,
        pub url: String,
        pub headers: Vec<(String, String)>,
        pub body: Option<String>,
    }

    impl Sent {
        pub fn header(&self, name: &str) -> Option<&str> {
            self.headers
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value.as_str())
        }
    }

    pub struct Mock {
        pub sent: Mutex<Vec<Sent>>,
        replies: Mutex<Vec<Result<Reply, String>>>,
    }

    impl Mock {
        pub fn answering(replies: Vec<Result<Reply, String>>) -> Arc<Self> {
            Arc::new(Self {
                sent: Mutex::new(Vec::new()),
                replies: Mutex::new(replies),
            })
        }

        pub fn ok(body: &str) -> Arc<Self> {
            Self::answering(vec![Ok(Reply {
                status: 200,
                body: body.to_owned(),
            })])
        }

        pub fn only_sent(&self) -> Sent {
            let sent = self.sent.lock().unwrap();
            assert_eq!(sent.len(), 1, "expected exactly one request");
            sent[0].clone()
        }
    }

    #[async_trait]
    impl Transport for Mock {
        async fn send(
            &self,
            method: &str,
            url: &str,
            headers: &[(String, String)],
            body: Option<&str>,
        ) -> Result<Reply, String> {
            self.sent.lock().unwrap().push(Sent {
                method: method.to_owned(),
                url: url.to_owned(),
                headers: headers.to_vec(),
                body: body.map(str::to_owned),
            });
            self.replies.lock().unwrap().remove(0)
        }
    }

    pub fn client(mock: &Arc<Mock>) -> SignedClient {
        let signer = LocalWallet::new(&mut thread_rng());
        SignedClient::new(
            mock.clone(),
            create_local_ephemeral(&signer),
            "0xabc0000000000000000000000000000000000001".to_owned(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::testing::*;
    use super::*;

    #[tokio::test]
    async fn requests_carry_the_identity_headers_and_signed_metadata() {
        let mock = Mock::ok("{}");
        let client = client(&mock);
        client
            .request(
                "PATCH",
                "https://x.example/api/a/b",
                Some(&serde_json::json!({"k": 1})),
            )
            .await
            .unwrap();
        let sent = mock.only_sent();
        assert_eq!(sent.method, "PATCH");
        assert_eq!(sent.body.as_deref(), Some("{\"k\":1}"));
        assert_eq!(sent.header("x-identity-metadata"), Some("{\"k\":1}"));
        assert!(sent.header("x-identity-timestamp").is_some());
        assert!(sent.header("x-identity-auth-chain-0").is_some());
        assert!(sent.header("x-identity-auth-chain-2").is_some());
    }

    #[tokio::test]
    async fn a_bodyless_request_signs_an_empty_object() {
        let mock = Mock::ok("{}");
        client(&mock)
            .request("GET", "https://x.example/api", None)
            .await
            .unwrap();
        let sent = mock.only_sent();
        assert_eq!(sent.body, None);
        assert_eq!(sent.header("x-identity-metadata"), Some("{}"));
    }

    #[tokio::test]
    async fn the_signature_covers_method_path_and_metadata() {
        let mock = Mock::ok("{}");
        client(&mock)
            .request("GET", "https://x.example/Api/Things?q=1", None)
            .await
            .unwrap();
        let sent = mock.only_sent();
        let timestamp = sent.header("x-identity-timestamp").unwrap();
        let signed_link = sent.header("x-identity-auth-chain-2").unwrap();
        let expected = format!("get:/api/things:{timestamp}:{{}}");
        assert!(signed_link.contains(&expected), "{signed_link}");
    }

    #[tokio::test]
    async fn non_2xx_and_transport_failures_are_errors() {
        let mock = Mock::answering(vec![
            Ok(Reply {
                status: 401,
                body: "nope".into(),
            }),
            Err("offline".into()),
        ]);
        let client = client(&mock);
        assert_eq!(
            client.request("GET", "https://x.example/", None).await,
            Err(SignedError::Status {
                status: 401,
                body: "nope".into()
            })
        );
        assert_eq!(
            client.request("GET", "https://x.example/", None).await,
            Err(SignedError::Network("offline".into()))
        );
    }

    #[test]
    fn urls_encode_segments_and_query() {
        assert_eq!(
            build_url(
                "https://p.example/api",
                &["places", "a b"],
                &[("search", "x y".to_owned())]
            )
            .unwrap(),
            "https://p.example/api/places/a%20b?search=x+y"
        );
    }
}
