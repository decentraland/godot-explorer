use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::godot_classes::dcl_tokio_rpc::GodotTokioCall;

use super::wallet::SimpleAuthChain;

const AUTH_FRONT_URL: &str = "https://decentraland.zone/auth/requests";
const AUTH_SERVER_ENDPOINT_URL: &str = "https://auth-api.decentraland.zone/requests";
// const AUTH_FRONT_URL: &str = "https://localhost:5173/auth/requests";
// const AUTH_SERVER_ENDPOINT_URL: &str = "https://auth-api.decentraland.zone/requests";

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

async fn fetch_polling_server(
    req_id: String,
) -> Result<(String, serde_json::Value), anyhow::Error> {
    let url = format!("{AUTH_SERVER_ENDPOINT_URL}/{req_id}");
    let mut attempt = 0;
    let mut requested_time = std::time::Instant::now();

    loop {
        tracing::debug!("trying req_id {:?} attempt ${attempt}", req_id);
        if attempt >= AUTH_SERVER_RETRIES {
            return Err(anyhow::Error::msg("too many atempts"));
        }
        attempt += 1;

        let diff = (std::time::Instant::now() - requested_time).as_millis() as i64;
        let remaining_delay = (AUTH_SERVER_RETRY_INTERVAL.as_millis() as i64) - diff;
        if remaining_delay > 0 {
            tokio::time::sleep(Duration::from_millis(remaining_delay as u64)).await;
        }

        requested_time = std::time::Instant::now();
        let response = reqwest::Client::builder()
            .timeout(AUTH_SERVER_REQUEST_TIMEOUT)
            .build()
            .expect("reqwest build error")
            .get(url.clone())
            .send()
            .await;

        let response = match response {
            Ok(response) => {
                if response.status().as_u16() == 204 {
                    continue;
                } else if response.status().is_success() {
                    match response.json::<RequestResult>().await {
                        Ok(response) => {
                            if let Some(response_data) = response.result {
                                Ok((response.sender, response_data))
                            } else if let Some(error) = response.error {
                                Err(anyhow::Error::msg(error.message))
                            } else {
                                Err(anyhow::Error::msg("invalid response"))
                            }
                        }
                        Err(error) => Err(anyhow::Error::msg(format!(
                            "error while parsing a task {:?}",
                            error
                        ))),
                    }
                } else {
                    Err(anyhow::Error::msg(format!(
                        "Success fetching task but then fail: {:?}",
                        response
                    )))
                }
            }
            Err(error) => {
                if let Some(status_code) = error.status() {
                    Err(anyhow::Error::msg(format!(
                        "Error fetching task with status {:?}: {:?}",
                        status_code, error
                    )))
                } else {
                    Err(anyhow::Error::msg(format!(
                        "Error fetching task: {:?}",
                        error
                    )))
                }
            }
        };

        if response.is_err() {
            tracing::error!("error fetching task: {:?}", response.err().unwrap());
            continue;
        }

        return response;
    }
}

async fn create_new_request(
    message: CreateRequest,
) -> Result<CreateRequestResponse, anyhow::Error> {
    let body = serde_json::to_string(&message).expect("valid json");
    let response = reqwest::Client::builder()
        .timeout(AUTH_SERVER_REQUEST_TIMEOUT)
        .build()
        .expect("reqwest build error")
        .post(AUTH_SERVER_ENDPOINT_URL)
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await?;

    if response.status().is_success() {
        Ok(response.json::<CreateRequestResponse>().await?)
    } else {
        let status_code = response.status().as_u16();
        let response = response.text().await?;
        Err(anyhow::Error::msg(format!(
            "Error creating request {status_code}: ${response}"
        )))
    }
}

pub async fn do_request(
    message: CreateRequest,
    url_reporter: tokio::sync::mpsc::Sender<GodotTokioCall>,
    target_config_id: Option<String>,
) -> Result<(String, serde_json::Value), anyhow::Error> {
    let request = create_new_request(message).await?;
    let req_id = request.request_id;
    let code = request.code;
    println!("code is: {}", code);

    // Determine target_config_id based on OS or use the provided one
    let target_config_id = target_config_id.unwrap_or_else(|| match std::env::consts::OS {
        "ios" => "ios".to_string(),
        "android" => "android".to_string(),
        _ => "alternative".to_string(),
    });

    let url = format!("{AUTH_FRONT_URL}/{req_id}?targetConfigId={target_config_id}");
    url_reporter
        .send(GodotTokioCall::OpenUrl {
            url,
            description: "".into(),
            use_webview: true,
        })
        .await?;

    fetch_polling_server(req_id).await
}

impl CreateRequest {
    pub fn from_new_ephemeral(ephemeral_message: &str) -> Self {
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

    use super::super::auth_identity::get_ephemeral_message;
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
        let expiration =
            std::time::SystemTime::now() + std::time::Duration::from_secs(30 * 24 * 3600);
        let ephemeral_message = get_ephemeral_message(ephemeral_address.as_str(), expiration);

        let result = do_request(
            CreateRequest {
                method: "dcl_personal_sign".to_owned(),
                params: vec![ephemeral_message.into()],
                auth_chain: None,
            },
            sx,
            None,
        )
        .await;

        tracing::info!("result {:?}", result);
    }
}
