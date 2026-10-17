use super::value::{SettingKind, SettingValue};

/// The keys the shell may read and write, nothing else in the legacy config.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum SettingKey {
    SubmitMessageClosesChat,
    Locale,
    GraphicProfile,
    DynamicGraphicsEnabled,
    DynamicSkybox,
    SkyboxTime,
    ViewDistance,
    ParticleQuality,
    LimitFps,
    MaxCacheSize,
    AudioGeneralVolume,
    AudioSceneVolume,
    AudioVoiceChatVolume,
    AudioUiVolume,
    AudioMusicVolume,
    AudioAvatarAndEmotesVolume,
    AudioMicAmplification,
}

impl SettingKey {
    pub const ALL: [SettingKey; 17] = [
        Self::SubmitMessageClosesChat,
        Self::Locale,
        Self::GraphicProfile,
        Self::DynamicGraphicsEnabled,
        Self::DynamicSkybox,
        Self::SkyboxTime,
        Self::ViewDistance,
        Self::ParticleQuality,
        Self::LimitFps,
        Self::MaxCacheSize,
        Self::AudioGeneralVolume,
        Self::AudioSceneVolume,
        Self::AudioVoiceChatVolume,
        Self::AudioUiVolume,
        Self::AudioMusicVolume,
        Self::AudioAvatarAndEmotesVolume,
        Self::AudioMicAmplification,
    ];

    /// The name used by `settings.cfg` and the shell's `keys.ts`.
    pub fn name(self) -> &'static str {
        match self {
            Self::SubmitMessageClosesChat => "submit_message_closes_chat",
            Self::Locale => "locale",
            Self::GraphicProfile => "graphic_profile",
            Self::DynamicGraphicsEnabled => "dynamic_graphics_enabled",
            Self::DynamicSkybox => "dynamic_skybox",
            Self::SkyboxTime => "skybox_time",
            Self::ViewDistance => "view_distance",
            Self::ParticleQuality => "particle_quality",
            Self::LimitFps => "limit_fps",
            Self::MaxCacheSize => "max_cache_size",
            Self::AudioGeneralVolume => "audio_general_volume",
            Self::AudioSceneVolume => "audio_scene_volume",
            Self::AudioVoiceChatVolume => "audio_voice_chat_volume",
            Self::AudioUiVolume => "audio_ui_volume",
            Self::AudioMusicVolume => "audio_music_volume",
            Self::AudioAvatarAndEmotesVolume => "audio_avatar_and_emotes_volume",
            Self::AudioMicAmplification => "audio_mic_amplification",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|key| key.name() == name)
    }

    pub fn kind(self) -> SettingKind {
        match self {
            Self::SubmitMessageClosesChat | Self::DynamicGraphicsEnabled | Self::DynamicSkybox => {
                SettingKind::Bool
            }
            Self::Locale => SettingKind::Text,
            Self::GraphicProfile
            | Self::SkyboxTime
            | Self::ParticleQuality
            | Self::LimitFps
            | Self::MaxCacheSize => SettingKind::Int,
            Self::ViewDistance
            | Self::AudioGeneralVolume
            | Self::AudioSceneVolume
            | Self::AudioVoiceChatVolume
            | Self::AudioUiVolume
            | Self::AudioMusicVolume
            | Self::AudioAvatarAndEmotesVolume
            | Self::AudioMicAmplification => SettingKind::Float,
        }
    }

    /// The `ConfigData` defaults. The locale is empty: follow the device until the player picks one.
    pub fn default_value(self) -> SettingValue {
        match self {
            Self::SubmitMessageClosesChat => SettingValue::Bool(false),
            Self::DynamicGraphicsEnabled | Self::DynamicSkybox => SettingValue::Bool(true),
            Self::Locale => SettingValue::Text(String::new()),
            Self::GraphicProfile | Self::ParticleQuality => SettingValue::Int(3),
            Self::LimitFps => SettingValue::Int(3),
            Self::SkyboxTime => SettingValue::Int(43200),
            Self::MaxCacheSize => SettingValue::Int(1),
            Self::ViewDistance => SettingValue::Float(300.0),
            Self::AudioGeneralVolume
            | Self::AudioSceneVolume
            | Self::AudioVoiceChatVolume
            | Self::AudioUiVolume
            | Self::AudioMusicVolume
            | Self::AudioAvatarAndEmotesVolume
            | Self::AudioMicAmplification => SettingValue::Float(100.0),
        }
    }

    /// Whether the engine has something to do when the key changes. The chat, skybox-mode and
    /// skybox-time keys are only read by their consumers.
    pub fn needs_apply(self) -> bool {
        !matches!(
            self,
            Self::SubmitMessageClosesChat | Self::DynamicSkybox | Self::SkyboxTime
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct SettingEntry {
    pub key: SettingKey,
    pub value: SettingValue,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip_and_are_unique() {
        for key in SettingKey::ALL {
            assert_eq!(SettingKey::from_name(key.name()), Some(key));
        }
        assert_eq!(SettingKey::from_name("camera_mode"), None);
        assert_eq!(SettingKey::from_name("Locale"), None);
    }

    #[test]
    fn defaults_match_their_kind() {
        for key in SettingKey::ALL {
            assert_eq!(
                key.default_value().coerce(key.kind()),
                Some(key.default_value())
            );
        }
    }

    #[test]
    fn config_data_defaults() {
        assert_eq!(SettingKey::LimitFps.default_value(), SettingValue::Int(3));
        assert_eq!(
            SettingKey::ViewDistance.default_value(),
            SettingValue::Float(300.0)
        );
        assert_eq!(
            SettingKey::SkyboxTime.default_value(),
            SettingValue::Int(43200)
        );
        assert_eq!(
            SettingKey::MaxCacheSize.default_value(),
            SettingValue::Int(1)
        );
    }
}
