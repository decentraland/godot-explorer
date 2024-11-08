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

#[derive(GodotClass)]
#[class(base=RefCounted)]
struct NetworkInspectedRequest {
    #[var]
    requested_by: GString,

    // REQUEST
    #[var]
    requested_at: u32,

    #[var]
    url: GString,
    #[var]
    method: GString,

    #[var]
    request_headers: Dictionary,

    // RESPONSE
    #[var]
    response_received_at: u32,
    #[var]
    response_received: bool,
    #[var]
    response_ok: bool,
    #[var]
    response_error: GString,

    #[var]
    response_status_code: i32,
    #[var]
    response_headers: Dictionary,

    // PAYLOAD RESPONSE
    #[var]
    response_payload_received: bool,
    #[var]
    response_payload_ok: bool,
    #[var]
    response_payload_received_at: u32,
    #[var]
    response_payload_error: GString,

    // not exposed
    request_body: Option<Vec<u8>>,
    response_body: Option<Vec<u8>>,
}

#[godot_api]
impl NetworkInspectedRequest {
    #[func]
    fn get_request_body(&self) -> String {
        self.request_body
            .as_ref()
            .map(|b| String::from_utf8_lossy(b).into_owned())
            .unwrap_or_default()
    }

    #[func]
    fn get_response_body(&self) -> String {
        self.request_body
            .as_ref()
            .map(|b| String::from_utf8_lossy(b).into_owned())
            .unwrap_or_default()
    }
}

static NETWORK_INSPECTED_REQUEST_ID_MONOTONIC_COUNTER: once_cell::sync::Lazy<
    std::sync::atomic::AtomicU32,
> = once_cell::sync::Lazy::new(|| std::sync::atomic::AtomicU32::new(0));

pub struct NetworkInspectEvent {
    id: NetworkInspectorId,
    payload: NetworkInspectPayload,
}

pub struct NetworkInspectRequestPayload {
    pub requester: String,
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
    requests: HashMap<NetworkInspectorId, Gd<NetworkInspectedRequest>>,
    receiver: tokio::sync::mpsc::Receiver<NetworkInspectEvent>,
    sender: tokio::sync::mpsc::Sender<NetworkInspectEvent>,
    _base: Base<Node>,
}

#[godot_api]
impl NetworkInspector {
    #[func]
    fn set_is_active(&mut self, value: bool) {
        NETWORK_INSPECTOR_ENABLE.store(value, std::sync::atomic::Ordering::Relaxed);
    }

    #[signal]
    fn request_changed(&self, id: u32) {}

    #[func]
    fn get_request_count(&self) -> u32 {
        self.requests.len() as u32
    }

    #[func]
    fn get_request(&self, id: u32) -> Option<Gd<NetworkInspectedRequest>> {
        self.requests
            .get(&NetworkInspectorId::from_u32(id))
            .cloned()
    }
}

#[godot_api]
impl INode for NetworkInspector {
    fn init(_base: Base<Node>) -> Self {
        let (sender, receiver) = tokio::sync::mpsc::channel(1000);
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
                    let mut inspected_request = NetworkInspectedRequest {
                        requested_by: request.requester.into(),
                        requested_at: godot::engine::Time::singleton().get_ticks_msec() as u32,
                        url: request.url.into(),
                        method: request.method.as_str().into(),
                        request_headers: Dictionary::new(),
                        response_received_at: 0,
                        response_received: false,
                        response_ok: false,
                        response_error: "".into(),
                        response_status_code: 0,
                        response_headers: Dictionary::new(),
                        response_payload_received: false,
                        response_payload_ok: false,
                        response_payload_received_at: 0,
                        response_payload_error: "".into(),
                        request_body: request.body,
                        response_body: None,
                    };

                    if let Some(headers) = request.headers {
                        for (key, value) in headers {
                            inspected_request
                                .request_headers
                                .insert(key.to_variant(), value.to_variant());
                        }
                    }

                    self.requests
                        .insert(event.id, Gd::from_init_fn(|_base| inspected_request));
                }
                NetworkInspectPayload::PartialResponse(response) => {
                    if let Some(request_gd) = self.requests.get_mut(&event.id) {
                        let mut request = request_gd.bind_mut();
                        request.response_received_at =
                            godot::engine::Time::singleton().get_ticks_msec() as u32;
                        request.response_received = true;

                        match response {
                            Ok(response) => {
                                request.response_ok = true;
                                request.response_status_code = response.status_code.as_u16() as i32;
                                if let Some(headers) = response.headers {
                                    for (key, value) in headers {
                                        request
                                            .response_headers
                                            .insert(key.to_variant(), value.to_variant());
                                    }
                                }
                            }
                            Err(error) => {
                                request.response_ok = false;
                                request.response_error = error.into();
                            }
                        }
                    }
                }
                NetworkInspectPayload::BodyResponse(response) => {
                    if let Some(request_gd) = self.requests.get_mut(&event.id) {
                        let mut request = request_gd.bind_mut();
                        request.response_payload_received_at =
                            godot::engine::Time::singleton().get_ticks_msec() as u32;
                        request.response_payload_received = true;
                        match response {
                            Ok(body) => {
                                request.response_payload_ok = true;
                                request.response_body = body.map(|s| s.into_bytes());
                            }
                            Err(error) => {
                                request.response_payload_ok = false;
                                request.response_payload_error = error.into();
                            }
                        }
                    }
                }
                NetworkInspectPayload::FullResponse(response) => {
                    if let Some(request_gd) = self.requests.get_mut(&event.id) {
                        let mut request = request_gd.bind_mut();
                        request.response_payload_received_at =
                            godot::engine::Time::singleton().get_ticks_msec() as u32;
                        request.response_payload_received = true;

                        request.response_received_at =
                            godot::engine::Time::singleton().get_ticks_msec() as u32;
                        request.response_received = true;

                        match response {
                            Ok((response, body)) => {
                                request.response_payload_ok = true;
                                request.response_status_code = response.status_code.as_u16() as i32;
                                if let Some(headers) = response.headers {
                                    for (key, value) in headers {
                                        request
                                            .response_headers
                                            .insert(key.to_variant(), value.to_variant());
                                    }
                                }
                                request.response_body = body.map(|s| s.into_bytes());
                            }
                            Err(error) => {
                                request.response_payload_ok = false;
                                request.response_payload_error = error.into();
                            }
                        }
                    }
                }
            }
        }

        for id in request_changed {
            self.base_mut().call_deferred(
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
