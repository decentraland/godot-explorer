use std::{cell::RefCell, rc::Rc};

use deno_core::{error::AnyError, op, ByteString, JsBuffer, Op, OpDecl, OpState, ResourceId};
use deno_fetch::FetchPermissions;
use deno_web::TimersPermission;
use serde::{Deserialize, Serialize};

use crate::http_request::http_requester::HttpRequester;

// TODO: fetch

// we have to provide fetch perm structs even though we don't use them
pub struct FP;
impl FetchPermissions for FP {
    fn check_net_url(&mut self, _: &deno_core::url::Url, _: &str) -> Result<(), AnyError> {
        panic!();
    }

    fn check_read(&mut self, _: &std::path::Path, _: &str) -> Result<(), AnyError> {
        panic!();
    }
}

pub struct TP;
impl TimersPermission for TP {
    fn allow_hrtime(&mut self) -> bool {
        false
    }

    fn check_unstable(&self, _: &OpState, _: &'static str) {
        panic!("i don't know what this is for")
    }
}

pub fn ops() -> Vec<OpDecl> {
    vec![
        op_fetch::DECL,
        op_fetch_send::DECL,
        op_fetch_custom_client::DECL,
    ]
}

#[op]
pub fn op_fetch(
    state: &mut OpState,
    method: ByteString,
    url: String,
    headers: Vec<(ByteString, ByteString)>,
    client_rid: Option<u32>,
    has_body: bool,
    body_length: Option<u64>,
    data: Option<JsBuffer>,
) -> Result<(), AnyError> {
    return Err(anyhow::Error::msg("not implemented"));
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchResponse {
    status: u16,
    status_text: String,
    headers: Vec<(ByteString, ByteString)>,
    url: String,
    response_rid: ResourceId,
    content_length: Option<u64>,
}

#[op]
pub fn op_fetch_send(
    state: Rc<RefCell<OpState>>,
    rid: ResourceId,
) -> Result<FetchResponse, AnyError> {
    return Err(anyhow::Error::msg("not implemented"));
}

#[derive(Deserialize, Default, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Proxy {
    pub url: String,
    pub basic_auth: Option<BasicAuth>,
}

#[derive(Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct BasicAuth {
    pub username: String,
    pub password: String,
}

// copy out the args struct so we can access the members...
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CreateHttpClientOptions {
    ca_certs: Vec<String>,
    proxy: Option<Proxy>,
    cert_chain: Option<String>,
    private_key: Option<String>,
}

#[op]
pub fn op_fetch_custom_client(
    state: &mut OpState,
    args: CreateHttpClientOptions,
) -> Result<ResourceId, AnyError> {
    return Err(anyhow::Error::msg("not implemented"));
}

#[op]
pub async fn op_fetch_custom(state: Rc<RefCell<OpState>>) -> Result<(), AnyError> {
    return Err(anyhow::Error::msg("not implemented"));
}
