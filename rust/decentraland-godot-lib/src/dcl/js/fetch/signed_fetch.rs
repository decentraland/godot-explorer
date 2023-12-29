use std::{cell::RefCell, rc::Rc};

use deno_core::{
    anyhow::{self},
    error::AnyError,
    op, OpState,
};
use http::Uri;

use crate::auth::{ephemeral_auth_chain::EphemeralAuthChain, wallet::sign_request};

use serde::Serialize;

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct SignedFetchMetaRealm {
    domain: Option<String>,
    catalyst_name: Option<String>,
    layer: Option<String>,
    lighthouse_version: Option<String>,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct SignedFetchMeta {
    origin: Option<String>,
    scene_id: Option<String>,
    parcel: Option<String>,
    tld: Option<String>,
    network: Option<String>,
    is_guest: Option<bool>,
    realm: SignedFetchMetaRealm,
}

#[op]
pub async fn op_signed_fetch_headers(
    op_state: Rc<RefCell<OpState>>,
    uri: String,
    method: Option<String>,
) -> Result<Vec<(String, String)>, AnyError> {
    let wallet = op_state
        .borrow()
        .borrow::<Option<EphemeralAuthChain>>()
        .clone();

    if let Some(ephemeral_wallet) = wallet {
        let meta = SignedFetchMeta {
            origin: Some("localhost".to_owned()),
            is_guest: Some(true),
            ..Default::default()
        };

        let headers = sign_request(
            method.as_deref().unwrap_or("get"),
            &Uri::try_from(uri)?,
            &ephemeral_wallet,
            meta,
        )
        .await;

        Ok(headers)
    } else {
        Err(anyhow::Error::msg("There is no wallet to sign headers."))
    }
}
