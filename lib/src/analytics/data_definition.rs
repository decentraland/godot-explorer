use chrono::{DateTime, SecondsFormat, Utc};
use godot::{classes::Os, obj::Singleton};
use serde::Serialize;

#[derive(Serialize)]
pub struct SegmentMetricEventBody {
    #[serde(rename = "type")]
    r#type: String,
    event: String,
    #[serde(rename = "userId")]
    user_id: String,
    // Client-generated unique id. Segment deduplicates on this (>=24h window, ~170d in
    // practice), which makes retries idempotent. Must be <100 chars — a UUID v4 is 36.
    #[serde(rename = "messageId")]
    message_id: String,
    // Segment-reserved event time, captured when the event was created on the device (NOT at
    // flush). Segment stores this as `originalTimestamp` and, together with the batch-level
    // `sentAt`, corrects device clock skew into the canonical `timestamp` column.
    timestamp: String,
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
        let dcl_renderer_type = format!("dao-godot-{}", Os::singleton().get_name());

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

#[derive(Clone)]
pub enum SegmentEvent {
    PerformanceMetrics(Box<SegmentEventPerformanceMetrics>),
    ExplorerError(SegmentEventExplorerError),
    ExplorerSceneLoadTimes(SegmentEventExplorerSceneLoadTimes),
    ExplorerMoveToParcel(String, SegmentEventExplorerMoveToParcel),
    SystemInfoReport(SegmentEventSystemInfoReport),
    ChatMessageSent(SegmentEventChatMessageSent),
    ClickButton(SegmentEventClickButton),
    ScreenViewed(SegmentEventScreenViewed),
    RequestFriend(SegmentEventRequestFriend),
    AcceptFriend(SegmentEventAcceptFriend),
    BlockUser(SegmentEventBlockUser),
    Unfriend(SegmentEventUnfriend),
    InstallAttribution(SegmentEventInstallAttribution),
    FirebaseInit(SegmentEventFirebaseInit),
    AttestationAttempt(SegmentEventAttestationAttempt),
    AttestationSessionCacheLoaded(SegmentEventAttestationSessionCacheLoaded),
    IosStoreKitEnvironment(SegmentEventIosStoreKitEnvironment),
    // Loading-pipeline funnel + diagnostics, emitted as a SINGLE "Loading Event"
    // discriminated by `type` and correlated across one full load by `loading_id`
    // (see SegmentEventLoading). Boxed: it is the largest variant by far.
    Loading(Box<SegmentEventLoading>),
}

/// Cross-system correlation anchor. The ONLY Segment event that carries the Firebase Analytics
/// app instance id — analysts pivot from Segment → Firebase via this event's `session_id` +
/// `firebase_user_id` pair. The inverse pivot (Firebase → Segment) uses the `user_id` /
/// `session_id` user properties that the plugin seeds on every Firebase event.
///
/// Only queued when `firebase_user_id` is non-empty (see `_on_firebase_app_instance_id_ready` in
/// metrics.rs); a missing id is logged as a `tracing::error!` and the event is skipped.
#[derive(Serialize, Clone)]
pub struct SegmentEventFirebaseInit {
    // Firebase Analytics app instance id (ga_pseudo_user_id). Guaranteed non-empty at queue time.
    pub firebase_user_id: String,
}

#[derive(Serialize, Clone)]
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
    // Javascript heap memory used by the scenes in megabytes
    pub used_jsheap_size: i32,
    // Memory used only by the explorer in kilo bytes (or populated from mobile metrics)
    pub memory_usage: i32,
    // Mobile device info (static - doesn't change during runtime)
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

    // Mobile metrics (dynamic - changes during runtime)
    // Device temperature in Celsius
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_temperature_celsius: Option<f32>,
    // Device thermal state (nominal/fair/serious/critical)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_thermal_state: Option<String>,
    // Battery percentage (0-100)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery_percent: Option<f32>,
    // Battery charging state (unknown/unplugged/plugged/usb/wireless/full)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub charging_state: Option<String>,
    // Network type (WiFi, Carrier 3G, Carrier 4G, Carrier 5G)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_type: Option<String>,
    // Peak network speed in Mbps
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_speed_peak_mbps: Option<f32>,
    // Total MB downloaded in the last minute
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_used_last_minute_mb: Option<f32>,

    // JavaScript (V8) memory metrics (additional context)
    // Number of active scenes with JavaScript runtimes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub js_scene_count: Option<i32>,
    // Average JS heap memory per scene in megabytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub average_jsheap_mb: Option<f32>,

    // Dynamic graphics system metrics
    // Whether dynamic graphics adjustment is enabled
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_graphics_enabled: Option<bool>,
    // Current state of the dynamic graphics system (Stabilizing/Monitoring/Adjusting)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_graphics_state: Option<String>,
    // Current graphic profile index (0=Very Low, 1=Low, 2=Medium, 3=High)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_graphics_profile: Option<i32>,
    // Frame time ratio (actual/target, <1 means headroom, >1 means struggling)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_time_ratio: Option<f32>,
    // Thermal state as interpreted by dynamic graphics (normal/high/critical)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_graphics_thermal_state: Option<String>,

    // Hardware benchmark result (from initial auto-detection)
    // GPU render time in milliseconds (lower is better)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub benchmark_gpu_score: Option<f32>,

    // Optimized asset usage counters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optimized_scene_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_scene_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optimized_wearable_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_wearable_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optimized_scene_pct: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optimized_wearable_pct: Option<f32>,
}

#[derive(Serialize, Clone)]
pub struct SegmentEventExplorerError {
    // Generic or Fatal.
    error_type: String,
    // Error description.
    error_message: String,
    // Error’s stack
    error_stack: String,
}

#[derive(Serialize, Clone)]
pub struct SegmentEventExplorerSceneLoadTimes {
    // Unique hash for the scene.
    scene_hash: String,
    // Time to load in seconds.
    elapsed: f32,
    // Boolean flag indicating wether the scene loaded without errors.
    success: bool,
}

/// Loading-event grammar version. Bump when the field set below changes.
pub const LOADING_SCHEMA_VERSION: u32 = 1;

/// Loading-pipeline funnel + diagnostics (#1602 / #1640 / #2450), emitted as ONE Segment event
/// (`"Loading Event"`) discriminated by `event_type` and correlated across a whole load by
/// `loading_id`. A "load" is one loading-screen-bounded episode; every event of that load
/// (`started` → `completed`, plus any `asset_failure`) shares its `loading_id`, so the funnel
/// reassembles in Segment by grouping on it. Built by the pure `LoadingFunnel`
/// (`scene_runner/loading_funnel.rs`) and emitted from `DclSceneManager` via
/// `Metrics::queue_loading_event`. Every type-specific field is `Option` and only populated for
/// its `type`. Values are bucketed/rounded — no PII, no parcel coords, no URLs.
#[derive(Serialize, Clone, Default)]
pub struct SegmentEventLoading {
    /// Discriminator: "started" | "completed" | "asset_failure" | "realm_change_failed".
    #[serde(rename = "type")]
    pub event_type: String,
    /// Correlation id for one complete load (episode). Shared by every event of that load.
    pub loading_id: u64,
    /// Schema/grammar version (LOADING_SCHEMA_VERSION).
    pub schema: u32,
    /// Coarse realm class: "world" | "genesis" | "other" | "unknown" (no URL/PII).
    pub realm_bucket: String,

    // --- type = "started" (context) ---
    /// Navigation entry point: "on_teleport" | "on_join_world" | "auto" | "-".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_world: Option<bool>,
    /// No resources were cached at load start (first-time / cold boot).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cold_cache: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_estimate_mbs: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,

    // --- type = "completed" (the funnel) ---
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reached_ready: Option<bool>,
    /// How the load ended: "completion" | "wall_clock_timeout" | "error" | "superseded" |
    /// "cancelled". `wall_clock_timeout` with `reached_ready=false` is the infinite-loading signal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dismissed_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_spawn_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_ready_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub complete_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scene_rendered_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub about_end_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discovery_ms: Option<i64>,
    /// Wall-clock ms the progress bar sat in the 25–30% plateau (the F-7 "looks frozen" signal).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_in_25_30_band_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase_breakdown: Option<LoadingPhaseBreakdown>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assets_expected: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assets_loaded: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assets_unfinished: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assets_errored: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peak_pending_assets: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frames: Option<i64>,

    // --- type = "asset_failure" | "realm_change_failed" ---
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
}

impl SegmentEventLoading {
    /// Base event with the always-present fields set and every type-specific field left `None`.
    /// Callers fill the fields for their `event_type` via struct-update (`..base`).
    pub fn base(event_type: &str, loading_id: u64, realm_bucket: String) -> Self {
        Self {
            event_type: event_type.to_string(),
            loading_id,
            schema: LOADING_SCHEMA_VERSION,
            realm_bucket,
            ..Default::default()
        }
    }
}

/// Entry ms of each loading phase since load begin (-1 = phase never reached). Analysts diff
/// consecutive phases for per-phase duration.
#[derive(Serialize, Clone)]
pub struct LoadingPhaseBreakdown {
    pub metadata: i64,
    pub spawning: i64,
    pub assets: i64,
    pub ready: i64,
    pub floating_islands: i64,
    pub done: i64,
}

impl Default for LoadingPhaseBreakdown {
    fn default() -> Self {
        Self {
            metadata: -1,
            spawning: -1,
            assets: -1,
            ready: -1,
            floating_islands: -1,
            done: -1,
        }
    }
}

// TODO: maybe important what realm?
#[derive(Serialize, Clone)]
pub struct SegmentEventExplorerMoveToParcel {
    // Parcel where the user is coming from.
    pub old_parcel: String,
}

#[derive(Serialize, Clone)]
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

#[derive(Serialize, Clone)]
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

#[derive(Serialize, Clone)]
pub struct SegmentEventClickButton {
    // Text of the button clicked.
    pub button_text: String,
    // Screen Name where the user clicked the button.
    pub screen_name: String,
    // JSON with extra payload, in case we need to track additional metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_properties: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct SegmentEventScreenViewed {
    // Name of the screen viewed.
    pub screen_name: String,
    // JSON with extra payload, in case we need to track additional metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_properties: Option<String>,
}

/// Reports the StoreKit environment detected on iOS at startup. The environment
/// is fixed by how the binary was distributed (App Store download → production;
/// TestFlight / App Review build → sandbox) and the app cannot choose it, so
/// this is the ground truth for which In-App-Purchase backend the device will
/// hit. Emitted before any IAP flow exists, purely to validate in production
/// that real App Store installs report `production`. See `docs/iap-zone-submission/`.
// Single atomic event carrying BOTH readings of the StoreKit environment so they
// can never arrive asymmetrically: the synchronous receipt read and the
// authoritative `AppTransaction` value, plus how long the latter took and whether
// they agree. One payload = the two readings are always present together, which
// is the whole point (measure the resolve latency, confirm they coincide).
#[derive(Serialize, Clone)]
pub struct SegmentEventIosStoreKitEnvironment {
    // Authoritative environment from AppTransaction (falls back to the receipt
    // read on timeout/error): "production" | "sandbox" | "xcode" | "unknown".
    pub environment: String,
    // Synchronous receipt-URL read taken at startup, for comparison.
    pub environment_sync: String,
    // How `environment` was determined: "app_transaction" (authoritative) |
    // "app_transaction_unverified" | "receipt_fallback" (AppTransaction threw) |
    // "timeout" (AppTransaction didn't resolve within the hard cap).
    pub source: String,
    // Wall-clock milliseconds AppTransaction took to resolve (measured in Swift).
    pub resolve_ms: f64,
    // Milliseconds since app startup when the synchronous receipt read was taken.
    pub environment_sync_at_ms: i64,
    // Milliseconds since app startup when the authoritative environment was
    // confirmed and readable. The gap to environment_sync_at_ms is how long the
    // device went before the authoritative value was available.
    pub environment_at_ms: i64,
    // Whether the authoritative and synchronous readings agree.
    #[serde(rename = "match")]
    pub matched: bool,
    // The app's own backend environment at the time: "org" | "zone" | "today".
    // A mismatch (e.g. StoreKit "sandbox" while app "org") is exactly the
    // App-Review conflict we're characterising.
    pub app_environment: String,
    // StoreKit's AppStore.canMakePayments for this device.
    pub can_make_payments: bool,
}

#[derive(Serialize, Clone)]
pub struct SegmentEventRequestFriend {
    // Wallet address of the user receiving the friend request.
    pub receiver_id: String,
}

#[derive(Serialize, Clone)]
pub struct SegmentEventAcceptFriend {
    // Wallet address of the user whose friend request was accepted.
    pub receiver_id: String,
    // Server-side friendship ID (for metrics mapping).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub friendship_id: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct SegmentEventBlockUser {
    // Wallet address of the user being blocked.
    pub receiver_id: String,
    // Whether the blocked user was a friend at the time of blocking.
    pub is_friend: bool,
}

#[derive(Serialize, Clone)]
pub struct SegmentEventUnfriend {
    // Wallet address of the user being unfriended.
    pub receiver_id: String,
}

#[derive(Serialize, Clone)]
pub struct SegmentEventInstallAttribution {
    // Raw referrer string (e.g. "utm_source=youtube&utm_campaign=xyz")
    pub referrer: String,
    // Parsed UTM source
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utm_source: Option<String>,
    // Parsed UTM medium
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utm_medium: Option<String>,
    // Parsed UTM campaign
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utm_campaign: Option<String>,
    // Parsed UTM content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utm_content: Option<String>,
    // Parsed UTM term
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utm_term: Option<String>,
    // Seconds since epoch when the referrer click happened
    pub click_timestamp: i64,
    // Seconds since epoch when the install began
    pub install_timestamp: i64,
    // Whether the app was launched as a Google Play Instant app
    pub google_play_instant: bool,
}

// Emitted once per attestation cycle attempt (FSM in attestation_service.gd).
// Robust to mid-cycle app kills: each attempt is fired-and-forget, so previous
// attempts survive even if the app dies before the cycle resolves.
//
// Multiple attempts share the same `cycle_id`. A cycle ends when one attempt
// returns outcome="success" OR the user re-launches the app (next launch
// starts a fresh cycle_id). Analysts compute:
//   - success rate    = cycles with any success attempt / distinct cycle_id
//   - retry distribution = count(attempts) per cycle_id
//   - abandonment     = cycle_id with no success and no recent attempt
//   - top failure     = group_by(failure_code) where outcome=failure
#[derive(Serialize, Clone)]
pub struct SegmentEventAttestationAttempt {
    // "ios" | "android"
    pub platform: String,
    // UUID v4, identical across attempts of the same cycle.
    pub cycle_id: String,
    // 1-based attempt counter within the cycle.
    pub attempt_number: u32,
    // What caused the cycle to start: "boot", "force_reattest", "on_demand".
    pub trigger: String,
    // The mobile-bff base URL this attempt actually hit (e.g.
    // "https://mobile-bff.decentraland.org"). Lets analysts segment by
    // backend — useful while staging deploys precede prod (.zone before .org)
    // and to spot leaks of non-prod URLs into release builds.
    pub bff_url: String,
    // "success" | "failure"
    pub outcome: String,
    // Which step failed: "challenge", "generate_key", "attest_key",
    // "play_integrity", "post_session", "plugin_missing", "unsupported".
    // None on success.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_step: Option<String>,
    // Server-side code (e.g. ATTESTATION_IOS_BAD_ASSERTION) or "client_error"
    // for plugin/network failures. None on success.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_code: Option<String>,
    // Total time spent on this attempt, including any plugin/HTTP latencies.
    pub attempt_duration_ms: u32,
    // Per-step durations. Each is only present if the step actually executed:
    // a failure at step 2 will populate step-1 timing and leave the rest null.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub challenge_ms: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generate_key_ms: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attest_key_ms: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub play_integrity_ms: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_session_ms: Option<u32>,
    // TTL of the issued session token (seconds). Only on success.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_ttl_s: Option<i64>,
}

// Emitted once at boot (after _ready loads the on-disk session, before any
// network calls). Measures the cache-hit ratio of the persisted session token.
#[derive(Serialize, Clone)]
pub struct SegmentEventAttestationSessionCacheLoaded {
    pub platform: String,
    // "hit"             — non-expired token loaded, no re-attest needed.
    // "miss_no_file"    — first launch (or post-uninstall).
    // "miss_expired"    — file present but expires_at within EXPIRY_MARGIN_SEC.
    // "miss_corrupted"  — file present but unreadable / unparseable.
    pub result: String,
    // Seconds remaining when result="hit". None for any miss.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining_s: Option<i64>,
}

pub fn build_segment_event_batch_item(
    user_id: String,
    common: &SegmentEventCommonExplorerFields,
    event_data: SegmentEvent,
    // Captured when the event was queued on the device (see Metrics::queue_event), NOT at
    // flush time — this is what gives per-event chronology instead of a batch collapsing to
    // one instant.
    created_at: DateTime<Utc>,
    // Stable per-event id generated at queue time. Reused verbatim across retries so Segment
    // can deduplicate re-sends.
    message_id: String,
) -> SegmentMetricEventBody {
    let (event_name, event_properties, override_position) = match event_data {
        SegmentEvent::PerformanceMetrics(event) => (
            "Performance Metrics".to_string(),
            serde_json::to_value(*event).unwrap(),
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
        SegmentEvent::RequestFriend(event) => (
            "Friend Request".to_string(),
            serde_json::to_value(event).unwrap(),
            None,
        ),
        SegmentEvent::AcceptFriend(event) => (
            "Friend Accept".to_string(),
            serde_json::to_value(event).unwrap(),
            None,
        ),
        SegmentEvent::BlockUser(event) => (
            "Block User".to_string(),
            serde_json::to_value(event).unwrap(),
            None,
        ),
        SegmentEvent::Unfriend(event) => (
            "Unfriend".to_string(),
            serde_json::to_value(event).unwrap(),
            None,
        ),
        SegmentEvent::InstallAttribution(event) => (
            "Install Attribution".to_string(),
            serde_json::to_value(event).unwrap(),
            None,
        ),
        SegmentEvent::FirebaseInit(event) => (
            "Firebase Init".to_string(),
            serde_json::to_value(event).unwrap(),
            None,
        ),
        SegmentEvent::AttestationAttempt(event) => (
            "Attestation Attempt".to_string(),
            serde_json::to_value(event).unwrap(),
            None,
        ),
        SegmentEvent::AttestationSessionCacheLoaded(event) => (
            "Attestation Session Cache Loaded".to_string(),
            serde_json::to_value(event).unwrap(),
            None,
        ),
        SegmentEvent::IosStoreKitEnvironment(event) => (
            "iOS StoreKit Environment".to_string(),
            serde_json::to_value(event).unwrap(),
            None,
        ),
        SegmentEvent::Loading(event) => (
            "Loading Event".to_string(),
            serde_json::to_value(*event).unwrap(),
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

    // ISO-8601 UTC with millisecond precision, e.g. "2026-07-02T13:38:08.243Z".
    let iso_ts = created_at.to_rfc3339_opts(SecondsFormat::Millis, true);

    // Custom, client-owned event time that no Segment clock-skew correction or warehouse
    // mapping can rewrite — the ground-truth chronology. Lives in `properties` next to
    // `renderer_version` so every event carries it, independent of the reserved `timestamp`.
    properties["client_timestamp"] = serde_json::Value::String(iso_ts.clone());

    SegmentMetricEventBody {
        r#type: "track".to_string(),
        event: event_name,
        user_id,
        message_id,
        timestamp: iso_ts,
        properties,
    }
}
