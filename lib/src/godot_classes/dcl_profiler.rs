use godot::prelude::*;

use crate::tools::profiler;

/// GDScript access to the engine profiler zones (see `tools::profiler`).
/// `zone_begin`/`zone_end` must pair on the same thread with no `await` in between.
#[derive(GodotClass)]
#[class(no_init, base=Object)]
pub struct DclProfiler;

#[godot_api]
impl DclProfiler {
    #[func]
    fn is_enabled() -> bool {
        profiler::enabled()
    }

    #[func]
    fn zone_begin(name: GString, text: GString) {
        profiler::zone_begin(&name.to_string(), &text.to_string());
    }

    #[func]
    fn zone_end() {
        profiler::zone_end();
    }

    /// Point marker on the timeline.
    #[func]
    fn mark(name: GString, text: GString) {
        profiler::mark(&name.to_string(), &text.to_string());
    }
}
