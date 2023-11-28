use std::time::Duration;

use base64::Engine as _;
use ethers::types::H160;
use rand::Rng;
use serde::{de::DeserializeOwned, Deserialize};

use crate::auth::wallet::AsH160;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GetAccountResponseData {
    address: String,
    chain_id: u64,
}

#[derive(Deserialize, Debug)]
struct GetAccountResponse {
    data: GetAccountResponseData,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct SignToServerResponseData {
    account: String,
    signature: String,
    chain_id: u64,
}

#[derive(Deserialize, Debug)]
struct SignToServerResponse {
    data: SignToServerResponseData,
}

const AUTH_FRONT_URL: &str = "https://leanmendoza.github.io/decentraland-auth/";
const AUTH_SERVER_ENDPOINT_URL: &str = "https://services.aesir-online.net/dcltest/queue/task";
const AUTH_SERVER_RETRIES: i32 = 60;
const AUTH_SERVER_RETRY_INTERVAL: Duration = Duration::from_secs(1);

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

async fn fetch_server<T>(req_id: String) -> Result<T, ()>
where
    T: DeserializeOwned,
{
    let mut attempt = 0;
    loop {
        tracing::debug!("trying req_id {:?} attempt ${attempt}", req_id);
        if attempt >= AUTH_SERVER_RETRIES {
            return Err(());
        }
        attempt += 1;

        let url = format!("{AUTH_SERVER_ENDPOINT_URL}/{req_id}");
        let response = reqwest::Client::builder()
            .timeout(AUTH_SERVER_RETRY_INTERVAL)
            .build()
            .expect("reqwest build error")
            .get(url)
            .send()
            .await;

        match response {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<T>().await {
                        Ok(response_data) => {
                            return Ok(response_data);
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
                        tracing::error!("Error fetching task: {:?}", error);
                    }
                } else {
                    tracing::error!("Error fetching task: {:?}", error);
                }
            }
        }
        break;
    }
    Err(())
}

pub async fn get_account(
    url_reporter: tokio::sync::mpsc::Sender<RemoteReportState>,
) -> Result<(H160, u64), ()> {
    let get_account_req_id = gen_id();
    let server_endpoint = urlencoding::encode(AUTH_SERVER_ENDPOINT_URL);
    let open_url: String = format!(
        "{AUTH_FRONT_URL}get-account?id={get_account_req_id}&server-endpoint={server_endpoint}"
    );

    tracing::debug!("get_account url {:?}", open_url);
    url_reporter
        .send(RemoteReportState::OpenUrl {
            url: open_url.clone(),
            description: "Know your public address account".to_owned(),
        })
        .await
        .unwrap();

    let account = fetch_server::<GetAccountResponse>(get_account_req_id).await?;
    let Some(address) = account.data.address.as_h160() else {
        return Err(());
    };
    Ok((address, account.data.chain_id))
}

pub async fn remote_sign_message(
    payload: &[u8],
    by_signer: Option<H160>,
    url_reporter: tokio::sync::mpsc::Sender<RemoteReportState>,
) -> Result<(H160, String, u64), ()> {
    let address = if by_signer.is_some() {
        format!("{:#x}", by_signer.unwrap())
    } else {
        "".into()
    };
    let sign_payload_req_id = gen_id();
    let server_endpoint = urlencoding::encode(AUTH_SERVER_ENDPOINT_URL);
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload);
    let open_url =
        format!("{AUTH_FRONT_URL}sign-to-server?id={sign_payload_req_id}&payload={payload}&address={address}&server-endpoint={server_endpoint}");

    tracing::debug!("sign url {:?}", open_url);
    url_reporter
        .send(RemoteReportState::OpenUrl {
            url: open_url.clone(),
            description: "Sign a message".to_owned(),
        })
        .await
        .unwrap();

    let sign_payload = fetch_server::<SignToServerResponse>(sign_payload_req_id).await?;
    let Some(account) = sign_payload.data.account.as_h160() else {
        return Err(());
    };
    Ok((
        account,
        sign_payload.data.signature,
        sign_payload.data.chain_id,
    ))
}

#[cfg(test)]
mod test {
    use super::*;
    use tracing_test::traced_test;

    #[traced_test]
    #[tokio::test]
    async fn test_gen_id() {
        let (sx, _rx) = tokio::sync::mpsc::channel(100);
        let Ok((signer, signature, chain_id)) =
            remote_sign_message("hello".as_bytes(), None, sx).await
        else {
            return;
        };
        tracing::info!("signer {:?} signature {:?}", signer, signature);
    }
}
