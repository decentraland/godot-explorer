//! Scene Logging System
//!
//! Captures CRDT messages, JS op-calls, and lifecycle events from Decentraland scenes.
//! Data is dispatched to GDScript via a signal, which then routes to:
//! - WebSocket (preview channel or dedicated target)
//! - Godot Editor Debugger (EngineDebugger)
//! - JSONL files (optional, when scene-logging-file is enabled)

pub mod config;
pub mod dispatcher;
pub mod logger;
pub mod storage;

pub use config::SceneLoggingConfig;
pub use dispatcher::SceneLogDispatcher;
pub use logger::{
    current_timestamp_ms, CrdtDirection, CrdtLogEntry, CrdtOperation, OpCallEndEntry,
    OpCallStartEntry, SceneLifecycleEntry, SceneLifecycleEvent, SceneLogEntry, SceneLoggerSender,
    SessionEndEntry, SessionStartEntry,
};
pub use storage::StorageManager;

use once_cell::sync::OnceCell;

/// Global sender for scene log entries. Set once when the SceneLogDispatcher is
/// created in DclGlobal. Scene threads clone this sender to push entries.
static SCENE_LOG_SENDER: OnceCell<SceneLoggerSender> = OnceCell::new();

/// Sets the global scene log sender. Called once from DclGlobal when the
/// SceneLogDispatcher is initialized. Returns Err if already set.
pub fn set_global_sender(sender: SceneLoggerSender) -> Result<(), &'static str> {
    SCENE_LOG_SENDER
        .set(sender)
        .map_err(|_| "Scene log sender already set")
}

/// Gets a clone of the global scene log sender.
/// Returns None if the dispatcher has not been initialized.
pub fn get_logger_sender() -> Option<SceneLoggerSender> {
    SCENE_LOG_SENDER.get().cloned()
}

/// Logs a scene lifecycle event. No-op if logging is not initialized.
pub fn log_lifecycle_event(
    scene_id: i32,
    event: SceneLifecycleEvent,
    tick: Option<u32>,
    delta_time: Option<f64>,
    error: Option<String>,
) {
    if let Some(sender) = get_logger_sender() {
        let entry = SceneLifecycleEntry {
            scene_id,
            timestamp_ms: current_timestamp_ms(),
            event,
            tick,
            delta_time,
            error,
        };
        let _ = sender.try_send(SceneLogEntry::SceneLifecycle(entry));
    }
}

/// Logs a CRDT operation from renderer to scene.
pub fn log_crdt_renderer_to_scene(
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
        let bin_payload = payload_data.map(|data| {
            data.iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>()
        });

        let entry = CrdtLogEntry {
            tick,
            timestamp_ms: current_timestamp_ms(),
            direction: CrdtDirection::RendererToScene,
            entity_id,
            component_name: component_id_to_name(component_id).to_string(),
            operation,
            crdt_timestamp,
            payload,
            bin_payload,
            raw_size_bytes: payload_data.map(|d| d.len()).unwrap_or(0),
        };
        let _ = sender.try_send(SceneLogEntry::CrdtMessage(entry));
    }
}
