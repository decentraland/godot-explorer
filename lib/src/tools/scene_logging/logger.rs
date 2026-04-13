//! Scene log entry types and sender/receiver type aliases.

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

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
    #[serde(rename = "perf")]
    PerformanceSnapshot(PerformanceSnapshotEntry),
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
    #[serde(rename = "sid")]
    pub scene_id: i32,
    #[serde(rename = "tk")]
    pub tick: u32,
    #[serde(rename = "t")]
    pub timestamp_ms: u64,
    #[serde(rename = "d")]
    pub direction: CrdtDirection,
    #[serde(rename = "e")]
    pub entity_id: u32,
    #[serde(rename = "c")]
    pub component_name: String,
    #[serde(rename = "op")]
    pub operation: CrdtOperation,
    #[serde(rename = "ct")]
    pub crdt_timestamp: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
    #[serde(rename = "bin", skip_serializing_if = "Option::is_none")]
    pub bin_payload: Option<String>,
    #[serde(rename = "l")]
    pub raw_size_bytes: usize,
}

/// A logged Deno op call start.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpCallStartEntry {
    pub call_id: u64,
    pub scene_id: i32,
    pub timestamp_ms: u64,
    pub op_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<serde_json::Value>,
}

/// A logged Deno op call end.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpCallEndEntry {
    pub call_id: u64,
    pub scene_id: i32,
    pub timestamp_ms: u64,
    pub op_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    pub is_async: bool,
    pub duration_ms: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Session start marker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStartEntry {
    pub session_id: String,
    pub timestamp_ms: u64,
    pub version: String,
    pub platform: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_name: Option<String>,
}

/// Session end marker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEndEntry {
    pub session_id: String,
    pub timestamp_ms: u64,
    pub total_crdt_messages: u64,
    pub total_op_calls: u64,
}

/// Scene lifecycle event types.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SceneLifecycleEvent {
    SceneInit,
    MainCrdtLoaded,
    ScriptLoaded,
    OnStart,
    OnStartEnd,
    OnUpdate,
    OnUpdateEnd,
    SceneShutdown,
}

/// A scene lifecycle event entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneLifecycleEntry {
    pub scene_id: i32,
    pub timestamp_ms: u64,
    pub event: SceneLifecycleEvent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tick: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta_time: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_parcel: Option<String>,
}

/// A performance snapshot with rendering, memory, and asset metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSnapshotEntry {
    #[serde(rename = "t")]
    pub timestamp_ms: u64,
    pub fps: f64,
    pub dt: f64,
    pub draw_calls: i64,
    pub primitives: i64,
    pub objects_in_frame: i64,
    pub mem_static_mb: f64,
    pub mem_gpu_mb: f64,
    pub mem_rust_mb: f64,
    pub js_heap_total_mb: f64,
    pub js_heap_used_mb: f64,
    pub js_heap_limit_mb: f64,
    pub js_external_mb: f64,
    pub assets_loading: u64,
    pub assets_loaded: u64,
    pub download_speed_mbs: f64,
    pub scene_count: i32,
}

/// Sender half of the scene logger channel.
pub type SceneLoggerSender = mpsc::Sender<SceneLogEntry>;

/// Gets the current timestamp in milliseconds since epoch.
pub fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
