//! RenderingServer methods of the engine fork (decentraland/godotengine#27) that the bindings'
//! `extension_api.json` does not list. Each is probed once and called by name; an engine without
//! it behaves as if the feature were off.

use std::cell::RefCell;
use std::collections::HashMap;

use godot::classes::{ClassDb, RenderingServer};
use godot::prelude::*;

const ASYNC_SHADER_COMPILATION: &str = "enable_async_shader_compilation";
const DRAW_READY: &str = "instance_geometry_is_draw_ready";

thread_local! {
    static PROBED: RefCell<HashMap<&'static str, bool>> = RefCell::new(HashMap::new());
}

fn has(method: &'static str) -> bool {
    PROBED.with(|probed| {
        *probed
            .borrow_mut()
            .entry(method)
            .or_insert_with(|| ClassDb::singleton().class_has_method("RenderingServer", method))
    })
}

/// Turns on non-blocking shader/pipeline compilation with `threads` workers. False when the
/// engine lacks it.
pub fn enable_async_shader_compilation(threads: i32) -> bool {
    if !has(ASYNC_SHADER_COMPILATION) {
        return false;
    }
    RenderingServer::singleton().call("enable_async_shader_compilation", &[threads.to_variant()]);
    true
}

pub fn has_draw_ready() -> bool {
    has(DRAW_READY)
}

/// Whether the renderer can draw `instance` without compiling first; true when the engine lacks
/// the query.
pub fn instance_geometry_is_draw_ready(instance: Rid) -> bool {
    if !has(DRAW_READY) {
        return true;
    }
    RenderingServer::singleton()
        .call("instance_geometry_is_draw_ready", &[instance.to_variant()])
        .booleanize()
}
