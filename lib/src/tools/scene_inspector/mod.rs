//! Scene Inspector
//!
//! Captures CRDT messages, JS op-calls, lifecycle events, and performance
//! snapshots from Decentraland scenes; also receives inspector commands.
//! Data is dispatched to GDScript via a signal, which then routes to:
//! - WebSocket (preview channel or dedicated target)
//! - JSONL files (optional, when scene-inspector-file is enabled)

pub mod config;
pub mod dispatcher;
pub mod logger;
pub mod storage;

pub use config::SceneInspectorConfig;
pub use dispatcher::SceneInspectorDispatcher;
pub use logger::{
    current_timestamp_ms, CrdtDirection, CrdtLogEntry, CrdtOperation, LogEntry, NetworkEntry,
    OpCallEndEntry, OpCallStartEntry, SceneInspectorEntry, SceneInspectorSender,
    SceneLifecycleEntry, SceneLifecycleEvent, SessionEndEntry, SessionStartEntry,
};
pub use storage::StorageManager;

use std::collections::VecDeque;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Mutex, OnceLock,
};

/// Encodes a byte slice as a lowercase hex string. Uses a pre-allocated buffer
/// with `std::fmt::Write` instead of per-byte `format!()` allocations.
pub fn bytes_to_hex(data: &[u8]) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(data.len() * 2);
    for b in data {
        let _ = write!(s, "{:02x}", b);
    }
    s
}

/// Global sender for Scene Inspector entries. Set once when the
/// SceneInspectorDispatcher is created in DclGlobal. Scene threads clone this
/// sender to push entries.
static SCENE_INSPECTOR_SENDER: OnceLock<SceneInspectorSender> = OnceLock::new();

/// Sets the global Scene Inspector sender. Called once from DclGlobal when the
/// SceneInspectorDispatcher is initialized. Returns Err if already set.
pub fn set_global_sender(sender: SceneInspectorSender) -> Result<(), &'static str> {
    SCENE_INSPECTOR_SENDER
        .set(sender)
        .map_err(|_| "Scene Inspector sender already set")
}

/// Gets a clone of the global Scene Inspector sender.
/// Returns None if the dispatcher has not been initialized.
pub fn get_logger_sender() -> Option<SceneInspectorSender> {
    SCENE_INSPECTOR_SENDER.get().cloned()
}

/// Total entries dropped because the dispatcher channel was full.
/// `try_send` is non-blocking by design (dropping is preferable to blocking
/// scene threads); this counter lets the dispatcher report the loss instead
/// of failing silently.
static DROPPED_COUNT: AtomicU64 = AtomicU64::new(0);

/// Send an entry on the bounded channel. If the channel is full, increment
/// `DROPPED_COUNT` and discard the entry rather than blocking the caller.
pub fn try_send_entry(sender: &SceneInspectorSender, entry: SceneInspectorEntry) {
    if sender.try_send(entry).is_err() {
        DROPPED_COUNT.fetch_add(1, Ordering::Relaxed);
    }
}

/// Atomically read and reset the dropped-entry counter. Called from the
/// dispatcher's periodic perf tick to log how many entries were lost since
/// the previous tick.
pub fn take_dropped_count() -> u64 {
    DROPPED_COUNT.swap(0, Ordering::Relaxed)
}

/// Whether per-tick lifecycle events (`OnUpdate` / `OnUpdateEnd`) are emitted.
/// Defaults to `true` (current behaviour). When disabled, one-shot lifecycle
/// events (init, script loaded, shutdown, …) and CRDT/op-call entries are
/// still emitted — only the 2-per-tick-per-scene firehose is suppressed.
static LIFECYCLE_VERBOSE: AtomicBool = AtomicBool::new(true);

pub fn set_lifecycle_verbose(enabled: bool) {
    LIFECYCLE_VERBOSE.store(enabled, Ordering::Relaxed);
}

pub fn is_lifecycle_verbose() -> bool {
    LIFECYCLE_VERBOSE.load(Ordering::Relaxed)
}

/// When `true`, always attach a hex-encoded copy of the raw proto payload
/// (`bin_payload`) to every CRDT entry. Default `false`: hex is only attached
/// as a fallback when JSON deserialization fails, avoiding the ~2x payload
/// bloat for consumers that only read the decoded JSON.
static INCLUDE_BIN_PAYLOAD: AtomicBool = AtomicBool::new(false);

pub fn set_include_bin_payload(enabled: bool) {
    INCLUDE_BIN_PAYLOAD.store(enabled, Ordering::Relaxed);
}

pub fn is_bin_payload_included() -> bool {
    INCLUDE_BIN_PAYLOAD.load(Ordering::Relaxed)
}

/// Whether a debug consumer is currently connected (the WS bridge flips this on
/// open/close). The master gate for opt-in capture: with no consumer connected,
/// producers must do NOTHING — no buffering "just in case", even if the tool is
/// left enabled in a production build. Keeps prod impact ≈ zero until someone
/// actually connects and subscribes.
static CONSUMER_CONNECTED: AtomicBool = AtomicBool::new(false);

pub fn set_consumer_connected(connected: bool) {
    CONSUMER_CONNECTED.store(connected, Ordering::Relaxed);
}

pub fn is_consumer_connected() -> bool {
    CONSUMER_CONNECTED.load(Ordering::Relaxed)
}

/// Whether captured log lines are folded into the scene-inspector stream as
/// `"log"` entries. Default `false`: opt-in via the `subscribe` command so the
/// log firehose doesn't flood the channel (which CRDT entries share) unless a
/// consumer asks for it.
static STREAM_LOGS: AtomicBool = AtomicBool::new(false);

pub fn set_stream_logs(enabled: bool) {
    STREAM_LOGS.store(enabled, Ordering::Relaxed);
}

pub fn is_stream_logs() -> bool {
    STREAM_LOGS.load(Ordering::Relaxed)
}

/// Whether HTTP observations are folded into the scene-inspector stream as
/// `"network"` entries. Default `false`; opt-in via `subscribe`.
static STREAM_NETWORK: AtomicBool = AtomicBool::new(false);

pub fn set_stream_network(enabled: bool) {
    STREAM_NETWORK.store(enabled, Ordering::Relaxed);
}

pub fn is_stream_network() -> bool {
    STREAM_NETWORK.load(Ordering::Relaxed)
}

/// Whether boot-time log lines are buffered into a bounded ring until a consumer
/// subscribes — so the most valuable startup logs (which happen in the window
/// between app launch and the first `subscribe`) aren't lost.
///
/// ARMED ONLY in debug builds with a scene-inspector target configured (the
/// bridge sets it at startup). NEVER armed in production: there, with no
/// connection, `emit_log` short-circuits and buffers nothing — honoring the
/// "no moving logs into a buffer without a connection" contract.
static EARLY_LOG_ARMED: AtomicBool = AtomicBool::new(false);

/// Max boot log lines held before a consumer subscribes. Bounded so an armed-but-
/// never-subscribed debug session can't grow without limit; oldest dropped first.
const EARLY_LOG_RING_CAP: usize = 4096;

/// Bounded FIFO of pre-subscribe log entries. Only ever touched in debug builds
/// (when armed). Lock contention is negligible: log volume is low and this is the
/// cold path (the live path doesn't lock).
static EARLY_LOG_RING: Mutex<VecDeque<LogEntry>> = Mutex::new(VecDeque::new());

/// Boot logs dropped from the ring because it was full (reported on flush).
static EARLY_LOG_DROPPED: AtomicU64 = AtomicU64::new(0);

/// Arm or disarm the bounded boot-log ring. Disarming clears whatever was held.
pub fn set_early_log_capture(enabled: bool) {
    EARLY_LOG_ARMED.store(enabled, Ordering::Relaxed);
    if !enabled {
        if let Ok(mut ring) = EARLY_LOG_RING.lock() {
            ring.clear();
        }
    }
}

pub fn is_early_log_armed() -> bool {
    EARLY_LOG_ARMED.load(Ordering::Relaxed)
}

/// Push a log entry into the bounded boot ring, dropping the oldest if full.
fn push_early_log(entry: LogEntry) {
    if let Ok(mut ring) = EARLY_LOG_RING.lock() {
        if ring.len() >= EARLY_LOG_RING_CAP {
            ring.pop_front();
            EARLY_LOG_DROPPED.fetch_add(1, Ordering::Relaxed);
        }
        ring.push_back(entry);
    }
}

/// Drain the boot-log ring into the live channel. Called once a consumer
/// subscribes to `log` (which implies the WS is connected), so the buffered
/// startup lines are delivered before live ones. No-op when empty / never armed.
pub fn flush_early_logs() {
    let drained: Vec<LogEntry> = match EARLY_LOG_RING.lock() {
        Ok(mut ring) => ring.drain(..).collect(),
        Err(_) => return,
    };
    if drained.is_empty() {
        return;
    }
    if let Some(sender) = get_logger_sender() {
        // Surface any ring overflow so a flooded boot window isn't silently lossy.
        let dropped = EARLY_LOG_DROPPED.swap(0, Ordering::Relaxed);
        if dropped > 0 {
            try_send_entry(
                &sender,
                SceneInspectorEntry::Log(LogEntry {
                    timestamp_ms: current_timestamp_ms(),
                    source: "scene_inspector".to_string(),
                    level: Some("warn".to_string()),
                    target: None,
                    file: None,
                    line: None,
                    msg: format!("early-log ring overflowed: {dropped} boot lines dropped"),
                }),
            );
        }
        for entry in drained {
            try_send_entry(&sender, SceneInspectorEntry::Log(entry));
        }
    }
}

/// Fold a captured log line into the scene-inspector stream. No-op unless log
/// streaming is enabled (via `subscribe`) AND the dispatcher is initialized.
///
/// MUST NOT log via `tracing` / Godot — it is called from inside the log sinks
/// (tracing layer, Godot logger, fd capture) and would recurse.
#[allow(clippy::too_many_arguments)]
pub fn emit_log(
    source: &str,
    level: Option<&str>,
    target: Option<&str>,
    file: Option<&str>,
    line: Option<u32>,
    msg: String,
) {
    let streaming = is_consumer_connected() && is_stream_logs();
    // Prod fast path: not streaming live AND not armed for boot capture → do
    // nothing, buffer nothing. `EARLY_LOG_ARMED` is never set in production, so
    // this is just two relaxed atomic loads and a branch — impact ≈ zero.
    if !streaming && !is_early_log_armed() {
        return;
    }
    let entry = LogEntry {
        timestamp_ms: current_timestamp_ms(),
        source: source.to_string(),
        level: level.map(str::to_string),
        target: target.map(str::to_string),
        file: file.map(str::to_string),
        line,
        msg,
    };
    if streaming {
        if let Some(sender) = get_logger_sender() {
            try_send_entry(&sender, SceneInspectorEntry::Log(entry));
        }
    } else {
        // Armed but no consumer subscribed yet: hold in the bounded boot ring,
        // flushed on the first `subscribe` (debug builds only).
        push_early_log(entry);
    }
}

/// Fold an HTTP observation into the scene-inspector stream. No-op unless
/// network streaming is enabled (via `subscribe`) AND the dispatcher exists.
pub fn emit_network(entry: NetworkEntry) {
    if !is_consumer_connected() || !is_stream_network() {
        return;
    }
    if let Some(sender) = get_logger_sender() {
        try_send_entry(&sender, SceneInspectorEntry::Network(entry));
    }
}

/// Logs a scene lifecycle event. No-op if the Scene Inspector is not
/// initialized. Per-tick events (`OnUpdate` / `OnUpdateEnd`) are additionally
/// gated by `LIFECYCLE_VERBOSE` so they can be suppressed without affecting
/// CRDT/op logging. One-shot events are always emitted while the inspector is
/// on.
pub fn log_lifecycle_event(
    scene_id: i32,
    event: SceneLifecycleEvent,
    tick: Option<u32>,
    delta_time: Option<f64>,
    error: Option<String>,
) {
    if matches!(
        event,
        SceneLifecycleEvent::OnUpdate | SceneLifecycleEvent::OnUpdateEnd
    ) && !is_lifecycle_verbose()
    {
        return;
    }

    if let Some(sender) = get_logger_sender() {
        let entry = SceneLifecycleEntry {
            scene_id,
            timestamp_ms: current_timestamp_ms(),
            event,
            tick,
            delta_time,
            error,
            title: None,
            base_parcel: None,
        };
        try_send_entry(&sender, SceneInspectorEntry::SceneLifecycle(entry));
    }
}

/// Logs a scene init event with title and base parcel metadata.
pub fn log_scene_init_event(scene_id: i32, title: Option<String>, base_parcel: Option<String>) {
    if let Some(sender) = get_logger_sender() {
        let entry = SceneLifecycleEntry {
            scene_id,
            timestamp_ms: current_timestamp_ms(),
            event: SceneLifecycleEvent::SceneInit,
            tick: None,
            delta_time: None,
            error: None,
            title,
            base_parcel,
        };
        try_send_entry(&sender, SceneInspectorEntry::SceneLifecycle(entry));
    }
}

/// Logs a CRDT operation from renderer to scene.
pub fn log_crdt_renderer_to_scene(
    scene_id: i32,
    tick: u32,
    entity_id: u32,
    component_id: u32,
    operation: CrdtOperation,
    crdt_timestamp: u32,
    payload_data: Option<&[u8]>,
) {
    use crate::dcl::components::{
        component_id_to_name, proto_components::deserialize_component_to_json,
    };

    if let Some(sender) = get_logger_sender() {
        let payload =
            payload_data.and_then(|data| deserialize_component_to_json(component_id, data));
        let bin_payload = payload_data
            .filter(|_| payload.is_none() || is_bin_payload_included())
            .map(bytes_to_hex);

        let entry = CrdtLogEntry {
            scene_id,
            tick,
            timestamp_ms: current_timestamp_ms(),
            direction: CrdtDirection::RendererToScene,
            entity_id,
            component_name: std::borrow::Cow::Borrowed(component_id_to_name(component_id)),
            operation,
            crdt_timestamp,
            payload,
            bin_payload,
            raw_size_bytes: payload_data.map(|d| d.len()).unwrap_or(0),
        };
        try_send_entry(&sender, SceneInspectorEntry::CrdtMessage(entry));
    }
}
