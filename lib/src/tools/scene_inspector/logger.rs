//! Scene Inspector entry types and sender/receiver type aliases.

use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use tokio::sync::mpsc;

/// A Scene Inspector entry: CRDT message, op call start/end, lifecycle, session
/// marker, or performance snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SceneInspectorEntry {
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
    /// A captured log line (Rust / GDScript+engine / native Swift+ObjC), folded
    /// in from the log capture sinks. Additive: external consumers that
    /// don't know this `type` ignore it.
    #[serde(rename = "log")]
    Log(LogEntry),
    /// An HTTP request/response observation, folded in from the network
    /// inspector. Additive, like `log`.
    #[serde(rename = "network")]
    Network(NetworkEntry),
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
    /// Borrowed `&'static str` for proto components (avoids per-message
    /// allocation in the scene-thread hot path); falls back to `String` only
    /// when serde deserialization needs ownership.
    #[serde(rename = "c")]
    pub component_name: Cow<'static, str>,
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
    /// `u32` to match the JS-side `nextCallId` counter, which starts at 1 and
    /// is bounded well below 2^32; using `u64` would silently lose precision
    /// once it crossed 2^53 (since JS `Number` is `f64`).
    pub call_id: u32,
    pub scene_id: i32,
    pub timestamp_ms: u64,
    pub op_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<serde_json::Value>,
}

/// A logged Deno op call end.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpCallEndEntry {
    /// See `OpCallStartEntry::call_id`.
    pub call_id: u32,
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

/// A captured log line, folded into the scene-inspector stream as a `"log"`
/// entry. `source` is the origin: `"rust"` (tracing), `"godot"` (GDScript /
/// engine messages), or `"native"` (Swift/ObjC stdout/stderr on iOS).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    #[serde(rename = "t")]
    pub timestamp_ms: u64,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    pub msg: String,
}

/// An HTTP request/response observation, folded into the scene-inspector stream
/// as a `"network"` entry. `phase` mirrors the `NetworkInspectEvent` lifecycle:
/// `"request"`, `"partial_response"`, `"body_response"`, or `"full_response"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkEntry {
    #[serde(rename = "t")]
    pub timestamp_ms: u64,
    pub id: u32,
    pub phase: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requester: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ok: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Sender half of the Scene Inspector channel.
pub type SceneInspectorSender = mpsc::Sender<SceneInspectorEntry>;

/// Gets the current timestamp in milliseconds since epoch.
pub fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    // The scene_inspector envelope is consumed by an external app; new entry
    // types must be ADDITIVE — a `type` discriminator + only-present-when-set
    // optional fields — so old consumers keep parsing. These tests pin that shape.

    #[test]
    fn log_entry_has_type_tag_and_fields() {
        let entry = SceneInspectorEntry::Log(LogEntry {
            timestamp_ms: 123,
            source: "rust".into(),
            level: Some("warn".into()),
            target: Some("dclgodot::comms".into()),
            file: Some("comms.rs".into()),
            line: Some(42),
            msg: "hello".into(),
        });
        let v = serde_json::to_value(&entry).unwrap();
        assert_eq!(v["type"], "log");
        assert_eq!(v["t"], 123);
        assert_eq!(v["source"], "rust");
        assert_eq!(v["level"], "warn");
        assert_eq!(v["line"], 42);
        assert_eq!(v["msg"], "hello");
    }

    #[test]
    fn log_entry_omits_unset_optionals() {
        let entry = SceneInspectorEntry::Log(LogEntry {
            timestamp_ms: 1,
            source: "native".into(),
            level: None,
            target: None,
            file: None,
            line: None,
            msg: "x".into(),
        });
        let v = serde_json::to_value(&entry).unwrap();
        let obj = v.as_object().unwrap();
        for k in ["level", "target", "file", "line"] {
            assert!(!obj.contains_key(k), "expected `{k}` to be omitted");
        }
        assert_eq!(v["type"], "log");
        assert_eq!(v["source"], "native");
    }

    #[test]
    fn network_entry_has_type_tag_and_fields() {
        let entry = SceneInspectorEntry::Network(NetworkEntry {
            timestamp_ms: 7,
            id: 99,
            phase: "full_response".into(),
            url: Some("https://example.org".into()),
            method: Some("GET".into()),
            requester: None,
            status: Some(200),
            ok: Some(true),
            error: None,
        });
        let v = serde_json::to_value(&entry).unwrap();
        assert_eq!(v["type"], "network");
        assert_eq!(v["id"], 99);
        assert_eq!(v["phase"], "full_response");
        assert_eq!(v["status"], 200);
        assert_eq!(v["ok"], true);
        let obj = v.as_object().unwrap();
        assert!(!obj.contains_key("requester"));
        assert!(!obj.contains_key("error"));
    }
}
