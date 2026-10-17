//! Report tag naming the resource a worker thread is loading, so a native crash
//! inside Godot's loader says which file it died on. The core batches the changes.

use dcl_core::{report::LOADING_RESOURCE_TAG, Core};

pub fn set_loading_resource(path: &str) {
    if let Some(core) = Core::try_get() {
        core.hub().set_report_tag(LOADING_RESOURCE_TAG, path);
    }
}

pub fn clear_loading_resource() {
    if let Some(core) = Core::try_get() {
        core.hub().clear_report_tag(LOADING_RESOURCE_TAG);
    }
}
