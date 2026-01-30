//! Core async logger with mpsc channel for non-blocking logging.

use super::{config::SceneLoggingConfig, get_stats, storage::StorageManager};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

/// Channel capacity for log entries. Uses bounded channel with backpressure.
const CHANNEL_CAPACITY: usize = 10_000;

/// Batch size for flushing entries to disk.
const BATCH_FLUSH_SIZE: usize = 1000;

/// Maximum time between flushes in milliseconds.
const FLUSH_INTERVAL_MS: u64 = 100;

/// A log entry that can be a CRDT message, op call start/end, or session marker.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SceneLogEntry {
    #[serde(rename = "crdt")]
    CrdtMessage(CrdtLogEntry),
    #[serde(rename = "op_call_start")]
    OpCallStart(OpCallStartEntry),
    #[serde(rename = "op_call_end")]
    OpCallEnd(OpCallEndEntry),
    #[serde(rename = "scene_lifecycle")]
    SceneLifecycle(SceneLifecycleEntry),
    #[serde(rename = "session_start")]
    SessionStart(SessionStartEntry),
    #[serde(rename = "session_end")]
    SessionEnd(SessionEndEntry),
}

/// CRDT operation type.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CrdtOperation {
    #[serde(rename = "p")]
    Put,
    #[serde(rename = "d")]
    Delete,
    #[serde(rename = "de")]
    DeleteEntity,
    #[serde(rename = "a")]
    Append,
}

impl std::fmt::Display for CrdtOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CrdtOperation::Put => write!(f, "put"),
            CrdtOperation::Delete => write!(f, "delete"),
            CrdtOperation::DeleteEntity => write!(f, "delete_entity"),
            CrdtOperation::Append => write!(f, "append"),
        }
    }
}

/// Direction of CRDT message flow.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CrdtDirection {
    /// Scene (JS) sending to Renderer (Godot).
    #[serde(rename = "s2r")]
    SceneToRenderer,
    /// Renderer (Godot) sending to Scene (JS).
    #[serde(rename = "r2s")]
    RendererToScene,
}

/// A logged CRDT message with compact field names.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrdtLogEntry {
    /// Tick number when this message was processed.
    #[serde(rename = "tk")]
    pub tick: u32,
    /// Timestamp in milliseconds since epoch.
    #[serde(rename = "t")]
    pub timestamp_ms: u64,
    /// Direction of the message.
    #[serde(rename = "d")]
    pub direction: CrdtDirection,
    /// Entity ID (combined number and version as u32).
    #[serde(rename = "e")]
    pub entity_id: u32,
    /// Human-readable component name.
    #[serde(rename = "c")]
    pub component_name: String,
    /// Type of CRDT operation.
    #[serde(rename = "op")]
    pub operation: CrdtOperation,
    /// CRDT timestamp from the message.
    #[serde(rename = "ct")]
    pub crdt_timestamp: u32,
    /// Deserialized component payload (JSON, human-readable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
    /// Raw binary payload (base64 encoded, for export/replay).
    #[serde(rename = "bin", skip_serializing_if = "Option::is_none")]
    pub bin_payload: Option<String>,
    /// Size of the raw binary data in bytes.
    #[serde(rename = "l")]
    pub raw_size_bytes: usize,
}

/// A logged Deno op call start (JS -> Rust runtime call).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpCallStartEntry {
    /// Unique call ID (used to correlate with OpCallEndEntry).
    pub call_id: u64,
    /// Scene ID that made this call.
    pub scene_id: i32,
    /// Timestamp when the call was made (ms since epoch).
    pub timestamp_ms: u64,
    /// Name of the op (e.g., "op_fetch_custom", "op_crdt_send_to_renderer").
    pub op_name: String,
    /// Arguments passed to the op (JSON serialized).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<serde_json::Value>,
}

/// A logged Deno op call end (result of JS -> Rust runtime call).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpCallEndEntry {
    /// Unique call ID (correlates with OpCallStartEntry).
    pub call_id: u64,
    /// Scene ID that made this call.
    pub scene_id: i32,
    /// Timestamp when the call completed (ms since epoch).
    pub timestamp_ms: u64,
    /// Name of the op (e.g., "op_fetch_custom", "op_crdt_send_to_renderer").
    pub op_name: String,
    /// Return value from the op (JSON serialized).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Whether the call was async (returned a Promise).
    pub is_async: bool,
    /// Duration of the call in milliseconds.
    pub duration_ms: f64,
    /// Error message if the call failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Session start marker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStartEntry {
    /// Unique session ID.
    pub session_id: String,
    /// Timestamp when the session started.
    pub timestamp_ms: u64,
    /// Version of the client.
    pub version: String,
    /// Platform (linux, windows, macos, android, ios).
    pub platform: String,
}

/// Session end marker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEndEntry {
    /// Session ID.
    pub session_id: String,
    /// Timestamp when the session ended.
    pub timestamp_ms: u64,
    /// Total CRDT messages logged in this session.
    pub total_crdt_messages: u64,
    /// Total op calls logged in this session.
    pub total_op_calls: u64,
}

/// Scene lifecycle event types.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SceneLifecycleEvent {
    /// Scene thread initialized, before loading any JS.
    SceneInit,
    /// main.crdt file loaded and processed.
    MainCrdtLoaded,
    /// main.js script loaded and executed.
    ScriptLoaded,
    /// onStart function called.
    OnStart,
    /// onStart function completed.
    OnStartEnd,
    /// onUpdate function called (each tick).
    OnUpdate,
    /// onUpdate function completed.
    OnUpdateEnd,
    /// Scene is shutting down.
    SceneShutdown,
}

/// A scene lifecycle event entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneLifecycleEntry {
    /// Scene ID.
    pub scene_id: i32,
    /// Timestamp in milliseconds since epoch.
    pub timestamp_ms: u64,
    /// The lifecycle event.
    pub event: SceneLifecycleEvent,
    /// Current tick number (for onUpdate events).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tick: Option<u32>,
    /// Delta time in seconds (for onUpdate events).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta_time: Option<f64>,
    /// Error message if the lifecycle event failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Sender half of the scene logger channel.
pub type SceneLoggerSender = mpsc::Sender<SceneLogEntry>;

/// Receiver half of the scene logger channel.
pub type SceneLoggerReceiver = mpsc::Receiver<SceneLogEntry>;

/// The main scene logger that manages async logging.
pub struct SceneLogger {
    config: SceneLoggingConfig,
    sender: SceneLoggerSender,
    session_id: String,
}

impl SceneLogger {
    /// Creates a new scene logger and spawns background tasks in a dedicated thread.
    pub fn new(config: SceneLoggingConfig) -> Self {
        let (sender, receiver) = mpsc::channel(CHANNEL_CAPACITY);
        let session_id = uuid::Uuid::new_v4().to_string();

        // Spawn a dedicated thread with its own tokio runtime for logging
        let writer_config = config.clone();
        let writer_session_id = session_id.clone();

        std::thread::Builder::new()
            .name("scene-logger".to_string())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to create scene logger runtime");

                rt.block_on(async move {
                    // Run the log writer task
                    log_writer_task(receiver, writer_config, writer_session_id).await;
                });
            })
            .expect("Failed to spawn scene logger thread");

        Self {
            config,
            sender,
            session_id,
        }
    }

    /// Gets a clone of the sender for this logger.
    pub fn sender(&self) -> SceneLoggerSender {
        self.sender.clone()
    }

    /// Gets the session ID for this logger.
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Gets the configuration for this logger.
    pub fn config(&self) -> &SceneLoggingConfig {
        &self.config
    }

    /// Logs a CRDT message. This method never blocks.
    pub fn log_crdt(&self, entry: CrdtLogEntry) {
        let _ = self.sender.try_send(SceneLogEntry::CrdtMessage(entry));
    }

    /// Logs an op call start. This method never blocks.
    pub fn log_op_start(&self, entry: OpCallStartEntry) {
        let _ = self.sender.try_send(SceneLogEntry::OpCallStart(entry));
    }

    /// Logs an op call end. This method never blocks.
    pub fn log_op_end(&self, entry: OpCallEndEntry) {
        let _ = self.sender.try_send(SceneLogEntry::OpCallEnd(entry));
    }
}

/// Background task that writes log entries to disk.
async fn log_writer_task(
    mut receiver: SceneLoggerReceiver,
    config: SceneLoggingConfig,
    session_id: String,
) {
    let mut storage = match StorageManager::new(config.clone(), session_id.clone()) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to create storage manager: {}", e);
            return;
        }
    };

    // Write session start marker
    let start_entry = SceneLogEntry::SessionStart(SessionStartEntry {
        session_id: session_id.clone(),
        timestamp_ms: current_timestamp_ms(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        platform: std::env::consts::OS.to_string(),
    });
    if let Err(e) = storage.write_entry(&start_entry) {
        tracing::error!("Failed to write session start: {}", e);
    }

    let mut entries_since_flush = 0;
    let mut crdt_count: u64 = 0;
    let mut op_count: u64 = 0;

    let flush_interval = tokio::time::Duration::from_millis(FLUSH_INTERVAL_MS);
    let mut flush_timer = tokio::time::interval(flush_interval);

    loop {
        tokio::select! {
            entry = receiver.recv() => {
                match entry {
                    Some(entry) => {
                        // Update stats
                        match &entry {
                            SceneLogEntry::CrdtMessage(_) => crdt_count += 1,
                            SceneLogEntry::OpCallStart(_) | SceneLogEntry::OpCallEnd(_) => op_count += 1,
                            _ => {}
                        }

                        // Write entry
                        if let Err(e) = storage.write_entry(&entry) {
                            tracing::error!("Failed to write log entry: {}", e);
                        }

                        entries_since_flush += 1;

                        // Batch flush
                        if entries_since_flush >= BATCH_FLUSH_SIZE {
                            if let Err(e) = storage.flush() {
                                tracing::error!("Failed to flush log entries: {}", e);
                            }
                            entries_since_flush = 0;

                            // Update global stats
                            if let Ok(mut stats) = get_stats().try_write() {
                                stats.total_crdt_messages = crdt_count;
                                stats.total_op_calls = op_count;
                            }
                        }
                    }
                    None => {
                        // Channel closed, write session end and exit
                        let end_entry = SceneLogEntry::SessionEnd(SessionEndEntry {
                            session_id,
                            timestamp_ms: current_timestamp_ms(),
                            total_crdt_messages: crdt_count,
                            total_op_calls: op_count,
                        });
                        let _ = storage.write_entry(&end_entry);
                        let _ = storage.flush();

                        // Update final stats
                        if let Ok(mut stats) = get_stats().try_write() {
                            stats.total_crdt_messages = crdt_count;
                            stats.total_op_calls = op_count;
                        }

                        tracing::info!(
                            "Scene logging session ended. CRDT: {}, Op calls: {}",
                            crdt_count,
                            op_count
                        );
                        return;
                    }
                }
            }
            _ = flush_timer.tick() => {
                // Periodic flush
                if entries_since_flush > 0 {
                    if let Err(e) = storage.flush() {
                        tracing::error!("Failed to flush log entries: {}", e);
                    }
                    entries_since_flush = 0;
                }
            }
        }
    }
}

/// Gets the current timestamp in milliseconds since epoch.
pub fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
