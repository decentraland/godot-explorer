//! What a scene test can ask of the app beyond the SDK (`~system/Testing.testCommand`), live only
//! under `--scene-test`: the shell's own commands and setting changes, sent to the core the way the
//! React Native shell sends them, and the debug hub's read-only queries for the engine state no SDK
//! component exposes.

use dcl_core::app::Control;
use dcl_core::settings::{self, SettingKey, SettingValue};
use dcl_core::Core;
use serde_json::{Map, Value};

use crate::tools::debug_hub::commands::{self, FocusTracker};

/// The debug-hub commands a test may run: they only read.
const QUERIES: [&str; 9] = [
    "state",
    "scenes",
    "scene",
    "entity",
    "ui_scene",
    "ui_entity",
    "avatars",
    "avatar",
    "app_ui",
];

/// Runs one test command on the main thread; the reply (or the reason it failed) is JSON text.
pub fn test_command(kind: &str, payload: &str) -> Result<String, String> {
    let payload: Value = serde_json::from_str(payload).map_err(|e| format!("bad payload: {e}"))?;
    let reply = match kind {
        "control" => control(payload)?,
        "setting" => setting(&payload)?,
        "query" => query(&payload)?,
        _ => return Err(format!("unknown test command: {kind}")),
    };
    Ok(reply.to_string())
}

/// A shell command (`{"type": "set_color", "target": "eyes", "color": "#ff0000"}`), handed to the
/// core: avatar edits come back to the engine as the avatar to render, the rest as the command.
fn control(payload: Value) -> Result<Value, String> {
    let control: Control =
        serde_json::from_value(payload).map_err(|e| format!("bad control: {e}"))?;
    let core = Core::try_get().ok_or("no core")?;
    core.with_app(|app| app.control(control));
    Ok(Value::Object(Map::new()))
}

/// `{"key": "graphic_profile", "value": 2}`: set through the core's settings, as the settings
/// screen does, without saving it to disk.
fn setting(payload: &Value) -> Result<Value, String> {
    let name = payload["key"].as_str().ok_or("setting: no key")?;
    let key = SettingKey::from_name(name).ok_or_else(|| format!("unknown setting: {name}"))?;
    let value = match &payload["value"] {
        Value::Bool(value) => SettingValue::Bool(*value),
        Value::Number(number) if number.is_i64() => SettingValue::Int(number.as_i64().unwrap_or(0)),
        Value::Number(number) => SettingValue::Float(number.as_f64().unwrap_or(0.0)),
        Value::String(text) => SettingValue::Text(text.clone()),
        other => return Err(format!("setting {name}: unsupported value {other}")),
    };
    let core = Core::try_get().ok_or("no core")?;
    let changed = settings::global(core.data_dir())
        .set(key, value, false)
        .map_err(|e| format!("setting {name}: {e}"))?;
    Ok(Value::from(changed.len()))
}

/// `{"cmd": "avatar", "args": {...}}`: a read-only debug-hub query, answered as the hub answers it.
fn query(payload: &Value) -> Result<Value, String> {
    let cmd = payload["cmd"].as_str().ok_or("query: no cmd")?;
    if !QUERIES.contains(&cmd) {
        return Err(format!("query: '{cmd}' is not a read-only query"));
    }
    let args = payload["args"].as_object().cloned().unwrap_or_default();
    commands::run(cmd, &args, &FocusTracker::default())
}
