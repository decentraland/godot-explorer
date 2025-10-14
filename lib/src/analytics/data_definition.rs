use serde::Serialize;

#[derive(Serialize)]
pub struct SegmentMetricEventBody {
    #[serde(rename = "type")]
    r#type: String,
    event: String,
    #[serde(rename = "userId")]
    user_id: String,
    properties: serde_json::Value,
}

#[derive(Serialize)]
// Same for all events sent from the explorer
pub struct SegmentEventCommonExplorerFields {
    // User’s wallet id, even for guests.
    pub dcl_eth_address: String,
    // If the user is a guest or not.
    pub dcl_is_guest: bool,
    // Realm where the user is connected.
    pub realm: String,
    // Current user position.
    pub position: String,
    // What type of client was used to render the world (Web/Native/VR)
    pub dcl_renderer_type: String,
    // Explorer’s unique session id.
    pub session_id: String,
    // Explorer’s release used.
    pub renderer_version: String,
}

impl SegmentEventCommonExplorerFields {
    pub fn new(session_id: String) -> Self {
        let dcl_renderer_type = format!("dao-godot-{}", godot::engine::Os::singleton().get_name());

        Self {
            dcl_eth_address: "unauthenticated".into(),
            dcl_is_guest: false,
            realm: "no-realm".into(),
            position: "no-position".into(),
            dcl_renderer_type,
            session_id,
            renderer_version: env!("GODOT_EXPLORER_VERSION").into(),
        }
    }
}

pub enum SegmentEvent {
    PerformanceMetrics(SegmentEventPerformanceMetrics),
    ExplorerError(SegmentEventExplorerError),
    ExplorerSceneLoadTimes(SegmentEventExplorerSceneLoadTimes),
    ExplorerMoveToParcel(String, SegmentEventExplorerMoveToParcel),
    SystemInfoReport(SegmentEventSystemInfoReport),
    ChatMessageSent(SegmentEventChatMessageSent),
    ClickButton(SegmentEventClickButton),
    ScreenViewed(SegmentEventScreenViewed),
}

#[derive(Serialize)]
pub struct SegmentEventPerformanceMetrics {
    // Total number of frames measured for this event.
    pub samples: u32,
    // Total length of the performance report.
    pub total_time: f32,
    // Amount of hiccups in 1000 frames.
    pub hiccups_in_thousand_frames: u32,
    // Total time length of hiccups measured in seconds.
    pub hiccups_time: f32,
    // Minimum delta (difference) between frames in milliseconds
    pub min_frame_time: f32,
    // Maximum delta (difference) between frames in milliseconds
    pub max_frame_time: f32,
    // Average delta (difference) between frames in milliseconds
    pub mean_frame_time: f32,
    // Median delta (difference) between frames in milliseconds
    pub median_frame_time: f32,
    // Percentile 1 of the delta (difference) between frames in milliseconds
    pub p1_frame_time: f32,
    // Percentile 5 of the delta (difference) between frames in milliseconds
    pub p5_frame_time: f32,
    // Percentile 10 of the delta (difference) between frames in milliseconds
    pub p10_frame_time: f32,
    // Percentile 20 of the delta (difference) between frames in milliseconds
    pub p20_frame_time: f32,
    // Percentile 50 of the delta (difference) between frames in milliseconds
    pub p50_frame_time: f32,
    // Percentile 75 of the delta (difference) between frames in milliseconds
    pub p75_frame_time: f32,
    // Percentile 80 of the delta (difference) between frames in milliseconds
    pub p80_frame_time: f32,
    // Percentile 90 of the delta (difference) between frames in milliseconds
    pub p90_frame_time: f32,
    // Percentile 95 of the delta (difference) between frames in milliseconds
    pub p95_frame_time: f32,
    // Percentile 99 of the delta (difference) between frames in milliseconds
    pub p99_frame_time: f32,
    // How many users where nearby the current user
    pub player_count: i32,
    // Javascript heap memory used by the scenes in kilo bytes
    pub used_jsheap_size: i32,
    // Memory used only by the explorer in kilo bytes
    pub memory_usage: i32,
    // Mobile-specific metrics (only sent on iOS/Android)
    // Device temperature in Celsius (Android only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_temperature_celsius: Option<f32>,
    // Device thermal state (iPhone only: nominal/fair/serious/critical)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_thermal_state: Option<String>,
    // Battery drain percentage per hour
    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery_drain_pct_per_hour: Option<f32>,
    // Device brand (e.g., "Apple", "Samsung")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_brand: Option<String>,
    // Device model (e.g., "iPhone 15 Pro", "Galaxy A53")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_model: Option<String>,
    // OS version (e.g., "iOS 17.0", "Android 15.0")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os_version: Option<String>,
    // Total device RAM in megabytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_ram_mb: Option<u32>,
    // Current RAM consumption in megabytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ram_consumption_mb: Option<u32>,
    // Network type (WiFi, Carrier 3G, Carrier 4G, Carrier 5G)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_type: Option<String>,
    // Network speed in Mbps (to be calculated later)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_speed_mbps: Option<f32>,
}

#[derive(Serialize)]
pub struct SegmentEventExplorerError {
    // Generic or Fatal.
    error_type: String,
    // Error description.
    error_message: String,
    // Error’s stack
    error_stack: String,
}

#[derive(Serialize)]
pub struct SegmentEventExplorerSceneLoadTimes {
    // Unique hash for the scene.
    scene_hash: String,
    // Time to load in seconds.
    elapsed: f32,
    // Boolean flag indicating wether the scene loaded without errors.
    success: bool,
}

// TODO: maybe important what realm?
#[derive(Serialize)]
pub struct SegmentEventExplorerMoveToParcel {
    // Parcel where the user is coming from.
    pub old_parcel: String,
}

#[derive(Serialize)]
pub struct SegmentEventSystemInfoReport {
    // Processor used by the user.
    processor_type: String,
    // How many processors are available in user's device.
    processor_count: u32,
    // Graphic Device used by the user.
    graphics_device_name: String,
    // Graphic device memory in mb.
    graphics_memory_mb: u32,
    // RAM memory in mb.
    system_memory_size_mb: u32,
}

#[derive(Serialize)]
pub struct SegmentEventChatMessageSent {
    // Length of the message sent.
    pub length: u32,
    // Whether it is Public or Private.
    pub channel: String,
    // Whether the message typed is a command or not (if applies).
    pub is_command: bool,
    // Whether the message is Private or not.
    pub is_private: bool,
    // ID of the Community the message was sent to. Otherwise NULL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub community_id: Option<String>,
    // Whether the message contains a mention from another User (i.e. @XYZ).
    pub is_mention: bool,
    // If the user is not in world, this is the screen where the event is fired.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screen_name: Option<String>,
}

#[derive(Serialize)]
pub struct SegmentEventClickButton {
    // Text of the button clicked.
    pub button_text: String,
    // Screen Name where the user clicked the button.
    pub screen_name: String,
    // JSON with extra payload, in case we need to track additional metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_properties: Option<String>,
}

#[derive(Serialize)]
pub struct SegmentEventScreenViewed {
    // Name of the screen viewed.
    pub screen_name: String,
}

pub fn build_segment_event_batch_item(
    user_id: String,
    common: &SegmentEventCommonExplorerFields,
    event_data: SegmentEvent,
) -> SegmentMetricEventBody {
    let (event_name, event_properties, override_position) = match event_data {
        SegmentEvent::PerformanceMetrics(event) => (
            "Performance Metrics".to_string(),
            serde_json::to_value(event).unwrap(),
            None,
        ),
        SegmentEvent::ExplorerError(event) => (
            "Explorer Error".to_string(),
            serde_json::to_value(event).unwrap(),
            None,
        ),
        SegmentEvent::ExplorerSceneLoadTimes(event) => (
            "Explorer Scene Load Times".to_string(),
            serde_json::to_value(event).unwrap(),
            None,
        ),
        SegmentEvent::ExplorerMoveToParcel(current_position, event) => (
            "Explorer Move To Parcel".to_string(),
            serde_json::to_value(event).unwrap(),
            Some(current_position),
        ),
        SegmentEvent::SystemInfoReport(event) => (
            "System Info Report".to_string(),
            serde_json::to_value(event).unwrap(),
            None,
        ),
        SegmentEvent::ChatMessageSent(event) => (
            "Chat Message Sent".to_string(),
            serde_json::to_value(event).unwrap(),
            None,
        ),
        SegmentEvent::ClickButton(event) => (
            "Click Button".to_string(),
            serde_json::to_value(event).unwrap(),
            None,
        ),
        SegmentEvent::ScreenViewed(event) => (
            "Screen Viewed".to_string(),
            serde_json::to_value(event).unwrap(),
            None,
        ),
    };

    let mut properties = serde_json::to_value(common).unwrap();
    // merge specific event properties with common properties
    for (k, v) in event_properties.as_object().unwrap().iter() {
        properties[k] = v.clone();
    }

    if let Some(position) = override_position {
        properties["position"] = serde_json::Value::String(position);
    }

    SegmentMetricEventBody {
        r#type: "track".to_string(),
        event: event_name,
        user_id,
        properties,
    }
}
