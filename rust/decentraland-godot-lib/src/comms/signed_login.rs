// https://github.com/decentraland/hammurabi/pull/33/files#diff-18afcd5f94e3688aad1ba36fa1db3e09b472b271d1e0cf5aeb59ebd32f43a328

use http::{Method, Uri};

use crate::{
    auth::ephemeral_auth_chain::EphemeralAuthChain,
    http_request::{
        http_requester::HttpRequester,
        request_response::{RequestOption, ResponseEnum, ResponseType},
    },
};

#[derive(Debug, serde::Deserialize)]
pub struct SignedLoginResponse {
    pub message: Option<String>,
    #[serde(rename = "fixedAdapter")]
    pub fixed_adapter: Option<String>,
}

#[derive(serde::Serialize)]
pub struct SignedLoginMeta {
    pub intent: String,
    pub signer: String,
    #[serde(rename = "isGuest")]
    is_guest: bool,
    origin: String,
}

impl SignedLoginMeta {
    pub fn new(is_guest: bool, origin: Uri) -> Self {
        let origin = origin.into_parts();

        Self {
            intent: "dcl:explorer:comms-handshake".to_owned(),
            signer: "dcl:explorer".to_owned(),
            is_guest,
            origin: format!("{}://{}", origin.scheme.unwrap(), origin.authority.unwrap()),
        }
    }
}

pub enum SignedLoginPollStatus {
    Pending,
    Complete(SignedLoginResponse),
    Error(anyhow::Error),
}

pub struct SignedLogin {
    http_requester: HttpRequester,
}

impl SignedLogin {
    pub fn new(uri: Uri, ephemeral_auth_chain: EphemeralAuthChain, meta: SignedLoginMeta) -> Self {
        let unix_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();

        let meta = serde_json::to_string(&meta).unwrap();
        let payload = format!("post:{}:{}:{}", uri.path(), unix_time, meta).to_lowercase();

        // TODO: should this block_on be async? the ephemeral wallet is sync
        let signature = futures_lite::future::block_on(
            ephemeral_auth_chain
                .ephemeral_wallet()
                .sign_message(&payload),
        )
        .expect("signature by ephemeral should always work");

        let mut chain = ephemeral_auth_chain.auth_chain().clone();
        chain.add_signed_entity(payload, signature);

        let mut headers = Vec::from_iter(
            chain
                .headers()
                .map(|(key, value)| format!("{}: {}", key, value)),
        );

        headers.push(format!("x-identity-timestamp: {unix_time}"));
        headers.push(format!("x-identity-metadata: {meta}"));

        let mut http_requester = HttpRequester::new(None);
        http_requester.send_request(RequestOption::new(
            0,
            uri.to_string(),
            Method::POST,
            ResponseType::AsJson,
            None,
            Some(headers),
        ));

        SignedLogin { http_requester }
    }

    pub fn poll(&mut self) -> SignedLoginPollStatus {
        if let Some(response) = self.http_requester.poll() {
            match response {
                Ok(response) => {
                    if let Ok(ResponseEnum::Json(Ok(json))) = response.response_data {
                        if let Ok(response) = serde_json::from_value::<SignedLoginResponse>(json) {
                            return SignedLoginPollStatus::Complete(response);
                        }
                    }
                }
                Err(e) => return SignedLoginPollStatus::Error(anyhow::anyhow!(e.error_message)),
            }

            return SignedLoginPollStatus::Error(anyhow::anyhow!("unknown error"));
        }
        SignedLoginPollStatus::Pending
    }
}
