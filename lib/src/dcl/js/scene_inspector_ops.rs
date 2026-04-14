//! Deno operations for the Scene Inspector from JavaScript.
//!
//! These ops allow JavaScript code to report op calls and other events
//! to the Scene Inspector by wrapping Deno.core.ops with a Proxy.

use std::cell::RefCell;
use std::rc::Rc;

use deno_core::{op2, OpDecl, OpState};
use serde::Deserialize;

use crate::tools::scene_inspector::{
    current_timestamp_ms, get_logger_sender, try_send_entry, OpCallEndEntry, OpCallStartEntry,
    SceneInspectorEntry,
};

/// Per-scene debug flag, inserted into the Deno `OpState` at scene boot from
/// `SpawnDclSceneData::should_debug`. When `false`, the JS `setupOpLogging`
/// wrapper bails out and CRDT/lifecycle hooks short-circuit, so non-debugged
/// scenes pay zero per-call overhead.
pub struct SceneDebugFlag(pub bool);

pub fn ops() -> Vec<OpDecl> {
    vec![
        op_scene_debug_enabled(),
        op_scene_inspector_op_start(),
        op_scene_inspector_op_end(),
    ]
}

/// Returns whether the current scene was spawned with `--scene-debug` enabled.
/// Called once from `main.js` during `setupOpLogging` so the JS wrapper knows
/// whether to install itself.
#[op2(fast)]
fn op_scene_debug_enabled(state: &mut OpState) -> bool {
    state
        .try_borrow::<SceneDebugFlag>()
        .map(|f| f.0)
        .unwrap_or(false)
}

/// Op call start data received from JavaScript.
#[derive(Debug, Deserialize)]
pub struct JsOpCallStartData {
    /// Unique call ID for correlation. `u32` because JS `Number` is `f64` —
    /// `u64` would silently lose precision once `nextCallId` crosses 2^53.
    pub call_id: u32,
    /// Name of the op (e.g., "op_fetch_custom").
    pub op_name: String,
    /// Arguments passed to the op (JSON value).
    #[serde(default)]
    pub args: Option<serde_json::Value>,
}

/// Op call end data received from JavaScript.
#[derive(Debug, Deserialize)]
pub struct JsOpCallEndData {
    /// Unique call ID for correlation. See `JsOpCallStartData::call_id`.
    pub call_id: u32,
    /// Name of the op (e.g., "op_fetch_custom").
    pub op_name: String,
    /// Return value from the op (JSON value).
    #[serde(default)]
    pub result: Option<serde_json::Value>,
    /// Whether the call was async (Promise).
    #[serde(default)]
    pub is_async: bool,
    /// Duration in milliseconds.
    #[serde(default)]
    pub duration_ms: f64,
    /// Error message if the call failed.
    #[serde(default)]
    pub error: Option<String>,
}

/// Log an op call start from JavaScript.
#[op2]
fn op_scene_inspector_op_start(state: Rc<RefCell<OpState>>, #[serde] data: JsOpCallStartData) {
    let scene_id = {
        let op_state = state.borrow();
        op_state
            .try_borrow::<crate::dcl::SceneId>()
            .map(|id| id.0)
            .unwrap_or(0)
    };

    if let Some(sender) = get_logger_sender() {
        let entry = OpCallStartEntry {
            call_id: data.call_id,
            scene_id,
            timestamp_ms: current_timestamp_ms(),
            op_name: data.op_name,
            args: data.args,
        };

        try_send_entry(&sender, SceneInspectorEntry::OpCallStart(entry));
    }
}

/// Log an op call end from JavaScript.
#[op2]
fn op_scene_inspector_op_end(state: Rc<RefCell<OpState>>, #[serde] data: JsOpCallEndData) {
    let scene_id = {
        let op_state = state.borrow();
        op_state
            .try_borrow::<crate::dcl::SceneId>()
            .map(|id| id.0)
            .unwrap_or(0)
    };

    if let Some(sender) = get_logger_sender() {
        let entry = OpCallEndEntry {
            call_id: data.call_id,
            scene_id,
            timestamp_ms: current_timestamp_ms(),
            op_name: data.op_name,
            result: data.result,
            is_async: data.is_async,
            duration_ms: data.duration_ms,
            error: data.error,
        };

        try_send_entry(&sender, SceneInspectorEntry::OpCallEnd(entry));
    }
}
