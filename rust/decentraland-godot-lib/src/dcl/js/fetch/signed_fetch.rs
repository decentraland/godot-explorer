use std::{cell::RefCell, rc::Rc};

use crate::auth::wallet::{sign_request, Wallet};
use deno_core::{error::AnyError, op, OpState};
use http::Uri;
use serde::Serialize;

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SignedFetchMetaRealm {
    domain: Option<String>,
    catalyst_name: Option<String>,
    layer: Option<String>,
    lighthouse_version: Option<String>,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SignedFetchMeta {
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
    state: Rc<RefCell<OpState>>,
    uri: String,
    method: Option<String>,
) -> Result<Vec<(String, String)>, AnyError> {
    let wallet = state.borrow().borrow::<Wallet>().clone();

    let meta = SignedFetchMeta {
        origin: Some("localhost".to_owned()),
        is_guest: Some(true),
        ..Default::default()
    };

    let headers = sign_request(
        method.as_deref().unwrap_or("get"),
        &Uri::try_from(uri)?,
        &wallet,
        meta,
    )
    .await;

    Ok(headers)
}
