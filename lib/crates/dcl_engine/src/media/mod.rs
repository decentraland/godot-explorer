//! Scene media: the native video and audio players behind `VideoPlayer` and `AudioStream`.

pub mod native_player;

use crate::engine::settings_link::get_variant;

/// A volume setting stored as a percentage, as 0..1; a missing or odd value reads as full.
pub fn setting_ratio(name: &str) -> f32 {
    let value = get_variant(name);
    let percent = value
        .try_to::<f64>()
        .or_else(|_| value.try_to::<i64>().map(|v| v as f64))
        .unwrap_or(100.0);
    (percent / 100.0) as f32
}

/// The level a native player plays at: master x scene x the component's own volume. Native
/// players bypass Godot's buses, so every level is applied here (it matches Unity).
pub fn native_effective_volume(muted: bool, own_volume: f32) -> f32 {
    if muted {
        return 0.0;
    }
    setting_ratio("audio_general_volume") * setting_ratio("audio_scene_volume") * own_volume
}

/// Play and pause commands closer than this are held back and applied once the window passes: a
/// scene toggling rapidly otherwise upsets the native player.
pub const PLAY_PAUSE_DEBOUNCE_MS: u64 = 100;

/// What a debounced play/pause does at `now`.
#[derive(Debug, PartialEq, Eq)]
pub enum Debounce {
    /// Apply now.
    Apply,
    /// Keep it pending until the window passes.
    Hold,
}

pub fn debounce(now_ms: u64, last_ms: u64) -> Debounce {
    if now_ms.saturating_sub(last_ms) < PLAY_PAUSE_DEBOUNCE_MS {
        Debounce::Hold
    } else {
        Debounce::Apply
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rapid_commands_are_held() {
        assert_eq!(debounce(1000, 950), Debounce::Hold);
        assert_eq!(debounce(1000, 900), Debounce::Apply);
        assert_eq!(debounce(0, 0), Debounce::Hold);
    }
}
