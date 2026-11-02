use std::{collections::HashMap, sync::atomic::AtomicBool};

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

static NETWORK_INSPECTED_REQUEST_ID_MONOTONIC_COUNTER: once_cell::sync::Lazy<
    std::sync::atomic::AtomicU32,
> = once_cell::sync::Lazy::new(|| std::sync::atomic::AtomicU32::new(0));

pub struct NetworkInspectEvent {
    pub id: NetworkInspectorId,
    pub payload: NetworkInspectPayload,
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
