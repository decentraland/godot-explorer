//! Scene Logging System
//!
//! A comprehensive logging and debugging tool for Decentraland scenes that captures:
//! - CRDT messages (component updates, entity lifecycle)
//! - Op calls (JS -> Rust runtime calls via Deno.core.ops)
//!
//! Data is stored in JSONL format and served via a web UI for visualization.
//!
//! # Usage
//!
//! Enable the `scene_logging` feature in Cargo.toml:
//! ```toml
//! [features]
//! scene_logging = ["dep:axum", "dep:tower-http"]
//! ```
//!
//! Then build with the feature enabled:
//! ```bash
//! cargo run -- run --features scene_logging
//! ```

mod config;
mod logger;
mod server;
mod storage;

pub use config::SceneLoggingConfig;
pub use logger::{
    current_timestamp_ms, CrdtDirection, CrdtLogEntry, CrdtOperation, OpCallEndEntry,
    OpCallStartEntry, SceneLifecycleEntry, SceneLifecycleEvent, SceneLogEntry, SceneLogger,
    SceneLoggerSender, SessionEndEntry, SessionStartEntry,
};
pub use server::start_server;
pub use storage::StorageManager;

use once_cell::sync::OnceCell;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Global scene logger instance.
static SCENE_LOGGER: OnceCell<Arc<SceneLogger>> = OnceCell::new();

/// Initializes the global scene logger with the given configuration.
///
/// Returns an error if the logger is already initialized.
pub fn init_global_logger(config: SceneLoggingConfig) -> Result<(), &'static str> {
    let logger = SceneLogger::new(config);
    SCENE_LOGGER
        .set(Arc::new(logger))
        .map_err(|_| "Scene logger already initialized")
}

/// Gets a reference to the global scene logger.
///
/// Returns None if the logger has not been initialized.
pub fn get_global_logger() -> Option<Arc<SceneLogger>> {
    SCENE_LOGGER.get().cloned()
}

/// Gets a sender for the global scene logger.
///
/// Returns None if the logger has not been initialized.
pub fn get_logger_sender() -> Option<SceneLoggerSender> {
    SCENE_LOGGER.get().map(|l| l.sender())
}

/// Statistics for the logging session.
#[derive(Default, Debug, Clone)]
pub struct LoggingStats {
    pub total_crdt_messages: u64,
    pub total_op_calls: u64,
    pub bytes_written: u64,
}

/// Global statistics instance.
static LOGGING_STATS: OnceCell<Arc<RwLock<LoggingStats>>> = OnceCell::new();

/// Gets the global logging statistics.
pub fn get_stats() -> Arc<RwLock<LoggingStats>> {
    LOGGING_STATS
        .get_or_init(|| Arc::new(RwLock::new(LoggingStats::default())))
        .clone()
}

/// Logs a scene lifecycle event. This is a convenience function that handles
/// the case where logging is not initialized.
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
/// This is used when the renderer sends dirty CRDT state back to the scene.
pub fn log_crdt_renderer_to_scene(
    tick: u32,
    entity_id: u32,
    component_id: u32,
    operation: CrdtOperation,
    crdt_timestamp: u32,
    payload_data: Option<&[u8]>,
) {
    use crate::dcl::components::{component_id_to_name, proto_components::deserialize_component_to_json};

    if let Some(sender) = get_logger_sender() {
        let payload = payload_data.and_then(|data| deserialize_component_to_json(component_id, data));
        // Encode as hex string
        let bin_payload = payload_data.map(|data| {
            data.iter().map(|b| format!("{:02x}", b)).collect::<String>()
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
