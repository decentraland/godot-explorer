use std::{cell::RefCell, collections::HashMap, rc::Rc, time::Duration};

use deno_core::{error::AnyError, op, Op, OpDecl, OpState};
use http::HeaderValue;
use reqwest::Response;
use serde::Serialize;

use crate::tools::network_inspector::{
    NetworkInspectEvent, NetworkInspectRequestPayload, NetworkInspectResponsePayload,
    NetworkInspectorId, NetworkInspectorSender,
};

mod signed_fetch;

pub fn ops() -> Vec<OpDecl> {
    vec![
        op_fetch_custom::DECL,
        op_fetch_consume_text::DECL,
        op_fetch_consume_bytes::DECL,
        signed_fetch::op_signed_fetch_headers::DECL,
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

    network_inspector_id: u32,
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
    _redirect: String, // TODO: unimplemented
    timeout: u32,
) -> Result<FetchResponse, AnyError> {
    let maybe_network_inspector_sender = op_state
        .borrow()
        .try_borrow::<NetworkInspectorSender>()
        .cloned();
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

    // match redirect.as_str() {
    //     "follow" => {}
    //     "error" => {}
    //     "manual" => {}
    //     _ => {}
    // };

    // Inspect Network
    let mut network_inspector_id = NetworkInspectorId::INVALID;
    if let Some(network_inspector_sender) = maybe_network_inspector_sender.as_ref() {
        let (inspect_event_id, inspect_event) =
            NetworkInspectEvent::new_request(NetworkInspectRequestPayload {
                url: url.clone(),
                method: method.clone(),
                body: if has_body {
                    Some(body_data.clone().as_bytes().to_vec())
                } else {
                    None
                },
                headers: Some(
                    headers
                        .iter()
                        .map(|(key, value)| {
                            (key.to_string(), value.to_str().unwrap_or("").to_string())
                        })
                        .collect(),
                ),
            });
        network_inspector_id = inspect_event_id;
        if let Err(err) = network_inspector_sender.try_send(inspect_event) {
            tracing::error!("Error sending inspect event: {}", err);
        }
    }

    let mut request = client
        .request(method.clone(), url.clone())
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
            drop(state);

            // Inspect Network
            if network_inspector_id.is_valid() {
                if let Some(network_inspector_sender) = maybe_network_inspector_sender.as_ref() {
                    let inspect_event = NetworkInspectEvent::new_partial_response(
                        network_inspector_id,
                        Ok(NetworkInspectResponsePayload {
                            status_code: status,
                            headers: Some(headers.clone()),
                        }),
                    );
                    if let Err(err) = network_inspector_sender.try_send(inspect_event) {
                        tracing::error!("Error sending inspect event: {}", err);
                    }
                }
            }

            let js_response = FetchResponse {
                ok: true,
                _internal_req_id: req_id,
                headers,
                redirected: status.is_redirection(),
                status: status.as_u16(),
                status_text: status.to_string(),
                _type: "basic".into(), // TODO
                url: url.clone(),
                network_inspector_id: network_inspector_id.to_u32(),
            };

            Ok(js_response)
        }
        Err(err) => {
            drop(state);

            // Inspect Network
            if network_inspector_id.is_valid() {
                if let Some(network_inspector_sender) = maybe_network_inspector_sender.as_ref() {
                    let inspect_event = NetworkInspectEvent::new_partial_response(
                        network_inspector_id,
                        Err(err.to_string()),
                    );
                    if let Err(err) = network_inspector_sender.try_send(inspect_event) {
                        tracing::error!("Error sending inspect event: {}", err);
                    }
                }
            }

            Ok(FetchResponse {
                _internal_req_id: req_id,
                headers: HashMap::new(),
                ok: false,
                redirected: false,
                status: 0,
                status_text: err.to_string(),
                _type: "error".into(),
                url: url.clone(),
                network_inspector_id: network_inspector_id.to_u32(),
            })
        }
    }
}

#[op]
async fn op_fetch_consume_text(
    op_state: Rc<RefCell<OpState>>,
    req_id: u32,
    inspector_network_req_id: u32,
) -> Result<String, AnyError> {
    let inspector_network_req_id = NetworkInspectorId::from_u32(inspector_network_req_id);
    let maybe_network_inspector_sender = if inspector_network_req_id.is_valid() {
        op_state
            .borrow()
            .try_borrow::<NetworkInspectorSender>()
            .cloned()
    } else {
        None
    };

    let response = {
        let mut state = op_state.borrow_mut();
        let fetch_request = state.borrow_mut::<FetchRequestsState>();
        let current_request = fetch_request.requests.get_mut(&req_id).unwrap();
        current_request.response.take()
    };

    if let Some(response) = response {
        match response.text().await {
            Ok(response) => {
                if let Some(network_inspector_sender) = maybe_network_inspector_sender.as_ref() {
                    let inspect_event = NetworkInspectEvent::new_body_response(
                        inspector_network_req_id,
                        Ok(Some(response.clone())),
                    );
                    if let Err(err) = network_inspector_sender.try_send(inspect_event) {
                        tracing::error!("Error sending inspect event: {}", err);
                    }
                }

                return Ok(response);
            }
            Err(err) => {
                if let Some(network_inspector_sender) = maybe_network_inspector_sender.as_ref() {
                    let inspect_event = NetworkInspectEvent::new_body_response(
                        inspector_network_req_id,
                        Err(err.to_string()),
                    );
                    if let Err(err) = network_inspector_sender.try_send(inspect_event) {
                        tracing::error!("Error sending inspect event: {}", err);
                    }
                }
            }
        }
    }

    if let Some(network_inspector_sender) = maybe_network_inspector_sender.as_ref() {
        let inspect_event = NetworkInspectEvent::new_body_response(
            inspector_network_req_id,
            Err("couldn't get response".into()),
        );
        if let Err(err) = network_inspector_sender.try_send(inspect_event) {
            tracing::error!("Error sending inspect event: {}", err);
        }
    }
    Err(anyhow::Error::msg("couldn't get response"))
}
#[op]
async fn op_fetch_consume_bytes(
    op_state: Rc<RefCell<OpState>>,
    req_id: u32,
    inspector_network_req_id: u32,
) -> Result<bytes::Bytes, AnyError> {
    let inspector_network_req_id = NetworkInspectorId::from_u32(inspector_network_req_id);
    let maybe_network_inspector_sender = if inspector_network_req_id.is_valid() {
        op_state
            .borrow()
            .try_borrow::<NetworkInspectorSender>()
            .cloned()
    } else {
        None
    };

    let response = {
        let mut state = op_state.borrow_mut();
        let fetch_request = state.borrow_mut::<FetchRequestsState>();
        let current_request = fetch_request.requests.get_mut(&req_id).unwrap();

        current_request.response.take()
    };

    if let Some(response) = response {
        match response.bytes().await {
            Ok(response) => {
                if let Some(network_inspector_sender) = maybe_network_inspector_sender.as_ref() {
                    let inspect_event =
                        NetworkInspectEvent::new_body_response(inspector_network_req_id, Ok(None));
                    if let Err(err) = network_inspector_sender.try_send(inspect_event) {
                        tracing::error!("Error sending inspect event: {}", err);
                    }
                }

                return Ok(response);
            }
            Err(err) => {
                if let Some(network_inspector_sender) = maybe_network_inspector_sender.as_ref() {
                    let inspect_event = NetworkInspectEvent::new_body_response(
                        inspector_network_req_id,
                        Err(err.to_string()),
                    );
                    if let Err(err) = network_inspector_sender.try_send(inspect_event) {
                        tracing::error!("Error sending inspect event: {}", err);
                    }
                }
            }
        }
    }

    if let Some(network_inspector_sender) = maybe_network_inspector_sender.as_ref() {
        let inspect_event = NetworkInspectEvent::new_body_response(
            inspector_network_req_id,
            Err("couldn't get response".into()),
        );
        if let Err(err) = network_inspector_sender.try_send(inspect_event) {
            tracing::error!("Error sending inspect event: {}", err);
        }
    }
    Err(anyhow::Error::msg("couldn't get response"))
}
