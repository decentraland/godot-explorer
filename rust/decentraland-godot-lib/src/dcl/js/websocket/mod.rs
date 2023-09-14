use std::{cell::RefCell, collections::HashMap, rc::Rc, time::Duration};

use deno_core::{error::AnyError, op, Op, OpDecl, OpState};
use reqwest::Response;
use serde::Serialize;

pub fn ops() -> Vec<OpDecl> {
    vec![
        op_fetch_custom::DECL,
        op_fetch_consume_json::DECL,
        op_fetch_consume_text::DECL,
        op_fetch_consume_bytes::DECL,
    ]
}

struct FetchRequest {
    response: Option<Response>,
}

struct WsState {
    counter: u32,
    client: reqwest::Client,
    requests: HashMap<u32, FetchRequest>,
}

#[derive(Serialize)]
struct FetchResponse {
    _internal_req_id: u32,
    headers: HashMap<String, String>,
    ok: bool,
    redirected: bool,
    status: u16,
    #[serde(rename = "statusText")]
    status_text: String,
    #[serde(rename = "type")]
    _type: String,
    url: String,
}

impl WsState {
    fn new() -> Self {
        let client = reqwest::ClientBuilder::new()
            .timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .expect("fail trying to build reqwest client");

        WsState {
            counter: 0,
            client,
            requests: HashMap::new(),
        }
    }
}

#[op]
async fn op_ws_create(
    op_state: Rc<RefCell<OpState>>,
    url: String,
    protocols: Vec<String>,
) -> Result<u32, AnyError> {
    let has_ws_state = op_state.borrow().has::<WsState>();
    if !has_ws_state {
        op_state.borrow_mut().put::<WsState>(WsState::new());
    }

    let (req_id, client) = {
        let mut state = op_state.borrow_mut();
        let fetch_request = state.borrow_mut::<WsState>();
        // let client = fetch_request.client.clone();
        fetch_request.counter += 1;

        // let req_id = fetch_request.counter;
        // fetch_request
        //     .requests
        //     .insert(req_id, FetchRequest { response: None });
        (req_id, client)
    };

    Ok(req_id)
}

#[op]
async fn op_fetch_consume_json(
    op_state: Rc<RefCell<OpState>>,
    req_id: u32,
) -> Result<serde_json::Value, AnyError> {
    let response = {
        let mut state = op_state.borrow_mut();
        let fetch_request = state.borrow_mut::<WsState>();
        let current_request = fetch_request.requests.get_mut(&req_id).unwrap();
        current_request.response.take()
    };

    if let Some(response) = response {
        return Ok(response.json::<serde_json::Value>().await?);
    }

    Err(anyhow::Error::msg("couldn't get response"))
}

#[op]
async fn op_fetch_consume_text(
    op_state: Rc<RefCell<OpState>>,
    req_id: u32,
) -> Result<String, AnyError> {
    let response = {
        let mut state = op_state.borrow_mut();
        let fetch_request = state.borrow_mut::<WsState>();
        let current_request = fetch_request.requests.get_mut(&req_id).unwrap();
        current_request.response.take()
    };

    if let Some(response) = response {
        return Ok(response.text().await?);
    }

    Err(anyhow::Error::msg("couldn't get response"))
}
#[op]
async fn op_fetch_consume_bytes(
    op_state: Rc<RefCell<OpState>>,
    req_id: u32,
) -> Result<bytes::Bytes, AnyError> {
    let response = {
        let mut state = op_state.borrow_mut();
        let fetch_request = state.borrow_mut::<WsState>();
        let current_request = fetch_request.requests.get_mut(&req_id).unwrap();
        current_request.response.take()
    };

    if let Some(response) = response {
        return Ok(response.bytes().await?);
    }

    Err(anyhow::Error::msg("couldn't get response"))
}
