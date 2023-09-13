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

struct FetchRequestsState {
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

impl FetchRequestsState {
    fn new() -> Self {
        let client = reqwest::ClientBuilder::new()
            .timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .expect("fail trying to build reqwest client");

        FetchRequestsState {
            counter: 0,
            client,
            requests: HashMap::new(),
        }
    }
}

#[op]
async fn op_fetch_custom(
    op_state: Rc<RefCell<OpState>>,
    method: String,
    url: String,
    headers: HashMap<String, String>,
    has_body: bool,
    body_data: String,
    _redirect: String, // TODO
    timeout: u32,
) -> Result<FetchResponse, AnyError> {
    let has_fetch_state = op_state.borrow().has::<FetchRequestsState>();
    if !has_fetch_state {
        op_state
            .borrow_mut()
            .put::<FetchRequestsState>(FetchRequestsState::new());
    }

    let mut state = op_state.borrow_mut();
    let fetch_request = state.borrow_mut::<FetchRequestsState>();
    let client = fetch_request.client.clone();
    fetch_request.counter += 1;

    let req_id = fetch_request.counter;
    fetch_request
        .requests
        .insert(req_id, FetchRequest { response: None });
    drop(state);

    let method = match method {
        _ => http::Method::GET,
    };

    let headers = headers
        .into_iter()
        .map(|(key, value)| (key.parse().unwrap(), value.parse().unwrap()))
        .collect::<reqwest::header::HeaderMap>();

    let mut request = client
        .request(method, url.clone())
        .headers(headers)
        .timeout(Duration::from_secs(timeout as u64));

    if has_body {
        request = request.body(body_data);
    }

    let result = request.send().await;
    let mut state = op_state.borrow_mut();
    let fetch_request = state.borrow_mut::<FetchRequestsState>();
    let current_request = fetch_request.requests.get_mut(&req_id).unwrap();

    match result {
        Ok(response) => {
            let status = response.status();
            let headers =
                HashMap::from_iter(response.headers().iter().map(|(key, value)| {
                    (key.to_string(), value.to_str().unwrap_or("").to_string())
                }));

            current_request.response = Some(response);
            let js_response = FetchResponse {
                ok: true,
                _internal_req_id: req_id,
                headers,
                redirected: status.is_redirection(),
                status: status.as_u16(),
                status_text: status.to_string(),
                _type: "basic".into(), // TODO
                url: url.clone(),
            };
            Ok(js_response)
        }
        Err(err) => Ok(FetchResponse {
            _internal_req_id: req_id,
            headers: HashMap::new(),
            ok: false,
            redirected: false,
            status: 0,
            status_text: err.to_string(),
            _type: "error".into(),
            url: url.clone(),
        }),
    }
}

#[op]
async fn op_fetch_consume_json(
    op_state: Rc<RefCell<OpState>>,
    req_id: u32,
) -> Result<serde_json::Value, AnyError> {
    let mut state = op_state.borrow_mut();
    let fetch_request = state.borrow_mut::<FetchRequestsState>();
    let current_request = fetch_request.requests.get_mut(&req_id).unwrap();
    let response = current_request.response.take();
    drop(state);

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
    let mut state = op_state.borrow_mut();
    let fetch_request = state.borrow_mut::<FetchRequestsState>();
    let current_request = fetch_request.requests.get_mut(&req_id).unwrap();
    let response = current_request.response.take();
    drop(state);

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
    let mut state = op_state.borrow_mut();
    let fetch_request = state.borrow_mut::<FetchRequestsState>();
    let current_request = fetch_request.requests.get_mut(&req_id).unwrap();
    let response = current_request.response.take();
    drop(state);

    if let Some(response) = response {
        return Ok(response.bytes().await?);
    }

    Err(anyhow::Error::msg("couldn't get response"))
}
