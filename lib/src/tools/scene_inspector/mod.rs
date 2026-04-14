//! Scene Inspector
//!
//! Captures CRDT messages, JS op-calls, lifecycle events, and performance
//! snapshots from Decentraland scenes; also receives inspector commands.
//! Data is dispatched to GDScript via a signal, which then routes to:
//! - WebSocket (preview channel or dedicated target)
//! - Godot Editor Debugger (EngineDebugger)
//! - JSONL files (optional, when scene-inspector-file is enabled)

pub mod config;
pub mod dispatcher;
pub mod logger;
pub mod storage;

pub use config::SceneInspectorConfig;
pub use dispatcher::SceneInspectorDispatcher;
pub use logger::{
    current_timestamp_ms, CrdtDirection, CrdtLogEntry, CrdtOperation, OpCallEndEntry,
    OpCallStartEntry, SceneInspectorEntry, SceneInspectorSender, SceneLifecycleEntry,
    SceneLifecycleEvent, SessionEndEntry, SessionStartEntry,
};
pub use storage::StorageManager;

use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    OnceLock,
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
        let bin_payload = payload_data.map(bytes_to_hex);

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
