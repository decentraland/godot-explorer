use std::{
    collections::{HashMap, HashSet},
    sync::atomic::AtomicBool,
};

use godot::prelude::*;

pub type NetworkInspectorSender = tokio::sync::mpsc::Sender<NetworkInspectEvent>;

#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
pub struct NetworkInspectorId(u32);

impl Default for NetworkInspectorId {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkInspectorId {
    pub const INVALID: NetworkInspectorId = NetworkInspectorId(0);

    pub fn new() -> Self {
        Self(
            NETWORK_INSPECTED_REQUEST_ID_MONOTONIC_COUNTER
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        )
    }

    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }

    pub fn from_u32(id: u32) -> Self {
        Self(id)
    }

    pub fn to_u32(&self) -> u32 {
        self.0
    }
}

struct NetworkInspectedRequest {
    requested_at: f64,
    request: NetworkInspectRequestPayload,

    response_received_at: Option<f64>,
    response: Result<NetworkInspectResponsePayload, String>,

    response_payload_received_at: Option<f64>,
    response_payload: Result<Option<String>, String>,
}

static NETWORK_INSPECTED_REQUEST_ID_MONOTONIC_COUNTER: once_cell::sync::Lazy<
    std::sync::atomic::AtomicU32,
> = once_cell::sync::Lazy::new(|| std::sync::atomic::AtomicU32::new(0));

pub struct NetworkInspectEvent {
    id: NetworkInspectorId,
    payload: NetworkInspectPayload,
}

pub struct NetworkInspectRequestPayload {
    pub url: String,
    pub method: http::Method,
    pub body: Option<Vec<u8>>,
    pub headers: Option<HashMap<String, String>>,
}

pub struct NetworkInspectResponsePayload {
    pub status_code: http::StatusCode,
    pub headers: Option<HashMap<String, String>>,
}

pub enum NetworkInspectPayload {
    Request(NetworkInspectRequestPayload),
    PartialResponse(Result<NetworkInspectResponsePayload, String>),
    BodyResponse(Result<Option<String>, String>),
    FullResponse(Result<(NetworkInspectResponsePayload, Option<String>), String>),
}

pub static NETWORK_INSPECTOR_ENABLE: AtomicBool = AtomicBool::new(false);

impl NetworkInspectEvent {
    pub fn new_request(request: NetworkInspectRequestPayload) -> (NetworkInspectorId, Self) {
        let id = NetworkInspectorId::new();
        (
            id,
            NetworkInspectEvent {
                id,
                payload: NetworkInspectPayload::Request(request),
            },
        )
    }

    pub fn new_partial_response(
        id: NetworkInspectorId,
        response: Result<NetworkInspectResponsePayload, String>,
    ) -> Self {
        NetworkInspectEvent {
            id,
            payload: NetworkInspectPayload::PartialResponse(response),
        }
    }

    pub fn new_body_response(
        id: NetworkInspectorId,
        response: Result<Option<String>, String>,
    ) -> Self {
        NetworkInspectEvent {
            id,
            payload: NetworkInspectPayload::BodyResponse(response),
        }
    }

    pub fn new_full_response(
        id: NetworkInspectorId,
        response: Result<(NetworkInspectResponsePayload, Option<String>), String>,
    ) -> Self {
        NetworkInspectEvent {
            id,
            payload: NetworkInspectPayload::FullResponse(response),
        }
    }
}

#[derive(GodotClass)]
#[class(base=Node)]
pub struct NetworkInspector {
    requests: HashMap<NetworkInspectorId, NetworkInspectedRequest>,
    receiver: tokio::sync::mpsc::Receiver<NetworkInspectEvent>,
    sender: tokio::sync::mpsc::Sender<NetworkInspectEvent>,
    #[base]
    _base: Base<Node>,
}

#[godot_api]
impl NetworkInspector {
    #[func]
    pub fn set_is_active(&mut self, value: bool) {
        NETWORK_INSPECTOR_ENABLE.store(value, std::sync::atomic::Ordering::Relaxed);
    }

    #[signal]
    pub fn request_changed(&self, id: u32) {}

    #[func]
    pub fn get_request(&self, id: u32) -> Dictionary {
        let mut dict = Dictionary::new();
        if let Some(request) = self.requests.get(&NetworkInspectorId(id)) {
            // dict.insert("id", request.id);
            dict.insert("requested_at", request.requested_at);
            dict.insert(
                "response_received_at",
                request.response_received_at.unwrap_or(0.0),
            );
            dict.insert(
                "response_payload_received_at",
                request.response_payload_received_at.unwrap_or(0.0),
            );
            dict.insert("url", request.request.url.as_str());
            dict.insert("method", request.request.method.as_str());

            let headers = {
                let mut dict = Dictionary::new();
                if let Some(headers) = &request.request.headers {
                    for (key, value) in headers.iter() {
                        dict.insert(key.as_str().to_string(), value.as_str().to_string());
                    }
                }
                dict
            };
            dict.insert("headers", headers);
        }
        dict
    }
}

#[godot_api]
impl INode for NetworkInspector {
    fn init(_base: Base<Node>) -> Self {
        let (sender, receiver) = tokio::sync::mpsc::channel(10);
        NetworkInspector {
            requests: HashMap::new(),
            receiver,
            sender,
            _base,
        }
    }

    fn process(&mut self, _dt: f64) {
        let mut request_changed = HashSet::new();
        while let Ok(event) = self.receiver.try_recv() {
            request_changed.insert(event.id.0);
            match event.payload {
                NetworkInspectPayload::Request(request) => {
                    self.requests.insert(
                        event.id,
                        NetworkInspectedRequest {
                            requested_at: 0.0,
                            request,
                            response_received_at: None,
                            response: Err("No response received".to_string()),
                            response_payload_received_at: None,
                            response_payload: Err("No response payload received".to_string()),
                        },
                    );
                }
                NetworkInspectPayload::PartialResponse(response) => {
                    if let Some(request) = self.requests.get_mut(&event.id) {
                        request.response_received_at = Some(0.0);
                        request.response = response;
                    }
                }
                NetworkInspectPayload::BodyResponse(response) => {
                    if let Some(request) = self.requests.get_mut(&event.id) {
                        request.response_payload_received_at = Some(0.0);
                        request.response_payload = response;
                    }
                }
                NetworkInspectPayload::FullResponse(response) => {
                    if let Some(request) = self.requests.get_mut(&event.id) {
                        request.response_received_at = Some(0.0);
                        request.response_payload_received_at = Some(0.0);

                        match response {
                            Ok((response, body)) => {
                                request.response = Ok(response);
                                request.response_payload = Ok(body);
                            }
                            Err(err) => {
                                request.response = Err(err.clone());
                                request.response_payload = Err(err);
                            }
                        }
                    }
                }
            }
        }

        for id in request_changed {
            self._base.call_deferred(
                "emit_signal".into(),
                &["request_changed".to_variant(), id.to_variant()],
            );
        }
    }
}

impl NetworkInspector {
    pub fn get_sender(&self) -> tokio::sync::mpsc::Sender<NetworkInspectEvent> {
        self.sender.clone()
    }
}
