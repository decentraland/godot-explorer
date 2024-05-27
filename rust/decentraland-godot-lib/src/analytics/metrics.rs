use std::sync::Arc;

use godot::{engine::Timer, prelude::*};

use crate::{
    godot_classes::dcl_global::DclGlobal,
    http_request::{
        http_queue_requester::HttpQueueRequester,
        request_response::{RequestOption, ResponseType},
    },
    scene_runner::tokio_runtime::TokioRuntime,
};

use super::{
    data_definition::{
        build_segment_event_batch_item, SegmentEvent, SegmentEventCommonExplorerFields,
        SegmentEventExplorerMoveToParcel,
    },
    frame::Frame,
};

#[derive(GodotClass)]
#[class(base=Node)]
pub struct Metrics {
    // Frame metrics
    frame: Frame,

    // Config
    user_id: String,
    write_key: String,

    // Common data to serialize
    common: SegmentEventCommonExplorerFields,

    // Collect events to send
    events: Vec<SegmentEvent>,
    serialized_events: Vec<String>,

    base: Base<Node>,
}

const SEGMENT_EVENT_SIZE_LIMIT_BYTES: usize = 32000;
const SEGMENT_BATCH_SIZE_LIMIT_BYTES: usize = 500000;

#[godot_api]
impl INode for Metrics {
    fn init(base: Base<Node>) -> Self {
        Self {
            user_id: "".into(),
            common: SegmentEventCommonExplorerFields::new("".into()),
            write_key: "".into(),
            frame: Frame::new(),
            events: Vec::new(),
            serialized_events: Vec::new(),
            base,
        }
    }

    fn ready(&mut self) {
        let mut timer = Timer::new_alloc();
        timer.set_wait_time(10.0);
        timer.set_one_shot(false);
        timer.set_autostart(true);

        let callable = self.base.callable("timer_timeout");
        timer.connect("timeout".into(), callable);

        self.base.add_child(timer.upcast());
    }

    fn process(&mut self, delta: f64) {
        if let Some(frame_data) = self.frame.process(1000.0 * delta as f32) {
            self.events.push(frame_data);
        }
    }
}

#[godot_api]
impl Metrics {
    #[func]
    fn timer_timeout(&mut self) {
        if !self.events.is_empty() || self.serialized_events.is_empty() {
            let http_requester = DclGlobal::singleton()
                .bind_mut()
                .get_http_requester()
                .bind_mut()
                .get_http_queue_requester();

            let mut accumulated_length: usize =
                self.serialized_events.iter().map(|s| s.len()).sum();

            while let Some(event) = self.events.pop() {
                let raw_event =
                    build_segment_event_batch_item(self.user_id.clone(), &self.common, event);

                let json_body =
                    serde_json::to_string(&raw_event).expect("Failed to serialize event body");

                if json_body.len() > SEGMENT_EVENT_SIZE_LIMIT_BYTES {
                    tracing::error!("Event too large: {}", json_body.len());
                    continue;
                }

                if accumulated_length + json_body.len() > SEGMENT_BATCH_SIZE_LIMIT_BYTES {
                    let http_requester = http_requester.clone();
                    let write_key = self.write_key.clone();
                    let serialized_events = std::mem::take(&mut self.serialized_events);
                    TokioRuntime::spawn(async move {
                        Self::send_segment_batch(http_requester, &write_key, &serialized_events)
                            .await;
                    });

                    // This events is queued until the next time is available to send events
                    self.serialized_events.push(json_body);
                    return;
                }

                accumulated_length += json_body.len();
                self.serialized_events.push(json_body);
            }

            if !self.serialized_events.is_empty() {
                let http_requester = http_requester.clone();
                let write_key = self.write_key.clone();
                let serialized_events = std::mem::take(&mut self.serialized_events);
                TokioRuntime::spawn(async move {
                    Self::send_segment_batch(http_requester, &write_key, &serialized_events).await;
                });
            }
        }
    }

    #[func]
    pub fn create_metrics(user_id: String, session_id: String) -> Gd<Metrics> {
        Gd::from_init_fn(|base| Self {
            user_id,
            common: SegmentEventCommonExplorerFields::new(session_id),
            write_key: "EAdAcIyGP6lIQAfpFF2BXpNzpj7XNWMm".into(),
            frame: Frame::new(),
            events: Vec::new(),
            serialized_events: Vec::new(),
            base,
        })
    }

    #[func]
    pub fn update_realm(&mut self, realm: String) {
        self.common.realm = realm;
    }

    #[func]
    pub fn update_identity(&mut self, dcl_eth_address: String, dcl_is_guest: bool) {
        self.common.dcl_eth_address = dcl_eth_address;
        self.common.dcl_is_guest = dcl_is_guest;
    }

    #[func]
    pub fn update_position(&mut self, position: String) {
        self.events.push(SegmentEvent::ExplorerMoveToParcel(
            position.clone(),
            SegmentEventExplorerMoveToParcel {
                old_parcel: self.common.position.clone(),
            },
        ));
        self.common.position = position;
    }
}

impl Metrics {
    async fn send_segment_batch(
        http_requester: Arc<HttpQueueRequester>,
        write_key: &str,
        events: &[String],
    ) {
        let json_body = format!(
            "{{\"writeKey\":\"{}\",\"batch\":[{}]}}",
            write_key,
            events.join(",")
        );

        let request = RequestOption::new(
            0,
            "https://api.segment.io/v1/batch".into(),
            http::Method::POST,
            ResponseType::AsString,
            Some(json_body.as_bytes().to_vec()),
            Some(vec!["Content-Type: application/json".to_string()]),
            None,
        );
        if let Err(err) = http_requester.request(request, 0).await {
            tracing::error!("Failed to send segment batch: {:?}", err);
        }
    }
}
