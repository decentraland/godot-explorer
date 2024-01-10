use std::{str::FromStr, time::Duration};

use base64::Engine as _;
use ethers::types::{Signature, H160};
use rand::Rng;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::auth::wallet::AsH160;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct SignResponseData {
    account: String,
    signature: String,
    chain_id: u64,
}

#[derive(Deserialize, Debug)]
struct RemoteWalletResponse<T> {
    ok: bool,
    reason: Option<String>,
    response: Option<T>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RPCSendableMessage {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    pub params: Vec<serde_json::Value>, // Using serde_json::Value for unknown[]
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RemoteWalletRequest {
    #[serde(rename = "send-async", rename_all = "camelCase")]
    SendAsync {
        body: RPCSendableMessage,
        #[serde(skip_serializing_if = "Option::is_none")]
        by_address: Option<String>,
    },
    #[serde(rename = "sign", rename_all = "camelCase")]
    Sign {
        b64_message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        by_address: Option<String>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct RegisterRequestBody {
    id: String,
    request: RemoteWalletRequest,
}

const AUTH_FRONT_URL: &str = "https://auth.dclexplorer.com/";
const AUTH_SERVER_ENDPOINT_URL: &str = "https://auth-server.dclexplorer.com/task/";
// const AUTH_FRONT_URL: &str = "http://localhost:5173/";
// const AUTH_SERVER_ENDPOINT_URL: &str = "http://localhost:5545/task/";

const AUTH_SERVER_RETRY_INTERVAL: Duration = Duration::from_secs(1);
const AUTH_SERVER_TIMEOUT: Duration = Duration::from_secs(600);
const AUTH_SERVER_RETRIES: u64 =
    AUTH_SERVER_TIMEOUT.as_secs() / AUTH_SERVER_RETRY_INTERVAL.as_secs();

const AUTH_SERVER_REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

pub enum RemoteReportState {
    OpenUrl { url: String, description: String },
}

pub fn gen_id() -> String {
    rand::thread_rng()
        .sample_iter(rand::distributions::Alphanumeric)
        .take(56)
        .collect::<Vec<u8>>()
        .into_iter()
        .map(|byte| byte as char)
        .collect()
}

async fn fetch_server<T>(req_id: String) -> Result<T, anyhow::Error>
where
    T: DeserializeOwned,
{
    let url = format!("{AUTH_SERVER_ENDPOINT_URL}{req_id}/response");
    let mut attempt = 0;
    loop {
        tracing::debug!("trying req_id {:?} attempt ${attempt}", req_id);
        if attempt >= AUTH_SERVER_RETRIES {
            return Err(anyhow::Error::msg("too many atempts"));
        }
        attempt += 1;

        let response = reqwest::Client::builder()
            .timeout(AUTH_SERVER_REQUEST_TIMEOUT)
            .build()
            .expect("reqwest build error")
            .get(url.clone())
            .send()
            .await;

        match response {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<RemoteWalletResponse<T>>().await {
                        Ok(response) => {
                            if let Some(response_data) = response.response {
                                return Ok(response_data);
                            } else if let Some(reason) = response.reason {
                                return Err(anyhow::Error::msg(reason));
                            } else {
                                tracing::error!("invalid response ok={:?}", response.ok);
                            }
                        }
                        Err(error) => {
                            tracing::error!("error while parsing a task {:?}", error);
                        }
                    }
                } else {
                    if response.status() == http::StatusCode::NOT_FOUND {
                        tokio::time::sleep(AUTH_SERVER_RETRY_INTERVAL).await;
                        continue;
                    }

                    tracing::error!("Success fetching task but then fail: {:?}", response);
                }
            }
            Err(error) => {
                if let Some(status_code) = error.status() {
                    if status_code == http::StatusCode::NOT_FOUND {
                        tokio::time::sleep(AUTH_SERVER_RETRY_INTERVAL).await;
                        continue;
                    } else {
                        tracing::error!("Error fetching task with status: {:?}", error);
                    }
                } else {
                    tracing::error!("Error fetching task: {:?}", error);
                }
            }
        }
        break;
    }
    Err(anyhow::Error::msg("couldn't get response"))
}

async fn register_request(
    req_id: String,
    request: RemoteWalletRequest,
) -> Result<(), anyhow::Error> {
    let body = RegisterRequestBody {
        id: req_id,
        request,
    };
    let body = serde_json::to_string(&body).expect("valid json");
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
        Ok(())
    } else {
        tracing::error!("Error registering request: {:?}", response);
        Err(anyhow::Error::msg("couldn't get response"))
    }
}

async fn generate_and_report_request(
    request: RemoteWalletRequest,
    url_reporter: tokio::sync::mpsc::Sender<RemoteReportState>,
) -> Result<String, anyhow::Error> {
    let req_id = gen_id();
    register_request(req_id.clone(), request).await?;
    let open_url = format!("{AUTH_FRONT_URL}remote-wallet/{req_id}");

    tracing::debug!("sign url {:?}", open_url);
    url_reporter
        .send(RemoteReportState::OpenUrl {
            url: open_url.clone(),
            description: "Sign a message".to_owned(),
        })
        .await?;

    Ok(req_id)
}

pub async fn remote_sign_message(
    payload: &[u8],
    by_signer: Option<H160>,
    url_reporter: tokio::sync::mpsc::Sender<RemoteReportState>,
) -> Result<(H160, Signature, u64), anyhow::Error> {
    let by_address = by_signer.map(|s| format!("{:#x}", s));
    let b64_message = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload);

    let req_id = generate_and_report_request(
        RemoteWalletRequest::Sign {
            b64_message,
            by_address,
        },
        url_reporter,
    )
    .await?;

    let sign_payload = fetch_server::<SignResponseData>(req_id).await?;
    let Some(account) = sign_payload.account.as_h160() else {
        return Err(anyhow::Error::msg("invalid account"));
    };
    let Ok(signature) = Signature::from_str(sign_payload.signature.as_str()) else {
        return Err(anyhow::Error::msg("invalid signature"));
    };

    Ok((account, signature, sign_payload.chain_id))
}

pub async fn remote_send_async(
    message: RPCSendableMessage,
    by_signer: Option<H160>,
    url_reporter: tokio::sync::mpsc::Sender<RemoteReportState>,
) -> Result<serde_json::Value, anyhow::Error> {
    let by_address = by_signer.map(|s| format!("{:#x}", s));
    let req_id = generate_and_report_request(
        RemoteWalletRequest::SendAsync {
            body: message,
            by_address,
        },
        url_reporter,
    )
    .await?;

    fetch_server::<serde_json::Value>(req_id).await
}

#[cfg(test)]
mod test {
    use super::*;
    use tracing_test::traced_test;

    #[traced_test]
    #[tokio::test]
    async fn test_gen_id() {
        let (sx, mut rx) = tokio::sync::mpsc::channel(100);

        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Some(RemoteReportState::OpenUrl { url, description }) => {
                        tracing::info!("url {:?} description {:?}", url, description);
                    }
                    None => {
                        break;
                    }
                }
            }
        });

        let Ok((signer, signature, _chain_id)) =
            remote_sign_message("hello".as_bytes(), None, sx).await
        else {
            return;
        };
        tracing::info!("signer {:?} signature {:?}", signer, signature);
    }

    #[traced_test]
    #[tokio::test]
    async fn test_send_async() {
        let (sx, mut rx) = tokio::sync::mpsc::channel(100);

        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Some(RemoteReportState::OpenUrl { url, description }) => {
                        tracing::info!("url {:?} description {:?}", url, description);
                    }
                    None => {
                        break;
                    }
                }
            }
        });

        let Ok(value) = remote_send_async(
            RPCSendableMessage {
                jsonrpc: "2.0".to_owned(),
                id: 1,
                method: "eth_chainId".to_owned(),
                params: vec![],
            },
            None,
            sx,
        )
        .await
        else {
            return;
        };
        tracing::info!("value {:?} ", value);
    }
}
