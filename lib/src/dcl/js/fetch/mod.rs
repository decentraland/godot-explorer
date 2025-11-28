use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::Arc, time::Duration};

use deno_core::{error::AnyError, op2, OpDecl, OpState};
use http::HeaderValue;
use reqwest::Response;
use serde::Serialize;
use tokio::sync::Semaphore;

use crate::{
    realm::scene_definition::SceneEntityDefinition,
    tools::network_inspector::{
        NetworkInspectEvent, NetworkInspectRequestPayload, NetworkInspectResponsePayload,
        NetworkInspectorId, NetworkInspectorSender,
    },
};

mod signed_fetch;

pub fn ops() -> Vec<OpDecl> {
    vec![
        op_fetch_custom(),
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

struct FetchRequestLimiter {
    sem: Arc<Semaphore>,
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

#[op2(async)]
#[serde]
#[allow(clippy::too_many_arguments)]
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

    let (req_id, client, semaphore) = {
        let mut state = op_state.borrow_mut();

        let semaphore = if let Some(value) = state.try_borrow::<FetchRequestLimiter>() {
            value.sem.clone()
        } else {
            state.put(FetchRequestLimiter {
                sem: Arc::new(Semaphore::new(2)),
            });
            state.borrow::<FetchRequestLimiter>().sem.clone()
        };

        let fetch_request = state.borrow_mut::<FetchRequestsState>();
        let client = fetch_request.client.clone();
        fetch_request.counter += 1;

        let req_id = fetch_request.counter;
        fetch_request
            .requests
            .insert(req_id, FetchRequest { response: None });
        (req_id, client, semaphore)
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
        let requester = {
            let state = op_state.borrow();
            let scene_entity_definition = state.borrow::<Arc<SceneEntityDefinition>>();
            format!(
                "{} @ {},{}",
                scene_entity_definition.get_title(),
                scene_entity_definition.get_base_parcel().x,
                scene_entity_definition.get_base_parcel().y
            )
        };

        tracing::debug!("fetch request: {} by {}", url, requester);

        let (inspect_event_id, inspect_event) =
            NetworkInspectEvent::new_request(NetworkInspectRequestPayload {
                requester,
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

    let _permit = match semaphore.acquire_owned().await {
        Ok(permit) => permit,
        Err(err) => {
            tracing::error!("Error acquiring semaphore: {}", err);
            return Err(anyhow::Error::msg("Error acquiring semaphore"));
        }
    };

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

            let status_code: u16 = status.as_u16();

            let js_response = FetchResponse {
                ok: (200..300).contains(&status_code),
                _internal_req_id: req_id,
                headers,
                redirected: status.is_redirection(),
                status: status_code,
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

#[op2(async)]
#[string]
async fn op_fetch_consume_text(
    op_state: Rc<RefCell<OpState>>,
    req_id: u32,
    inspector_network_req_id: u32,
) -> Result<String, AnyError> {
    let inspector_network_req_id = NetworkInspectorId::from_u32(inspector_network_req_id);
    let maybe_network_inspector_sender = if inspector_network_req_id.is_valid() {
        let url = {
            let state = op_state.borrow();
            let fetch_request = state.borrow::<FetchRequestsState>();
            fetch_request
                .requests
                .get(&req_id)
                .unwrap()
                .response
                .as_ref()
                .unwrap()
                .url()
                .to_string()
        };
        let requester = {
            let state = op_state.borrow();
            let scene_entity_definition = state.borrow::<Arc<SceneEntityDefinition>>();
            format!(
                "{} @ {},{}",
                scene_entity_definition.get_title(),
                scene_entity_definition.get_base_parcel().x,
                scene_entity_definition.get_base_parcel().y
            )
        };
        tracing::debug!("op_fetch_consume_text request: {} by {}", url, requester);

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

                tracing::debug!("op_fetch_consume_text response: {}", response);
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
#[op2(async)]
#[serde]
async fn op_fetch_consume_bytes(
    op_state: Rc<RefCell<OpState>>,
    req_id: u32,
    inspector_network_req_id: u32,
) -> Result<bytes::Bytes, AnyError> {
    let inspector_network_req_id = NetworkInspectorId::from_u32(inspector_network_req_id);
    let maybe_network_inspector_sender = if inspector_network_req_id.is_valid() {
        let url = {
            let state = op_state.borrow();
            let fetch_request = state.borrow::<FetchRequestsState>();
            fetch_request
                .requests
                .get(&req_id)
                .unwrap()
                .response
                .as_ref()
                .unwrap()
                .url()
                .to_string()
        };
        let requester = {
            let state = op_state.borrow();
            let scene_entity_definition = state.borrow::<Arc<SceneEntityDefinition>>();
            format!(
                "{} @ {},{}",
                scene_entity_definition.get_title(),
                scene_entity_definition.get_base_parcel().x,
                scene_entity_definition.get_base_parcel().y
            )
        };
        tracing::debug!("op_fetch_consume_bytes request: {} by {}", url, requester);
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
                    let body = String::from_utf8_lossy(response.as_ref()).to_string();
                    let inspect_event = NetworkInspectEvent::new_body_response(
                        inspector_network_req_id,
                        Ok(Some(body)),
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
