use std::{collections::HashMap, sync::Arc};

use godot::{engine::Timer, prelude::*};

use crate::{
    godot_classes::{
        dcl_android_plugin::DclGodotAndroidPlugin,
        dcl_global::DclGlobal,
        dcl_ios_plugin::{DclIosPlugin, DclMobileDeviceInfo},
    },
    http_request::{
        http_queue_requester::HttpQueueRequester,
        request_response::{RequestOption, ResponseType},
    },
    scene_runner::tokio_runtime::TokioRuntime,
};

use super::{
    data_definition::{
        build_segment_event_batch_item, SegmentEvent, SegmentEventChatMessageSent,
        SegmentEventClickButton, SegmentEventCommonExplorerFields,
        SegmentEventExplorerMoveToParcel, SegmentEventScreenViewed,
    },
    frame::Frame,
};

#[derive(Clone, Copy)]
enum MobilePlatform {
    Ios,
    Android,
}

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

    // Which mobile platform is available (checked once at ready)
    mobile_platform: Option<MobilePlatform>,
    // Static mobile device info (fetched once at ready)
    device_info: Option<DclMobileDeviceInfo>,

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
            mobile_platform: None,
            device_info: None,
            base,
        }
    }

    fn ready(&mut self) {
        let mut timer = Timer::new_alloc();
        timer.set_wait_time(10.0);
        timer.set_one_shot(false);
        timer.set_autostart(true);

        let callable = self.base().callable("timer_timeout");
        timer.connect("timeout".into(), callable);

        self.base_mut().add_child(timer.upcast());

        // Check which mobile plugin is available and fetch static device info (checked once)
        if DclIosPlugin::is_available() {
            self.mobile_platform = Some(MobilePlatform::Ios);
            self.device_info = DclIosPlugin::get_mobile_device_info_internal();
            tracing::info!("iOS mobile platform detected for metrics collection");
        } else if DclGodotAndroidPlugin::is_available() {
            self.mobile_platform = Some(MobilePlatform::Android);
            self.device_info = DclGodotAndroidPlugin::get_mobile_device_info_internal();
            tracing::info!("Android mobile platform detected for metrics collection");
        }
    }

    fn process(&mut self, delta: f64) {
        // Get fresh mobile metrics based on detected platform
        let mobile_metrics = match self.mobile_platform {
            Some(MobilePlatform::Ios) => DclIosPlugin::get_mobile_metrics_internal(),
            Some(MobilePlatform::Android) => DclGodotAndroidPlugin::get_mobile_metrics_internal(),
            None => None,
        };

        if let Some(frame_data) = self.frame.process(
            1000.0 * delta as f32,
            self.device_info.as_ref(),
            mobile_metrics.as_ref(),
        ) {
            self.events.push(frame_data);
        }
    }
}

#[godot_api]
impl Metrics {
    #[func]
    fn timer_timeout(&mut self) {
        self.process_and_send_events(false);
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
            mobile_platform: None,
            device_info: None,
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

    #[func]
    #[allow(clippy::too_many_arguments)]
    pub fn track_chat_message_sent(
        &mut self,
        length: u32,
        channel: String,
        is_private: bool,
        is_mention: bool,
        is_command: bool,
        community_id: String,
        screen_name: String,
    ) {
        self.events
            .push(SegmentEvent::ChatMessageSent(SegmentEventChatMessageSent {
                length,
                channel,
                is_command,
                is_private,
                community_id: if community_id.is_empty() {
                    None
                } else {
                    Some(community_id)
                },
                is_mention,
                screen_name: if screen_name.is_empty() {
                    None
                } else {
                    Some(screen_name)
                },
            }));
    }

    #[func]
    pub fn track_click_button(
        &mut self,
        button_text: String,
        screen_name: String,
        extra_properties: String,
    ) {
        self.events
            .push(SegmentEvent::ClickButton(SegmentEventClickButton {
                button_text,
                screen_name,
                extra_properties: if extra_properties.is_empty() {
                    None
                } else {
                    Some(extra_properties)
                },
            }));
    }

    #[func]
    pub fn track_screen_viewed(&mut self, screen_name: String) {
        self.events
            .push(SegmentEvent::ScreenViewed(SegmentEventScreenViewed {
                screen_name,
            }));
    }

    #[func]
    pub fn flush(&mut self) {
        tracing::warn!("Flushing metrics - forcing immediate send of all pending events");

        // Process all events with ignore_batch_limit = true
        self.process_and_send_events(true);
    }

    fn process_and_send_events(&mut self, ignore_batch_limit: bool) {
        tracing::info!(
            "process_and_send_events: events={}, serialized={}, ignore_limit={}",
            self.events.len(),
            self.serialized_events.len(),
            ignore_batch_limit
        );

        if self.events.is_empty() && self.serialized_events.is_empty() {
            tracing::info!("No events to process, returning early");
            return;
        }

        let http_requester = DclGlobal::singleton()
            .bind_mut()
            .get_http_requester()
            .bind_mut()
            .get_http_queue_requester();

        let mut accumulated_length: usize = self.serialized_events.iter().map(|s| s.len()).sum();

        tracing::info!("Starting event processing loop");
        while let Some(event) = self.events.pop() {
            let raw_event =
                build_segment_event_batch_item(self.user_id.clone(), &self.common, event);

            let json_body =
                serde_json::to_string(&raw_event).expect("Failed to serialize event body");

            if json_body.len() > SEGMENT_EVENT_SIZE_LIMIT_BYTES {
                tracing::error!("Event too large: {}", json_body.len());
                continue;
            }

            if !ignore_batch_limit
                && accumulated_length + json_body.len() > SEGMENT_BATCH_SIZE_LIMIT_BYTES
            {
                let http_requester = http_requester.clone();
                let write_key = self.write_key.clone();
                let serialized_events = std::mem::take(&mut self.serialized_events);
                TokioRuntime::spawn(async move {
                    Self::send_segment_batch(http_requester, &write_key, &serialized_events).await;
                });

                // This event is queued until the next time is available to send events
                self.serialized_events.push(json_body);
                return;
            }

            accumulated_length += json_body.len();
            self.serialized_events.push(json_body);
        }

        tracing::info!(
            "Event processing loop complete. Serialized events: {}",
            self.serialized_events.len()
        );

        if !self.serialized_events.is_empty() {
            let http_requester = http_requester.clone();
            let write_key = self.write_key.clone();
            let serialized_events = std::mem::take(&mut self.serialized_events);
            tracing::info!(
                "Spawning async task to send {} events",
                serialized_events.len()
            );
            TokioRuntime::spawn(async move {
                Self::send_segment_batch(http_requester, &write_key, &serialized_events).await;
            });
        } else {
            tracing::info!("No serialized events to send");
        }
    }
}

impl Metrics {
    async fn send_segment_batch(
        http_requester: Arc<HttpQueueRequester>,
        write_key: &str,
        events: &[String],
    ) {
        // Log the events being sent
        tracing::warn!("Sending segment batch with {} events", events.len());

        // Parse and log each event name
        for (idx, event) in events.iter().enumerate() {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(event) {
                if let Some(event_name) = parsed.get("event").and_then(|v| v.as_str()) {
                    tracing::info!("  Event {}: {}", idx + 1, event_name);
                }
            }
        }

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
            Some(HashMap::from([(
                "Content-Type".to_string(),
                "application/json".to_string(),
            )])),
            None,
        );
        if let Err(err) = http_requester.request(request, 0).await {
            tracing::error!("Failed to send segment batch: {:?}", err);
        } else {
            tracing::info!("Segment batch sent successfully");
        }
    }
}
