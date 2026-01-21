use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::godot_classes::dcl_tokio_rpc::GodotTokioCall;

use super::wallet::SimpleAuthChain;

// Production
// const AUTH_FRONT_URL: &str = "https://decentraland.org/auth/requests";
// const AUTH_MOBILE_FRONT_URL: &str = "https://decentraland.org/auth/mobile";
// const AUTH_SERVER_ENDPOINT_URL: &str = "https://auth-api.decentraland.org/requests";
// const AUTH_SERVER_ENDPOINT_BASE_URL: &str = "https://auth-api.decentraland.org";

// Localhost with .zone auth-api
// const AUTH_FRONT_URL: &str = "http://localhost:5173/auth/requests";
// const AUTH_MOBILE_FRONT_URL: &str = "http://localhost:5173/auth/mobile";
// const AUTH_SERVER_ENDPOINT_URL: &str = "https://auth-api.decentraland.zone/requests";
// const AUTH_SERVER_ENDPOINT_BASE_URL: &str = "https://auth-api.decentraland.zone";

// Production
const AUTH_FRONT_URL: &str = "https://decentraland.org/auth/requests";
const AUTH_MOBILE_FRONT_URL: &str = "https://decentraland.org/auth/mobile";
const AUTH_SERVER_ENDPOINT_URL: &str = "https://auth-api.decentraland.org/requests";
const AUTH_SERVER_ENDPOINT_BASE_URL: &str = "https://auth-api.decentraland.org";

const AUTH_SERVER_RETRY_INTERVAL: Duration = Duration::from_secs(1);
const AUTH_SERVER_TIMEOUT: Duration = Duration::from_secs(600);
const AUTH_SERVER_RETRIES: u64 =
    AUTH_SERVER_TIMEOUT.as_secs() / AUTH_SERVER_RETRY_INTERVAL.as_secs();

const AUTH_SERVER_REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRequest {
    pub method: String,
    pub params: Vec<serde_json::Value>, // Using serde_json::Value for unknown[]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_chain: Option<SimpleAuthChain>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateRequestResponse {
    request_id: String,
    // expiration: serde_json::Value,
    code: i32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RequestResultError {
    // code: i32,
    message: String,
    // data: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RequestResult {
    sender: String,
    result: Option<serde_json::Value>,
    error: Option<RequestResultError>,
}

/// Response from the identity endpoint containing the full AuthIdentity
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityResponse {
    pub identity: AuthIdentity,
}

/// The AuthIdentity returned by the server for mobile auth flow
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthIdentity {
    pub ephemeral_identity: EphemeralIdentity,
    pub expiration: String, // ISO date string
    pub auth_chain: Vec<AuthLink>,
}

/// The ephemeral identity with private key provided by the server
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EphemeralIdentity {
    pub private_key: String,
    pub public_key: String,
    pub address: String,
}

/// A single link in the auth chain
#[derive(Debug, Deserialize, Clone)]
pub struct AuthLink {
    #[serde(rename = "type")]
    pub ty: String,
    pub payload: String,
    #[serde(default)]
    pub signature: String,
}

/// Fetches an identity result by ID from the auth server.
/// Used for mobile deep link auth flow where we receive the identity ID via deep link
/// instead of polling for the result.
/// Returns the full AuthIdentity including ephemeral private key and auth chain.
pub async fn fetch_identity_by_id(identity_id: String) -> Result<IdentityResponse, anyhow::Error> {
    let url = format!("{AUTH_SERVER_ENDPOINT_BASE_URL}/identities/{identity_id}");
    tracing::debug!(
        "fetch_identity_by_id: requesting identity_id={}, url={}",
        identity_id,
        url
    );

    let response = reqwest::Client::builder()
        .timeout(AUTH_SERVER_REQUEST_TIMEOUT)
        .build()
        .expect("reqwest build error")
        .get(&url)
        .send()
        .await?;

    let status = response.status();
    tracing::debug!(
        "fetch_identity_by_id: received response status={} for identity_id={}",
        status,
        identity_id
    );

    if status.as_u16() == 204 {
        tracing::debug!(
            "fetch_identity_by_id: identity not ready yet (204) for identity_id={}",
            identity_id
        );
        return Err(anyhow::Error::msg("Identity not ready yet (204)"));
    }

    if !status.is_success() {
        let status_code = status.as_u16();
        tracing::error!(
            "fetch_identity_by_id: error status={} for identity_id={}",
            status_code,
            identity_id
        );
        return Err(anyhow::Error::msg(format!(
            "Error fetching identity: status {}",
            status_code
        )));
    }

    let result = response.json::<IdentityResponse>().await?;
    tracing::debug!(
        "fetch_identity_by_id: successfully parsed identity response for identity_id={}, full response: \
        address={}, \
        public_key={}, \
        private_key_len={}, \
        expiration={}, \
        auth_chain_len={}, \
        auth_chain={:?}",
        identity_id,
        result.identity.ephemeral_identity.address,
        result.identity.ephemeral_identity.public_key,
        result.identity.ephemeral_identity.private_key.len(),
        result.identity.expiration,
        result.identity.auth_chain.len(),
        result.identity.auth_chain.iter().map(|link| format!(
            "AuthLink {{ type: {}, payload_len: {}, signature_len: {} }}",
            link.ty,
            link.payload.len(),
            link.signature.len()
        )).collect::<Vec<_>>()
    );
    Ok(result)
}

async fn fetch_polling_server(
    req_id: String,
) -> Result<(String, serde_json::Value), anyhow::Error> {
    let url = format!("{AUTH_SERVER_ENDPOINT_URL}/{req_id}");
    tracing::debug!(
        "fetch_polling_server: starting polling for req_id={}, url={}, max_retries={}, timeout={}s",
        req_id,
        url,
        AUTH_SERVER_RETRIES,
        AUTH_SERVER_TIMEOUT.as_secs()
    );
    let mut attempt = 0;
    let mut requested_time = std::time::Instant::now();

    loop {
        tracing::debug!(
            "fetch_polling_server: attempt {}/{} for req_id={}",
            attempt + 1,
            AUTH_SERVER_RETRIES,
            req_id
        );
        if attempt >= AUTH_SERVER_RETRIES {
            tracing::warn!(
                "fetch_polling_server: max retries ({}) exceeded for req_id={}",
                AUTH_SERVER_RETRIES,
                req_id
            );
            return Err(anyhow::Error::msg("too many atempts"));
        }
        attempt += 1;

        let diff = (std::time::Instant::now() - requested_time).as_millis() as i64;
        let remaining_delay = (AUTH_SERVER_RETRY_INTERVAL.as_millis() as i64) - diff;
        if remaining_delay > 0 {
            tracing::trace!(
                "fetch_polling_server: sleeping for {}ms before next attempt",
                remaining_delay
            );
            tokio::time::sleep(Duration::from_millis(remaining_delay as u64)).await;
        }

        requested_time = std::time::Instant::now();
        tracing::trace!("fetch_polling_server: sending GET request to {}", url);
        let response = reqwest::Client::builder()
            .timeout(AUTH_SERVER_REQUEST_TIMEOUT)
            .build()
            .expect("reqwest build error")
            .get(url.clone())
            .send()
            .await;

        let response = match response {
            Ok(response) => {
                let status = response.status();
                tracing::debug!(
                    "fetch_polling_server: received status={} for req_id={}",
                    status,
                    req_id
                );
                if status.as_u16() == 204 {
                    tracing::debug!(
                        "fetch_polling_server: result not ready yet (204), continuing polling for req_id={}",
                        req_id
                    );
                    continue;
                } else if status.is_success() {
                    match response.json::<RequestResult>().await {
                        Ok(response) => {
                            tracing::debug!(
                                "fetch_polling_server: parsed response for req_id={}, sender={}, has_result={}, has_error={}",
                                req_id,
                                response.sender,
                                response.result.is_some(),
                                response.error.is_some()
                            );
                            if let Some(response_data) = response.result {
                                tracing::debug!(
                                    "fetch_polling_server: success! got result for req_id={} from sender={}",
                                    req_id,
                                    response.sender
                                );
                                Ok((response.sender, response_data))
                            } else if let Some(error) = response.error {
                                tracing::error!(
                                    "fetch_polling_server: server returned error for req_id={}: {}",
                                    req_id,
                                    error.message
                                );
                                Err(anyhow::Error::msg(error.message))
                            } else {
                                tracing::error!(
                                    "fetch_polling_server: invalid response (no result or error) for req_id={}",
                                    req_id
                                );
                                Err(anyhow::Error::msg("invalid response"))
                            }
                        }
                        Err(error) => {
                            tracing::error!(
                                "fetch_polling_server: failed to parse response JSON for req_id={}: {:?}",
                                req_id,
                                error
                            );
                            Err(anyhow::Error::msg(format!(
                                "error while parsing a task {:?}",
                                error
                            )))
                        }
                    }
                } else {
                    tracing::error!(
                        "fetch_polling_server: unexpected status={} for req_id={}",
                        status,
                        req_id
                    );
                    Err(anyhow::Error::msg(format!(
                        "Success fetching task but then fail: {:?}",
                        response
                    )))
                }
            }
            Err(error) => {
                if let Some(status_code) = error.status() {
                    tracing::error!(
                        "fetch_polling_server: request error with status={} for req_id={}: {:?}",
                        status_code,
                        req_id,
                        error
                    );
                    Err(anyhow::Error::msg(format!(
                        "Error fetching task with status {:?}: {:?}",
                        status_code, error
                    )))
                } else {
                    tracing::error!(
                        "fetch_polling_server: request error (no status) for req_id={}: {:?}",
                        req_id,
                        error
                    );
                    Err(anyhow::Error::msg(format!(
                        "Error fetching task: {:?}",
                        error
                    )))
                }
            }
        };

        if response.is_err() {
            tracing::error!(
                "fetch_polling_server: error on attempt {}, will retry for req_id={}: {:?}",
                attempt,
                req_id,
                response.as_ref().err()
            );
            continue;
        }

        tracing::info!(
            "fetch_polling_server: completed successfully after {} attempts for req_id={}",
            attempt,
            req_id
        );
        return response;
    }
}

async fn create_new_request(
    message: CreateRequest,
) -> Result<CreateRequestResponse, anyhow::Error> {
    tracing::debug!(
        "create_new_request: creating request with method={}, params_count={}, has_auth_chain={}",
        message.method,
        message.params.len(),
        message.auth_chain.is_some()
    );

    let body = serde_json::to_string(&message).expect("valid json");
    tracing::trace!(
        "create_new_request: POST to {} with body length={}",
        AUTH_SERVER_ENDPOINT_URL,
        body.len()
    );

    let response = reqwest::Client::builder()
        .timeout(AUTH_SERVER_REQUEST_TIMEOUT)
        .build()
        .expect("reqwest build error")
        .post(AUTH_SERVER_ENDPOINT_URL)
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await?;

    let status = response.status();
    tracing::debug!("create_new_request: received response status={}", status);

    if status.is_success() {
        let result = response.json::<CreateRequestResponse>().await?;
        tracing::debug!(
            "create_new_request: success! request_id={}, code={}",
            result.request_id,
            result.code
        );
        Ok(result)
    } else {
        let status_code = status.as_u16();
        let response_text = response.text().await?;
        tracing::error!(
            "create_new_request: failed with status={}, response={}",
            status_code,
            response_text
        );
        Err(anyhow::Error::msg(format!(
            "Error creating request {status_code}: ${response_text}"
        )))
    }
}

/// Creates an auth request and opens the browser for mobile.
/// Instead of polling, the app should wait for a deep link with the identity ID.
/// Returns the request_id that will be received via deep link `decentraland://open?signin=${request_id}`
pub async fn do_request_mobile(
    _message: CreateRequest,
    url_reporter: tokio::sync::mpsc::Sender<GodotTokioCall>,
    provider: Option<String>,
) -> Result<(), anyhow::Error> {
    tracing::debug!(
        "do_request_mobile: starting mobile auth request, provider={:?}",
        provider
    );

    // Build URL with optional provider parameter
    let url = if let Some(provider) = provider {
        format!("{}?provider={}", AUTH_MOBILE_FRONT_URL, provider)
    } else {
        AUTH_MOBILE_FRONT_URL.to_string()
    };
    tracing::debug!("do_request_mobile: opening auth URL={}", url);

    url_reporter
        .send(GodotTokioCall::OpenUrl {
            url,
            description: "".into(),
            use_webview: true,
        })
        .await?;

    tracing::debug!("do_request_mobile: auth URL sent to Godot, waiting for deep link callback");

    Ok(())
}

pub async fn do_request(
    message: CreateRequest,
    url_reporter: tokio::sync::mpsc::Sender<GodotTokioCall>,
) -> Result<(String, serde_json::Value), anyhow::Error> {
    tracing::debug!(
        "do_request: starting auth request, method={}",
        message.method
    );

    let request = create_new_request(message).await?;
    let req_id = request.request_id;
    let code = request.code;
    tracing::debug!(
        "do_request: request created with req_id={}, code={}",
        req_id,
        code
    );

    let url = format!("{AUTH_FRONT_URL}/{req_id}?targetConfigId=alternative");
    tracing::debug!("do_request: opening auth URL={}", url);

    url_reporter
        .send(GodotTokioCall::OpenUrl {
            url: url.clone(),
            description: "".into(),
            use_webview: true,
        })
        .await?;

    tracing::debug!(
        "do_request: auth URL sent to Godot, starting polling for req_id={}",
        req_id
    );

    let result = fetch_polling_server(req_id.clone()).await;
    match &result {
        Ok((sender, _)) => {
            tracing::debug!(
                "do_request: completed successfully for req_id={}, sender={}",
                req_id,
                sender
            );
        }
        Err(e) => {
            tracing::error!("do_request: failed for req_id={}: {:?}", req_id, e);
        }
    }
    result
}

impl CreateRequest {
    pub fn from_new_ephemeral(ephemeral_message: &str) -> Self {
        tracing::debug!(
            "CreateRequest::from_new_ephemeral: creating request with message_len={}",
            ephemeral_message.len()
        );
        Self {
            method: "dcl_personal_sign".to_owned(),
            params: vec![ephemeral_message.into()],
            auth_chain: None,
        }
    }

    pub fn from_send_async_ephemeral(
        method: String,
        params: Vec<serde_json::Value>,
        auth_chain: SimpleAuthChain,
    ) -> Self {
        tracing::debug!(
            "CreateRequest::from_send_async_ephemeral: creating request with method={}, params_count={}",
            method,
            params.len(),
        );
        Self {
            method,
            params,
            auth_chain: Some(auth_chain),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::auth::wallet::Wallet;

    use super::super::auth_identity::{get_ephemeral_message, AUTH_CHAIN_EXPIRATION_SECS};
    use super::*;
    use ethers_signers::LocalWallet;
    use rand::thread_rng;
    use tracing_test::traced_test;

    #[traced_test]
    #[tokio::test]
    async fn test_gen_id() {
        let (sx, mut rx) = tokio::sync::mpsc::channel(100);

        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Some(GodotTokioCall::OpenUrl {
                        url,
                        description,
                        use_webview,
                    }) => {
                        tracing::info!(
                            "url {:?} description {:?} use_webview {:?}",
                            url,
                            description,
                            use_webview
                        );
                    }
                    _ => {
                        break;
                    }
                }
            }
        });

        let local_wallet = LocalWallet::new(&mut thread_rng());
        let ephemeral_wallet = Wallet::new_from_inner(Box::new(local_wallet));
        let ephemeral_address = format!("{:#x}", ephemeral_wallet.address());
        let expiration = std::time::SystemTime::now()
            + std::time::Duration::from_secs(AUTH_CHAIN_EXPIRATION_SECS);
        let ephemeral_message = get_ephemeral_message(ephemeral_address.as_str(), expiration);

        let result = do_request(
            CreateRequest {
                method: "dcl_personal_sign".to_owned(),
                params: vec![ephemeral_message.into()],
                auth_chain: None,
            },
            sx,
        )
        .await;

        tracing::info!("result {:?}", result);
    }
}
