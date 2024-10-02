use std::{cell::RefCell, collections::HashMap, rc::Rc, time::Duration};

use deno_core::{error::AnyError, op2, OpDecl, OpState};
use http::HeaderValue;
use reqwest::Response;
use serde::Serialize;

mod signed_fetch;

pub fn ops() -> Vec<OpDecl> {
    vec![
        op_fetch_custom(),
        op_fetch_consume_json(),
        op_fetch_consume_text(),
        op_fetch_consume_bytes(),
        signed_fetch::op_signed_fetch_headers(),
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

#[op2(async)]
#[serde]
async fn op_fetch_custom(
    op_state: Rc<RefCell<OpState>>,
    #[string] method: String,
    #[string] url: String,
    #[serde] headers: HashMap<String, String>,
    has_body: bool,
    #[string] body_data: String,
    #[string] _redirect: String, // TODO: unimplemented
    timeout: u32,
) -> Result<FetchResponse, AnyError> {
    let has_fetch_state = op_state.borrow().has::<FetchRequestsState>();
    if !has_fetch_state {
        op_state
            .borrow_mut()
            .put::<FetchRequestsState>(FetchRequestsState::new());
    }

    let (req_id, client) = {
        let mut state = op_state.borrow_mut();
        let fetch_request = state.borrow_mut::<FetchRequestsState>();
        let client = fetch_request.client.clone();
        fetch_request.counter += 1;

        let req_id = fetch_request.counter;
        fetch_request
            .requests
            .insert(req_id, FetchRequest { response: None });
        (req_id, client)
    };

    let method = match method.as_str() {
        "GET" => http::Method::GET,
        "POST" => http::Method::POST,
        "PUT" => http::Method::PUT,
        "DELETE" => http::Method::DELETE,
        "HEAD" => http::Method::HEAD,
        "OPTIONS" => http::Method::OPTIONS,
        "CONNECT" => http::Method::CONNECT,
        "PATCH" => http::Method::PATCH,
        "TRACE" => http::Method::TRACE,
        _ => http::Method::GET,
    };

    let mut headers = headers
        .into_iter()
        .map(|(key, value)| (key.parse().unwrap(), value.parse().unwrap()))
        .collect::<reqwest::header::HeaderMap>();

    headers.append("User-Agent", HeaderValue::from_static("DCLExplorer/0.1"));
    headers.append(
        "Origin",
        HeaderValue::from_static("https://decentraland.org"),
    );

    let mut request = client
        .request(method, url.clone())
        .headers(headers)
        .timeout(Duration::from_secs(timeout as u64));

    // match redirect.as_str() {
    //     "follow" => {}
    //     "error" => {}
    //     "manual" => {}
    //     _ => {}
    // };

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

#[op2(async)]
#[serde]
async fn op_fetch_consume_json(
    op_state: Rc<RefCell<OpState>>,
    req_id: u32,
) -> Result<serde_json::Value, AnyError> {
    let response = {
        let mut state = op_state.borrow_mut();
        let fetch_request = state.borrow_mut::<FetchRequestsState>();
        let current_request = fetch_request.requests.get_mut(&req_id).unwrap();
        current_request.response.take()
    };

    if let Some(response) = response {
        return Ok(response.json::<serde_json::Value>().await?);
    }

    Err(anyhow::Error::msg("couldn't get response"))
}

#[op2(async)]
#[string]
async fn op_fetch_consume_text(
    op_state: Rc<RefCell<OpState>>,
    req_id: u32,
) -> Result<String, AnyError> {
    let response = {
        let mut state = op_state.borrow_mut();
        let fetch_request = state.borrow_mut::<FetchRequestsState>();
        let current_request = fetch_request.requests.get_mut(&req_id).unwrap();
        current_request.response.take()
    };

    if let Some(response) = response {
        return Ok(response.text().await?);
    }

    Err(anyhow::Error::msg("couldn't get response"))
}
#[op2(async)]
#[serde]
async fn op_fetch_consume_bytes(
    op_state: Rc<RefCell<OpState>>,
    req_id: u32,
) -> Result<bytes::Bytes, AnyError> {
    let response = {
        let mut state = op_state.borrow_mut();
        let fetch_request = state.borrow_mut::<FetchRequestsState>();
        let current_request = fetch_request.requests.get_mut(&req_id).unwrap();
        current_request.response.take()
    };

    if let Some(response) = response {
        return Ok(response.bytes().await?);
    }

    Err(anyhow::Error::msg("couldn't get response"))
}
