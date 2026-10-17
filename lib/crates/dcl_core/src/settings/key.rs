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
    // Engine-owned: the shell does not list them, the engine reads and writes them.
    WindowMode,
    UiZoom,
    Resolution3dScale,
    ProcessTickQuotaMs,
    ShowFps,
    Skybox,
    ShadowQuality,
    BloomQuality,
    AntiAliasing,
    TextureQuality,
    AvatarImpostorsEnabled,
    FirstLaunchCompleted,
    BenchmarkGpuScore,
    BenchmarkRamGb,
    LastRealmJoined,
    LastParcelX,
    LastParcelY,
    FirstMoveInWorldSent,
    InstallReferrerSent,
    DebugRotateGuestAnchor,
    LocalAssetsCacheVersion,
}

impl SettingKey {
    pub const ALL: [SettingKey; 38] = [
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
        Self::WindowMode,
        Self::UiZoom,
        Self::Resolution3dScale,
        Self::ProcessTickQuotaMs,
        Self::ShowFps,
        Self::Skybox,
        Self::ShadowQuality,
        Self::BloomQuality,
        Self::AntiAliasing,
        Self::TextureQuality,
        Self::AvatarImpostorsEnabled,
        Self::FirstLaunchCompleted,
        Self::BenchmarkGpuScore,
        Self::BenchmarkRamGb,
        Self::LastRealmJoined,
        Self::LastParcelX,
        Self::LastParcelY,
        Self::FirstMoveInWorldSent,
        Self::InstallReferrerSent,
        Self::DebugRotateGuestAnchor,
        Self::LocalAssetsCacheVersion,
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
            Self::WindowMode => "window_mode",
            Self::UiZoom => "ui_zoom",
            Self::Resolution3dScale => "resolution_3d_scale",
            Self::ProcessTickQuotaMs => "process_tick_quota_ms",
            Self::ShowFps => "show_fps",
            Self::Skybox => "skybox",
            Self::ShadowQuality => "shadow_quality",
            Self::BloomQuality => "bloom_quality",
            Self::AntiAliasing => "anti_aliasing",
            Self::TextureQuality => "texture_quality",
            Self::AvatarImpostorsEnabled => "avatar_impostors_enabled",
            Self::FirstLaunchCompleted => "first_launch_completed",
            Self::BenchmarkGpuScore => "benchmark_gpu_score",
            Self::BenchmarkRamGb => "benchmark_ram_gb",
            Self::LastRealmJoined => "last_realm_joined",
            Self::LastParcelX => "last_parcel_x",
            Self::LastParcelY => "last_parcel_y",
            Self::FirstMoveInWorldSent => "first_move_in_world_sent",
            Self::InstallReferrerSent => "install_referrer_sent",
            Self::DebugRotateGuestAnchor => "debug_rotate_guest_anchor",
            Self::LocalAssetsCacheVersion => "local_assets_cache_version",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|key| key.name() == name)
    }

    pub fn kind(self) -> SettingKind {
        match self {
            Self::SubmitMessageClosesChat
            | Self::DynamicGraphicsEnabled
            | Self::DynamicSkybox
            | Self::ShowFps
            | Self::AvatarImpostorsEnabled
            | Self::FirstLaunchCompleted
            | Self::FirstMoveInWorldSent
            | Self::InstallReferrerSent
            | Self::DebugRotateGuestAnchor => SettingKind::Bool,
            Self::Locale | Self::LastRealmJoined => SettingKind::Text,
            Self::GraphicProfile
            | Self::SkyboxTime
            | Self::ParticleQuality
            | Self::LimitFps
            | Self::MaxCacheSize
            | Self::WindowMode
            | Self::ProcessTickQuotaMs
            | Self::Skybox
            | Self::ShadowQuality
            | Self::BloomQuality
            | Self::AntiAliasing
            | Self::TextureQuality
            | Self::LastParcelX
            | Self::LastParcelY
            | Self::LocalAssetsCacheVersion => SettingKind::Int,
            Self::ViewDistance
            | Self::AudioGeneralVolume
            | Self::AudioSceneVolume
            | Self::AudioVoiceChatVolume
            | Self::AudioUiVolume
            | Self::AudioMusicVolume
            | Self::AudioAvatarAndEmotesVolume
            | Self::AudioMicAmplification
            | Self::UiZoom
            | Self::Resolution3dScale
            | Self::BenchmarkGpuScore
            | Self::BenchmarkRamGb => SettingKind::Float,
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
            Self::WindowMode => SettingValue::Int(0),
            Self::UiZoom => SettingValue::Float(-1.0),
            Self::Resolution3dScale => SettingValue::Float(1.0),
            Self::ProcessTickQuotaMs => SettingValue::Int(10),
            Self::ShowFps => SettingValue::Bool(true),
            Self::Skybox => SettingValue::Int(0),
            Self::ShadowQuality => SettingValue::Int(0),
            Self::BloomQuality => SettingValue::Int(0),
            Self::AntiAliasing => SettingValue::Int(0),
            Self::TextureQuality => SettingValue::Int(1),
            Self::AvatarImpostorsEnabled => SettingValue::Bool(true),
            Self::FirstLaunchCompleted => SettingValue::Bool(false),
            Self::BenchmarkGpuScore => SettingValue::Float(-1.0),
            Self::BenchmarkRamGb => SettingValue::Float(-1.0),
            Self::LastRealmJoined => SettingValue::Text(String::new()),
            Self::LastParcelX => SettingValue::Int(72),
            Self::LastParcelY => SettingValue::Int(-10),
            Self::FirstMoveInWorldSent => SettingValue::Bool(false),
            Self::InstallReferrerSent => SettingValue::Bool(false),
            Self::DebugRotateGuestAnchor => SettingValue::Bool(false),
            Self::LocalAssetsCacheVersion => SettingValue::Int(0),
        }
    }

    /// Whether the engine has something to do when the key changes. The chat, skybox-mode and
    /// skybox-time keys are only read by their consumers.
    pub fn needs_apply(self) -> bool {
        self.shell_visible()
            && !matches!(
                self,
                Self::SubmitMessageClosesChat | Self::DynamicSkybox | Self::SkyboxTime
            )
    }

    /// The keys the shell lists; the rest are the engine's own state.
    pub fn shell_visible(self) -> bool {
        Self::ALL[..17].contains(&self)
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
