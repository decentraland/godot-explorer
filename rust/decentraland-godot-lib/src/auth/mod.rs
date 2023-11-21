use std::time::Duration;

use rand::Rng;
use serde::{de::DeserializeOwned, Deserialize};

pub mod wallet;

#[derive(Deserialize, Debug)]
struct GetAccountResponseData {
    address: String,
}

#[derive(Deserialize, Debug)]
struct GetAccountResponse {
    data: GetAccountResponseData,
}

#[derive(Deserialize, Debug)]
struct SignToServerResponseData {
    signature: String,
}

#[derive(Deserialize, Debug)]
struct SignToServerResponse {
    data: SignToServerResponseData,
}

pub struct RemoteWallet {
    address: String,
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

const AUTH_FRONT_URL: &str = "http://localhost:5173/";
const AUTH_SERVER_URL: &str = "http://localhost:3000/";

async fn fetch_server<T>(req_id: String) -> Result<T, ()>
where
    T: DeserializeOwned,
{
    loop {
        tracing::info!("trying req_id {:?}", req_id);

        let url = format!("{AUTH_SERVER_URL}task/{req_id}");
        let response = reqwest::get(url).await;

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
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        continue;
                    }

                    tracing::error!("Success fetching task but then fail: {:?}", response);
                }
            }
            Err(error) => {
                if let Some(status_code) = error.status() {
                    if status_code == http::StatusCode::NOT_FOUND {
                        tokio::time::sleep(Duration::from_secs(1)).await;
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

impl RemoteWallet {
    pub async fn try_fetch() -> Result<Self, ()> {
        // let address = "0x0000000".to_string();

        let get_account_req_id = gen_id();
        let uri = format!("{AUTH_FRONT_URL}get-account?id={get_account_req_id}");

        tracing::info!("uri: {:?}", uri);
        // godot::engine::Os::singleton().shell_open(uri.into());

        let account = fetch_server::<GetAccountResponse>(get_account_req_id).await?;

        let sign_payload_req_id = gen_id();
        let payload = "hello%20from%20rust"; // TODO: encode-uri
        let uri =
            format!("{AUTH_FRONT_URL}sign-to-server?id={sign_payload_req_id}&payload={payload}");

        tracing::info!("uri: {:?}", uri);
        // godot::engine::Os::singleton().shell_open(uri.into());

        let sign_payload = fetch_server::<SignToServerResponse>(sign_payload_req_id).await?;

        tracing::info!("account {:?} sign_payload: {:?}", account, sign_payload);

        Ok(Self {
            address: account.data.address,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use tracing_test::traced_test;

    #[traced_test]
    #[tokio::test]
    async fn test_gen_id() {
        let wallet = RemoteWallet::try_fetch().await;
    }
}
