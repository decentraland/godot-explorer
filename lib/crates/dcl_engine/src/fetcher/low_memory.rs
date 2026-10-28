//! The memory monitor's critical-pressure warning (issue #2002): the `LOW_MEMORY_WARNING` metric
//! on every platform, the shell's modal everywhere but Android.

use godot::classes::Os;
use godot::global::godot_warn;
use godot::obj::Singleton;

use super::globals;
use crate::{engine::local_task::defer, godot_classes::dcl_global::DclGlobal};

pub fn on_warning(entity_id: &str, footprint_mb: i32, available_mb: i32) {
    godot_warn!(
        "[memory] critical pressure in {entity_id}: {footprint_mb} MB footprint, {available_mb} MB available"
    );
    // Android keeps playing without the modal (product decision).
    let show_modal = !DclGlobal::singleton().bind().is_android;
    let extra = serde_json::json!({
        "entity_id": entity_id,
        "footprint_mb": footprint_mb,
        "available_mb": available_mb,
        "platform": Os::singleton().get_name().to_string(),
        "modal_shown": show_modal,
    });
    globals::metrics()
        .bind_mut()
        .track_screen_viewed("LOW_MEMORY_WARNING".to_string(), extra.to_string());
    // The OS may kill the app right after this warning, so the periodic flush is not enough.
    defer(|| globals::metrics().bind_mut().flush());
    if show_modal {
        globals::engine()
            .bind()
            .report_notice("low_memory".into(), "".into());
    }
}
