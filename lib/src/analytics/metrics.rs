use std::{collections::HashMap, sync::Arc};

use godot::{classes::Timer, prelude::*};

use crate::{
    godot_classes::{
        dcl_android_plugin::DclAndroidPlugin,
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
        build_segment_event_batch_item, SegmentEvent, SegmentEventAcceptFriend,
        SegmentEventBlockUser, SegmentEventChatMessageSent, SegmentEventClickButton,
        SegmentEventCommonExplorerFields, SegmentEventExplorerMoveToParcel,
        SegmentEventRequestFriend, SegmentEventScreenViewed, SegmentEventUnfriend,
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

    // Debug level: 0=disabled, 1=enabled (full JSON output)
    debug_level: u8,

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
            debug_level: 0,
            base,
        }
    }

    fn ready(&mut self) {
        let mut timer = Timer::new_alloc();
        timer.set_wait_time(10.0);
        timer.set_one_shot(false);
        timer.set_autostart(true);

        let callable = self.base().callable("timer_timeout");
        timer.connect("timeout", &callable);

        self.base_mut().add_child(&timer.upcast::<Node>());

        // Check which mobile plugin is available and fetch static device info (checked once)
        if DclIosPlugin::is_available() {
            self.mobile_platform = Some(MobilePlatform::Ios);
            self.device_info = DclIosPlugin::get_mobile_device_info_internal();
            tracing::debug!("iOS mobile platform detected for metrics collection");
        } else if DclAndroidPlugin::is_available() {
            self.mobile_platform = Some(MobilePlatform::Android);
            self.device_info = DclAndroidPlugin::get_mobile_device_info_internal();
            tracing::debug!("Android mobile platform detected for metrics collection");
        }
    }

    fn process(&mut self, delta: f64) {
        // frame.process() returns Some only when 1000 frames have been collected
        if let Some(mut frame_data) = self.frame.process(1000.0 * delta as f32) {
            // Enrich the event with mobile/device/network data
            self.populate_event_metrics(&mut frame_data);
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
            debug_level: 0,
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
        let event = SegmentEvent::ExplorerMoveToParcel(
            position.clone(),
            SegmentEventExplorerMoveToParcel {
                old_parcel: self.common.position.clone(),
            },
        );
        self.events.push(event.clone());
        self.debug_print_event("Explorer Move To Parcel", &event);
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
        let event = SegmentEvent::ChatMessageSent(SegmentEventChatMessageSent {
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
        });
        self.events.push(event.clone());
        self.debug_print_event("Chat Message Sent", &event);
    }

    #[func]
    pub fn track_click_button(
        &mut self,
        button_text: String,
        screen_name: String,
        extra_properties: String,
    ) {
        let event = SegmentEvent::ClickButton(SegmentEventClickButton {
            button_text,
            screen_name,
            extra_properties: if extra_properties.is_empty() {
                None
            } else {
                Some(extra_properties)
            },
        });
        self.events.push(event.clone());
        self.debug_print_event("Click Button", &event);
    }

    #[func]
    pub fn track_screen_viewed(&mut self, screen_name: String, extra_properties: String) {
        let event = SegmentEvent::ScreenViewed(SegmentEventScreenViewed {
            screen_name,
            extra_properties: if extra_properties.is_empty() {
                None
            } else {
                Some(extra_properties)
            },
        });
        self.events.push(event.clone());
        self.debug_print_event("Screen Viewed", &event);
    }

    #[func]
    pub fn track_request_friend(&mut self, receiver_id: String) {
        let event = SegmentEvent::RequestFriend(SegmentEventRequestFriend { receiver_id });
        self.events.push(event.clone());
        self.debug_print_event("Friend Request", &event);
    }

    #[func]
    pub fn track_accept_friend(&mut self, receiver_id: String, friendship_id: String) {
        let event = SegmentEvent::AcceptFriend(SegmentEventAcceptFriend {
            receiver_id,
            friendship_id: if friendship_id.is_empty() {
                None
            } else {
                Some(friendship_id)
            },
        });
        self.events.push(event.clone());
        self.debug_print_event("Friend Accept", &event);
    }

    #[func]
    pub fn track_block_user(&mut self, receiver_id: String, is_friend: bool) {
        let event = SegmentEvent::BlockUser(SegmentEventBlockUser {
            receiver_id,
            is_friend,
        });
        self.events.push(event.clone());
        self.debug_print_event("Block User", &event);
    }

    #[func]
    pub fn track_unfriend(&mut self, receiver_id: String) {
        let event = SegmentEvent::Unfriend(SegmentEventUnfriend { receiver_id });
        self.events.push(event.clone());
        self.debug_print_event("Unfriend", &event);
    }

    #[func]
    pub fn flush(&mut self) {
        tracing::debug!("Flushing metrics - forcing immediate send of all pending events");

        // Process all events with ignore_batch_limit = true
        self.process_and_send_events(true);
    }

    #[func]
    pub fn set_debug_level(&mut self, level: u8) {
        self.debug_level = level.min(1); // Clamp to 0-1

        if self.debug_level == 1 {
            tracing::info!("Metrics debug mode enabled");
        } else {
            tracing::info!("Metrics debug mode disabled");
        }
    }

    #[func]
    pub fn get_debug_level(&self) -> u8 {
        self.debug_level
    }

    fn process_and_send_events(&mut self, ignore_batch_limit: bool) {
        tracing::debug!(
            "process_and_send_events: events={}, serialized={}, ignore_limit={}",
            self.events.len(),
            self.serialized_events.len(),
            ignore_batch_limit
        );

        if self.events.is_empty() && self.serialized_events.is_empty() {
            tracing::debug!("No events to process, returning early");
            return;
        }

        let http_requester = DclGlobal::singleton()
            .bind_mut()
            .get_http_requester()
            .bind_mut()
            .get_http_queue_requester();

        let mut accumulated_length: usize = self.serialized_events.iter().map(|s| s.len()).sum();

        tracing::debug!("Starting event processing loop");
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

        tracing::debug!(
            "Event processing loop complete. Serialized events: {}",
            self.serialized_events.len()
        );

        if !self.serialized_events.is_empty() {
            let http_requester = http_requester.clone();
            let write_key = self.write_key.clone();
            let serialized_events = std::mem::take(&mut self.serialized_events);
            tracing::debug!(
                "Spawning async task to send {} events",
                serialized_events.len()
            );
            TokioRuntime::spawn(async move {
                Self::send_segment_batch(http_requester, &write_key, &serialized_events).await;
            });
        } else {
            tracing::debug!("No serialized events to send");
        }
    }
}

impl Metrics {
    /// Print debug information for a queued event (full JSON when enabled)
    fn debug_print_event(&self, event_name: &str, event: &SegmentEvent) {
        if self.debug_level == 0 {
            return; // Disabled
        }

        // Build the complete event as it would be sent to Segment
        let event_body =
            build_segment_event_batch_item(self.user_id.clone(), &self.common, event.clone());

        let json = serde_json::to_string_pretty(&event_body)
            .unwrap_or_else(|e| format!("<serialization error: {}>", e));

        tracing::debug!("[Metrics] Event queued: {}\n{}", event_name, json);
    }

    fn populate_event_metrics(&self, event: &mut SegmentEvent) {
        if let SegmentEvent::PerformanceMetrics(metrics) = event {
            // Fetch dynamic mobile metrics ONLY when event is about to be sent
            let mobile_metrics = match self.mobile_platform {
                Some(MobilePlatform::Ios) => DclIosPlugin::get_mobile_metrics_internal(),
                Some(MobilePlatform::Android) => DclAndroidPlugin::get_mobile_metrics_internal(),
                None => None,
            };

            // Populate mobile metrics
            if let Some(mobile_metrics) = mobile_metrics {
                metrics.memory_usage = mobile_metrics.memory_usage;
                metrics.device_temperature_celsius =
                    Some(mobile_metrics.device_temperature_celsius);
                metrics.device_thermal_state = if mobile_metrics.device_thermal_state.is_empty() {
                    None
                } else {
                    Some(mobile_metrics.device_thermal_state)
                };
                metrics.battery_percent = if mobile_metrics.battery_percent >= 0.0 {
                    Some(mobile_metrics.battery_percent)
                } else {
                    None
                };
                metrics.charging_state = if mobile_metrics.charging_state.is_empty()
                    || mobile_metrics.charging_state == "unknown"
                {
                    None
                } else {
                    Some(mobile_metrics.charging_state)
                };
            }

            // Populate static device info
            if let Some(device_info) = &self.device_info {
                metrics.device_brand = if device_info.device_brand.is_empty() {
                    None
                } else {
                    Some(device_info.device_brand.clone())
                };
                metrics.device_model = if device_info.device_model.is_empty() {
                    None
                } else {
                    Some(device_info.device_model.clone())
                };
                metrics.os_version = if device_info.os_version.is_empty() {
                    None
                } else {
                    Some(device_info.os_version.clone())
                };
                metrics.total_ram_mb = if device_info.total_ram_mb >= 0 {
                    Some(device_info.total_ram_mb as u32)
                } else {
                    None
                };
            }

            // Populate network and player count from DclGlobal
            if let Some(global) = DclGlobal::try_singleton() {
                let global_bind = global.bind();

                // Network metrics
                let content_provider = global_bind.content_provider.clone();
                let content_provider_bind = content_provider.bind();

                let peak_speed = content_provider_bind.get_network_speed_peak_mbs();
                metrics.network_speed_peak_mbps = Some(peak_speed as f32);

                let used_last_minute = content_provider_bind.get_network_used_last_minute_mb();
                metrics.network_used_last_minute_mb = Some(used_last_minute as f32);

                // Player count
                metrics.player_count = global_bind.avatars.bind().get_avatars_count();

                // JavaScript (V8) heap memory metrics (requires use_deno feature)
                #[cfg(feature = "use_deno")]
                {
                    let scene_runner = global_bind.scene_runner.clone();
                    let scene_runner_bind = scene_runner.bind();

                    let jsheap_total_mb = scene_runner_bind.get_total_deno_memory_mb();
                    let jsheap_count = scene_runner_bind.get_deno_scene_count();
                    let jsheap_avg_mb = scene_runner_bind.get_average_deno_memory_mb();

                    if jsheap_count > 0 {
                        // Populate existing field in megabytes
                        metrics.used_jsheap_size = jsheap_total_mb as i32;
                        // Populate additional context fields
                        metrics.js_scene_count = Some(jsheap_count);
                        metrics.average_jsheap_mb = Some(jsheap_avg_mb as f32);
                    }
                }

                // Dynamic graphics metrics
                let dynamic_manager = global_bind.dynamic_graphics_manager.clone();
                let dm_bind = dynamic_manager.bind();
                metrics.dynamic_graphics_enabled = Some(dm_bind.is_enabled());
                if dm_bind.is_enabled() {
                    metrics.dynamic_graphics_state = Some(dm_bind.get_state_name().to_string());
                    metrics.dynamic_graphics_profile = Some(dm_bind.get_current_profile());
                    metrics.frame_time_ratio = Some(dm_bind.get_frame_time_ratio() as f32);
                    // Use the internal thermal state from the dynamic manager
                    metrics.dynamic_graphics_thermal_state =
                        Some(dm_bind.get_thermal_state_string().to_string());
                }

                // Hardware benchmark result from config (GDScript property)
                let config = global_bind.config.clone();
                let gpu_score = config.get("benchmark_gpu_score");
                if let Ok(score) = gpu_score.try_to::<f64>() {
                    if score > 0.0 {
                        metrics.benchmark_gpu_score = Some(score as f32);
                    }
                }

                // Optimized asset counters
                let opt_scenes = content_provider_bind.get_optimized_scene_count();
                let rt_scenes = content_provider_bind.get_runtime_scene_count();
                let opt_wearables = content_provider_bind.get_optimized_wearable_count();
                let rt_wearables = content_provider_bind.get_runtime_wearable_count();

                metrics.optimized_scene_count = Some(opt_scenes);
                metrics.runtime_scene_count = Some(rt_scenes);
                metrics.optimized_wearable_count = Some(opt_wearables);
                metrics.runtime_wearable_count = Some(rt_wearables);

                let total_scenes = opt_scenes + rt_scenes;
                metrics.optimized_scene_pct = if total_scenes > 0 {
                    Some((opt_scenes as f32 / total_scenes as f32) * 100.0)
                } else {
                    None
                };

                let total_wearables = opt_wearables + rt_wearables;
                metrics.optimized_wearable_pct = if total_wearables > 0 {
                    Some((opt_wearables as f32 / total_wearables as f32) * 100.0)
                } else {
                    None
                };
            }
        }
    }

    async fn send_segment_batch(
        http_requester: Arc<HttpQueueRequester>,
        write_key: &str,
        events: &[String],
    ) {
        // Log the events being sent
        tracing::debug!("Sending segment batch with {} events", events.len());

        // Parse and log each event name
        for (idx, event) in events.iter().enumerate() {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(event) {
                if let Some(event_name) = parsed.get("event").and_then(|v| v.as_str()) {
                    tracing::debug!("  Event {}: {}", idx + 1, event_name);
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
            tracing::debug!("Segment batch sent successfully");
        }
    }
}
