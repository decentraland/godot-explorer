use crate::av::backend::BackendType;
use godot::classes::AudioStreamPlayer;
use godot::prelude::*;

/// Stream state constants, written by GDScript and polled by the scene runner.
/// The numeric values match the `VIDEO_STATE_*` ones so both media components
/// speak the same language to their native backends.
pub const STREAM_STATE_NONE: i32 = 0;
pub const STREAM_STATE_LOADING: i32 = 1;
pub const STREAM_STATE_READY: i32 = 2;
pub const STREAM_STATE_PLAYING: i32 = 3;
pub const STREAM_STATE_PAUSED: i32 = 6;
pub const STREAM_STATE_ERROR: i32 = 7;

#[derive(GodotClass)]
#[class(init, base=AudioStreamPlayer)]
pub struct DclAudioStream {
    #[var]
    dcl_volume: f32,

    #[var]
    dcl_muted: bool,

    #[export]
    dcl_source: GString,

    #[var]
    dcl_scene_id: i32,

    /// See `STREAM_STATE_*`. Written by GDScript from the native player.
    #[var]
    stream_state: i32,

    /// Last `MediaState` appended to `PBAudioEvent`, to avoid re-emitting it.
    pub last_media_state: i32,

    backend_type: BackendType,

    base: Base<AudioStreamPlayer>,
}

#[godot_api]
impl DclAudioStream {
    /// Initialize the native backend. Mirrors `DclVideoPlayer::init_backend`,
    /// minus everything video: the GDScript side never touches a texture.
    #[func]
    pub fn init_backend(&mut self, backend_type: i32, source: GString, playing: bool) {
        self.backend_type = match backend_type {
            1 => BackendType::ExoPlayer,
            2 => BackendType::AVPlayer,
            _ => BackendType::Noop,
        };
        self.dcl_source = source.clone();

        tracing::debug!(
            "DclAudioStream::init_backend - type={:?}, source={}, playing={}",
            self.backend_type,
            self.dcl_source,
            playing
        );

        self.base_mut().call(
            "_init_backend_impl",
            &[
                backend_type.to_variant(),
                source.to_variant(),
                playing.to_variant(),
            ],
        );
    }

    #[func]
    pub fn backend_play(&mut self) {
        self.base_mut().call("_backend_play", &[]);
    }

    #[func]
    pub fn backend_pause(&mut self) {
        self.base_mut().call("_backend_pause", &[]);
    }

    #[func]
    pub fn backend_dispose(&mut self) {
        self.base_mut().call("_backend_dispose", &[]);
        self.backend_type = BackendType::Noop;
    }

    pub fn set_muted(&mut self, value: bool) {
        self.dcl_muted = value;
    }

    pub fn set_volume(&mut self, value: f32) {
        self.dcl_volume = value;
    }
}
