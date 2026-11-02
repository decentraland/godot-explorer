use chrono::Utc;
use godot::prelude::*;

const REAL_MINUTES_PER_CYCLE: f64 = 120.0; // 120 real minutes per in-game day
const CYCLE_DURATION_IN_GAME_SECONDS: f64 = 24.0 * 3600.0; // 86400 in-game seconds

#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub struct DclGlobalTime {
    _base: Base<RefCounted>,
}

#[godot_api]
impl DclGlobalTime {
    #[func]
    pub fn get_world_time() -> f64 {
        // Get current Unix time in milliseconds for higher precision.
        let millis = Utc::now().timestamp_millis() as f64;
        let real_seconds = millis / 1000.0;

        // Calculate the speed factor: in-game seconds per real second.
        // For example, with a 2-minute cycle: speed_factor = 86400 / (2*60) = 720.
        let speed_factor = CYCLE_DURATION_IN_GAME_SECONDS / (REAL_MINUTES_PER_CYCLE * 60.0);

        // Multiply the real seconds by the speed factor, then wrap it to [0, 86400)
        (real_seconds * speed_factor) % CYCLE_DURATION_IN_GAME_SECONDS
    }
}
