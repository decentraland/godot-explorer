//! Audio bus volumes from the player's settings.

use godot::classes::AudioServer;
use godot::prelude::*;

use crate::fetcher::globals;

pub const MIN_VOLUME_DB: f32 = -80.0;

/// A slider position (0 to 100) as bus gain. Linear in amplitude, like Unity's
/// `AudioUtils.PercentageVolumeToDecibel`: 50% is -6 dB, not -40 dB.
pub fn percentage_to_db(percentage: f32) -> f32 {
    if percentage <= 0.0 {
        return MIN_VOLUME_DB;
    }
    (20.0 * (percentage / 100.0).log10()).max(MIN_VOLUME_DB)
}

/// Each setting and the bus it drives.
const BUSES: [(&str, &str); 6] = [
    ("audio_general_volume", "Master"),
    ("audio_scene_volume", "Scene"),
    ("audio_ui_volume", "UI"),
    ("audio_music_volume", "Music"),
    ("audio_avatar_and_emotes_volume", "AvatarAndEmotes"),
    ("audio_voice_chat_volume", "VoiceChat"),
];

fn apply_bus(bus: &str, percentage: f32) {
    let mut server = AudioServer::singleton();
    let index = server.get_bus_index(bus);
    if index < 0 {
        godot_error!("AudioSettings: unknown audio bus {bus}");
        return;
    }
    server.set_bus_volume_db(index, percentage_to_db(percentage));
}

pub const SCENE_VOLUME: &str = "audio_scene_volume";
pub const VOICE_CHAT_VOLUME: &str = "audio_voice_chat_volume";
pub const AVATAR_AND_EMOTES_VOLUME: &str = "audio_avatar_and_emotes_volume";

/// Applies one setting by name, or `forced` in its place (the loading screen ducks the scene's
/// audio this way); false when it is not an audio volume.
pub fn apply_setting_with(name: &str, forced: Option<f32>) -> bool {
    let Some((setting, bus)) = BUSES.iter().find(|(s, _)| *s == name) else {
        return false;
    };
    let percentage = forced.unwrap_or_else(|| {
        globals::engine()
            .bind()
            .setting((*setting).into())
            .try_to::<f64>()
            .unwrap_or(100.0) as f32
    });
    apply_bus(bus, percentage);
    true
}

pub fn apply_setting(name: &str) -> bool {
    apply_setting_with(name, None)
}

pub fn apply_volume_settings() {
    for (setting, _) in BUSES {
        apply_setting(setting);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_is_linear_in_amplitude() {
        assert_eq!(percentage_to_db(0.0), MIN_VOLUME_DB);
        assert!((percentage_to_db(100.0)).abs() < 1e-4);
        assert!((percentage_to_db(50.0) + 6.0206).abs() < 1e-3);
        assert_eq!(percentage_to_db(0.0001), MIN_VOLUME_DB);
    }
}
