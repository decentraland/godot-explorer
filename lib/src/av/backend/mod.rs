use godot::{engine::ImageTexture, prelude::*};

use super::stream_processor::{AVCommand, StreamStateData};

/// Represents the different video player backend types.
/// Each backend is responsible for handling video playback on specific platforms or for specific URL schemes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BackendType {
    /// LiveKit streaming backend for `livekit-video://` URLs
    LiveKit,
    /// ExoPlayer backend for Android platform (regular videos)
    ExoPlayer,
    /// AVPlayer backend for iOS platform
    AVPlayer,
    /// No-op backend when no video playback is available
    #[default]
    Noop,
}

impl BackendType {
    /// Select the appropriate backend based on the video source URL and current platform.
    pub fn from_source(source: &str) -> Self {
        // LiveKit streams have a special URL scheme
        if source.starts_with("livekit-video://") {
            return BackendType::LiveKit;
        }

        // Platform-specific backend selection for regular videos
        #[cfg(target_os = "android")]
        {
            return BackendType::ExoPlayer;
        }

        #[cfg(target_os = "ios")]
        {
            return BackendType::AVPlayer;
        }

        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        {
            BackendType::Noop
        }
    }

    /// Returns the integer representation for passing to GDScript
    pub fn to_gd_int(self) -> i32 {
        match self {
            BackendType::LiveKit => 0,
            BackendType::ExoPlayer => 1,
            BackendType::AVPlayer => 2,
            BackendType::Noop => 3,
        }
    }
}

pub struct AudioSink {
    pub command_sender: tokio::sync::mpsc::Sender<AVCommand>,
}

pub struct VideoSink {
    pub source: String,
    pub command_sender: tokio::sync::mpsc::Sender<AVCommand>,
    pub texture: Gd<ImageTexture>,
    pub size: (u32, u32),
    pub current_time: f64,
    pub length: Option<f64>,
    pub rate: Option<f64>,
    pub stream_data_state_receiver: tokio::sync::mpsc::Receiver<StreamStateData>,
}

pub fn av_sinks(
    source: String,
    texture: Option<Gd<ImageTexture>>,
    audio_stream_player: Gd<godot::prelude::AudioStreamPlayer>,
    playing: bool,
    repeat: bool,
    wait_for_resource: Option<tokio::sync::oneshot::Receiver<String>>,
) -> (Option<VideoSink>, AudioSink) {
    noop::av_sinks(
        source,
        texture,
        audio_stream_player,
        playing,
        repeat,
        wait_for_resource,
    )
}

pub mod noop;
