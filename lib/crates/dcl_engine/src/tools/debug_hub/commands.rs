//! The commands of the debug hub: inspection (scenes, entities, UI, avatars, focus), state, and a
//! few typed controls (teleport, screenshot, log filter). It replaces the GDScript `eval` of the
//! old hub: what a developer evaluated is a typed command now, and `node_query` reads any
//! property of a node by name.

use std::cell::RefCell;

use godot::classes::{Control, Engine, Marshalls, Os, ProjectSettings, Time};
use godot::prelude::*;
use serde_json::{json, Map, Value};

use super::collector;
use crate::fetcher::globals;
use crate::godot_classes::dcl_global::DclGlobal;
use crate::world;

/// Focus-change entries kept for the `focus` command.
const FOCUS_HISTORY_MAX: usize = 64;

/// Polls the keyboard-focus owner each frame and logs every change (releasing to nothing too, which
/// `gui_focus_changed` misses): a diagnostic for "input stops working" bugs, since movement is
/// gated by the explorer UI having focus.
#[derive(Default)]
pub struct FocusTracker {
    last: RefCell<String>,
    history: RefCell<Vec<Value>>,
}

impl FocusTracker {
    pub fn poll(&self) {
        let Some(window) = DclGlobal::singleton()
            .get_tree_or_null()
            .and_then(|t| t.get_root())
        else {
            return;
        };
        let description = describe_focus(window.gui_get_focus_owner());
        let mut last = self.last.borrow_mut();
        if *last == description {
            return;
        }
        let mut history = self.history.borrow_mut();
        history.push(json!({
            "t_ms": Time::singleton().get_ticks_msec(),
            "frame": Engine::singleton().get_process_frames(),
            "from": *last,
            "to": description,
        }));
        let excess = history.len().saturating_sub(FOCUS_HISTORY_MAX);
        history.drain(..excess);
        *last = description;
    }

    fn report(&self) -> Value {
        let ui_root = world::state::current().map_or_else(
            || "<no explorer>".to_string(),
            |w| w.ui_root.get_path().to_string(),
        );
        json!({
            "current": *self.last.borrow(),
            "ui_root_path": ui_root,
            "history": *self.history.borrow(),
        })
    }
}

fn describe_focus(owner: Option<Gd<Control>>) -> String {
    match owner {
        None => "<none>".to_string(),
        Some(node) => format!("{} [{}]", node.get_path(), node.get_class()),
    }
}

fn arg_i64(args: &Map<String, Value>, key: &str) -> Option<i64> {
    args.get(key).and_then(Value::as_i64)
}

fn filters(args: &Map<String, Value>) -> Map<String, Value> {
    args.get("filters")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
}

fn checked(result: Value) -> Result<Value, String> {
    match result.get("error").and_then(Value::as_str) {
        Some(error) => Err(error.to_string()),
        None => Ok(result),
    }
}

/// Runs an inspection or control command; `Err` carries the reason a client is told.
pub fn run(
    command: &str,
    args: &Map<String, Value>,
    focus: &FocusTracker,
) -> Result<Value, String> {
    match command {
        "ping" => Ok(ping()),
        "focus" => Ok(focus.report()),
        "scenes" => Ok(collector::scenes_summary()),
        "avatars" => Ok(collector::avatars()),
        "state" => Ok(state()),
        "scene" => {
            let id = arg_i64(args, "scene_id").unwrap_or(-1);
            checked(collector::scene(id, &filters(args)))
        }
        "entity" => {
            let mut entity_filters = filters(args);
            // Also accepted at the top level, for older clients.
            for key in ["include_parents", "include_children"] {
                if let (Some(value), false) = (args.get(key), entity_filters.contains_key(key)) {
                    entity_filters.insert(key.into(), value.clone());
                }
            }
            let scene = arg_i64(args, "scene_id").unwrap_or(-1);
            let entity = arg_i64(args, "entity_id").unwrap_or(-1);
            checked(collector::entity(scene, entity, &entity_filters))
        }
        "ui_scene" | "ui_entity" => {
            let mut ui_filters = filters(args);
            ui_filters.insert("tree".into(), json!("ui"));
            let scene = arg_i64(args, "scene_id").unwrap_or(-1);
            if command == "ui_scene" {
                checked(collector::scene(scene, &ui_filters))
            } else {
                let entity = arg_i64(args, "entity_id").unwrap_or(-1);
                checked(collector::entity(scene, entity, &ui_filters))
            }
        }
        "app_ui" => checked(collector::app_ui(&filters(args))),
        "avatar" => {
            let by = args.get("by").and_then(Value::as_str).unwrap_or_default();
            if by.is_empty() {
                return Err("missing 'by' (expected address|alias|entity|local)".to_string());
            }
            // `local` is keyless; every other mode needs a value.
            if by != "local" && !args.contains_key("value") {
                return Err("missing 'value'".to_string());
            }
            let value = args.get("value").cloned().unwrap_or(Value::Null);
            checked(collector::avatar(by, &value, &filters(args)))
        }
        "teleport" => teleport(args),
        "screenshot" => screenshot(args),
        "log_filter" => log_filter(args),
        "eval" => Err(
            "eval was removed: use node_query, state, teleport, screenshot or log_filter"
                .to_string(),
        ),
        _ => Err(format!("unknown command: {command}")),
    }
}

fn ping() -> Value {
    let version = ProjectSettings::singleton()
        .get_setting_ex("application/config/version")
        .default_value(&"unknown".to_variant())
        .done()
        .stringify()
        .to_string();
    let engine = Engine::singleton()
        .get_version_info()
        .get("string")
        .map(|v| v.stringify().to_string())
        .unwrap_or_default();
    json!({
        "version": version,
        "engine": engine,
        "scenes_loaded": globals::scene_runner().bind().debug_get_loaded_scene_ids().len(),
    })
}

/// Where the player is and how the engine is doing: the dump a developer asks for first.
fn state() -> Value {
    let global = DclGlobal::singleton();
    let realm = globals::realm().bind().get_realm_string().to_string();
    let mut value = json!({
        "realm": realm,
        "fps": Engine::singleton().get_frames_per_second(),
        "scenes_loaded": globals::scene_runner().bind().debug_get_loaded_scene_ids().len(),
        "memory_static_mb": Os::singleton().get_static_memory_usage() as f64 / 1_048_576.0,
        "camera_mode": global.bind().current_camera_mode,
        "portrait": global.bind().is_portrait,
        "signed_in": !globals::player_identity().bind().get_address_str().is_empty(),
    });
    if let Some(world) = world::state::current() {
        let parcel = world.parcel.get();
        let position = world.player.get_global_position();
        value["parcel"] = json!([parcel.x, parcel.y]);
        value["player_position"] = json!([position.x, position.y, position.z]);
        value["loading"] = json!(world.loading.bind().is_running());
    }
    value
}

fn teleport(args: &Map<String, Value>) -> Result<Value, String> {
    let (Some(x), Some(y)) = (arg_i64(args, "x"), arg_i64(args, "y")) else {
        return Err("teleport needs integer 'x' and 'y' (a parcel)".to_string());
    };
    if world::state::current().is_none() {
        return Err("no world is loaded".to_string());
    }
    globals::engine()
        .bind()
        .teleport(Vector2i::new(x as i32, y as i32));
    Ok(json!({"parcel": [x, y]}))
}

/// Saves the viewport to `user://debug/`; with `inline: true` the PNG comes back base64-encoded.
fn screenshot(args: &Map<String, Value>) -> Result<Value, String> {
    let image = DclGlobal::singleton()
        .get_tree_or_null()
        .and_then(|tree| tree.get_root())
        .and_then(|window| window.get_texture())
        .and_then(|texture| texture.get_image())
        .ok_or("the viewport has no image")?;
    let path = format!(
        "user://debug/screenshot-{}.png",
        Time::singleton().get_ticks_msec()
    );
    crate::test_runner::testing_tools::ensure_parent_dir(&path);
    if image.save_png(path.as_str()) != godot::global::Error::OK {
        return Err(format!("could not save {path}"));
    }
    let mut result = json!({
        "path": path,
        "global_path": ProjectSettings::singleton().globalize_path(path.as_str()).to_string(),
        "width": image.get_width(),
        "height": image.get_height(),
    });
    if args.get("inline").and_then(Value::as_bool).unwrap_or(false) {
        let png = image.save_png_to_buffer();
        result["png_base64"] = json!(Marshalls::singleton().raw_to_base64(&png).to_string());
    }
    Ok(result)
}

fn log_filter(args: &Map<String, Value>) -> Result<Value, String> {
    let filter = args
        .get("filter")
        .and_then(Value::as_str)
        .filter(|f| !f.is_empty())
        .ok_or("log_filter needs a 'filter' like \"dclgodot::comms=debug,warn\"")?;
    DclGlobal::set_rust_log_filter(filter.into());
    Ok(json!({"filter": filter}))
}
