use std::collections::{HashMap, HashSet};

use godot::prelude::*;

pub use dcl_core::network_inspector::{
    NetworkInspectEvent, NetworkInspectPayload, NetworkInspectRequestPayload,
    NetworkInspectResponsePayload, NetworkInspectorId, NetworkInspectorSender,
    NETWORK_INSPECTOR_ENABLE,
};

#[derive(GodotClass)]
#[class(init, base=RefCounted)]
struct NetworkInspectedRequest {
    #[var(pub)]
    requested_by: GString,

    // REQUEST
    #[var(pub)]
    requested_at: u32,

    #[var(pub)]
    url: GString,
    #[var(pub)]
    method: GString,

    #[var(pub)]
    request_headers: VarDictionary,

    // RESPONSE
    #[var(pub)]
    response_received_at: u32,
    #[var(pub)]
    response_received: bool,
    #[var(pub)]
    response_ok: bool,
    #[var(pub)]
    response_error: GString,

    #[var(pub)]
    response_status_code: i32,
    #[var(pub)]
    response_headers: VarDictionary,

    // PAYLOAD RESPONSE
    #[var(pub)]
    response_payload_received: bool,
    #[var(pub)]
    response_payload_ok: bool,
    #[var(pub)]
    response_payload_received_at: u32,
    #[var(pub)]
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
    fn request_changed(id: u32);

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
            request_changed.insert(event.id.to_u32());
            // Tee into the unified scene-inspector stream as a `"network"` entry
            // (additive; no-op unless a consumer subscribed to network).
            if crate::tools::scene_inspector::is_stream_network() {
                crate::tools::scene_inspector::emit_network(network_entry_from(&event));
            }
            match event.payload {
                NetworkInspectPayload::Request(request) => {
                    let mut inspected_request = NetworkInspectedRequest {
                        requested_by: GString::from(request.requester.as_str()),
                        requested_at: godot::classes::Time::singleton().get_ticks_msec() as u32,
                        url: GString::from(request.url.as_str()),
                        method: request.method.as_str().into(),
                        request_headers: VarDictionary::new(),
                        response_received_at: 0,
                        response_received: false,
                        response_ok: false,
                        response_error: "".into(),
                        response_status_code: 0,
                        response_headers: VarDictionary::new(),
                        response_payload_received: false,
                        response_payload_ok: false,
                        response_payload_received_at: 0,
                        response_payload_error: "".into(),
                        request_body: request.body,
                        response_body: None,
                    };

                    if let Some(headers) = request.headers {
                        for (key, value) in headers {
                            let _ = inspected_request
                                .request_headers
                                .insert(&key.to_variant(), &value.to_variant());
                        }
                    }

                    self.requests
                        .insert(event.id, Gd::from_init_fn(|_base| inspected_request));
                }
                NetworkInspectPayload::PartialResponse(response) => {
                    if let Some(request_gd) = self.requests.get_mut(&event.id) {
                        let mut request = request_gd.bind_mut();
                        request.response_received_at =
                            godot::classes::Time::singleton().get_ticks_msec() as u32;
                        request.response_received = true;

                        match response {
                            Ok(response) => {
                                request.response_ok = true;
                                request.response_status_code = response.status_code.as_u16() as i32;
                                if let Some(headers) = response.headers {
                                    for (key, value) in headers {
                                        let _ = request
                                            .response_headers
                                            .insert(&key.to_variant(), &value.to_variant());
                                    }
                                }
                            }
                            Err(error) => {
                                request.response_ok = false;
                                request.response_error = GString::from(error.as_str());
                            }
                        }
                    }
                }
                NetworkInspectPayload::BodyResponse(response) => {
                    if let Some(request_gd) = self.requests.get_mut(&event.id) {
                        let mut request = request_gd.bind_mut();
                        request.response_payload_received_at =
                            godot::classes::Time::singleton().get_ticks_msec() as u32;
                        request.response_payload_received = true;
                        match response {
                            Ok(body) => {
                                request.response_payload_ok = true;
                                request.response_body = body.map(|s| s.into_bytes());
                            }
                            Err(error) => {
                                request.response_payload_ok = false;
                                request.response_payload_error = GString::from(error.as_str());
                            }
                        }
                    }
                }
                NetworkInspectPayload::FullResponse(response) => {
                    if let Some(request_gd) = self.requests.get_mut(&event.id) {
                        let mut request = request_gd.bind_mut();
                        request.response_payload_received_at =
                            godot::classes::Time::singleton().get_ticks_msec() as u32;
                        request.response_payload_received = true;

                        request.response_received_at =
                            godot::classes::Time::singleton().get_ticks_msec() as u32;
                        request.response_received = true;

                        match response {
                            Ok((response, body)) => {
                                request.response_payload_ok = true;
                                request.response_status_code = response.status_code.as_u16() as i32;
                                if let Some(headers) = response.headers {
                                    for (key, value) in headers {
                                        let _ = request
                                            .response_headers
                                            .insert(&key.to_variant(), &value.to_variant());
                                    }
                                }
                                request.response_body = body.map(|s| s.into_bytes());
                            }
                            Err(error) => {
                                request.response_payload_ok = false;
                                request.response_payload_error = GString::from(error.as_str());
                            }
                        }
                    }
                }
            }
        }

        for id in request_changed {
            self.base_mut().call_deferred(
                "emit_signal",
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

/// Build a scene-inspector `NetworkEntry` from a `NetworkInspectEvent` so HTTP
/// traffic can be folded into the unified stream. Mirrors the event's phase.
fn network_entry_from(event: &NetworkInspectEvent) -> crate::tools::scene_inspector::NetworkEntry {
    use crate::tools::scene_inspector::{current_timestamp_ms, NetworkEntry};
    let mut entry = NetworkEntry {
        timestamp_ms: current_timestamp_ms(),
        id: event.id.to_u32(),
        phase: String::new(),
        url: None,
        method: None,
        requester: None,
        status: None,
        ok: None,
        error: None,
    };
    match &event.payload {
        NetworkInspectPayload::Request(r) => {
            entry.phase = "request".into();
            entry.url = Some(r.url.clone());
            entry.method = Some(r.method.as_str().to_string());
            entry.requester = Some(r.requester.clone());
        }
        NetworkInspectPayload::PartialResponse(res) => {
            entry.phase = "partial_response".into();
            match res {
                Ok(p) => {
                    entry.status = Some(p.status_code.as_u16());
                    entry.ok = Some(true);
                }
                Err(e) => {
                    entry.ok = Some(false);
                    entry.error = Some(e.clone());
                }
            }
        }
        NetworkInspectPayload::BodyResponse(res) => {
            entry.phase = "body_response".into();
            match res {
                Ok(_) => entry.ok = Some(true),
                Err(e) => {
                    entry.ok = Some(false);
                    entry.error = Some(e.clone());
                }
            }
        }
        NetworkInspectPayload::FullResponse(res) => {
            entry.phase = "full_response".into();
            match res {
                Ok((p, _)) => {
                    entry.status = Some(p.status_code.as_u16());
                    entry.ok = Some(true);
                }
                Err(e) => {
                    entry.ok = Some(false);
                    entry.error = Some(e.clone());
                }
            }
        }
    }
    entry
}
