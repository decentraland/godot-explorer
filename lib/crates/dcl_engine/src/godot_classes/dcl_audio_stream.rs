use godot::classes::{AudioStreamPlayer, IAudioStreamPlayer, Time};
use godot::prelude::*;

use crate::av::backend::BackendType;
use crate::engine::local_task::{next_frame, spawn_local};
use crate::media::native_player::{Flavor, NativePlayer};
use crate::media::{debounce, native_effective_volume, Debounce};

/// Stream state constants, polled by the scene runner. The numeric values match the
/// `VIDEO_STATE_*` ones so both media components speak the same language to their native backends.
pub const STREAM_STATE_NONE: i32 = 0;
pub const STREAM_STATE_LOADING: i32 = 1;
pub const STREAM_STATE_READY: i32 = 2;
pub const STREAM_STATE_PLAYING: i32 = 3;
pub const STREAM_STATE_PAUSED: i32 = 6;
pub const STREAM_STATE_ERROR: i32 = 7;

/// The state of an audio stream's native player. A live stream (Icecast, SHOUTcast, endless HLS,
/// the component's main use) reports duration 0 for its whole life, so a duration test reads it
/// as loading forever: what was asked of the player tells loading from stopped on every source.
pub fn stream_state(native_playing: bool, asked_to_play: bool, has_played: bool) -> i32 {
    if native_playing {
        STREAM_STATE_PLAYING
    } else if asked_to_play {
        // Asked to play and not started: still buffering.
        STREAM_STATE_LOADING
    } else if has_played {
        STREAM_STATE_PAUSED
    } else {
        STREAM_STATE_READY
    }
}

/// Audio-only sibling of the video player: it drives the same native players (ExoPlayer on
/// Android, AVPlayer on iOS) without the video surface. The native node's own `process` polls
/// the video surface, which is why it is disabled here.
#[derive(GodotClass)]
#[class(init, base=AudioStreamPlayer)]
pub struct DclAudioStream {
    #[var(pub)]
    dcl_volume: f32,

    #[var(pub)]
    dcl_muted: bool,

    #[export]
    #[var(pub)]
    dcl_source: GString,

    #[var(pub)]
    dcl_scene_id: i32,

    /// See `STREAM_STATE_*`.
    #[var(pub)]
    stream_state: i32,

    /// Last `MediaState` appended to `PBAudioEvent`, to avoid re-emitting it.
    pub last_media_state: i32,

    backend_type: BackendType,

    native: Option<Gd<NativePlayer>>,
    source: String,
    is_playing: bool,
    last_effective_volume: f32,
    last_play_pause_ms: u64,
    /// -1 none, 0 pause, 1 play.
    pending_play_state: i32,
    /// Bumped on dispose so an init still waiting for a frame knows it was superseded.
    init_epoch: u32,
    /// The native player only carries a state worth reporting once it has a source.
    source_ready: bool,
    /// Separates "paused" from "never started", which the native player cannot.
    has_played: bool,

    base: Base<AudioStreamPlayer>,
}

#[godot_api]
impl IAudioStreamPlayer for DclAudioStream {
    fn process(&mut self, _delta: f64) {
        if self.backend_type == BackendType::Noop || self.native.is_none() {
            return;
        }
        self.process_pending_play_state();
        self.update_volume();
        self.update_stream_state();
    }
}

#[godot_api]
impl DclAudioStream {
    /// Initialize the native backend. Mirrors `DclVideoPlayer::init_backend`, minus everything
    /// video.
    #[func]
    pub fn init_backend(&mut self, backend_type: i32, source: GString, playing: bool) {
        self.backend_dispose();
        self.backend_type = match backend_type {
            1 => BackendType::ExoPlayer,
            2 => BackendType::AVPlayer,
            _ => BackendType::Noop,
        };
        self.dcl_source = source.clone();
        self.source = source.to_string();
        self.is_playing = playing;

        tracing::debug!(
            "DclAudioStream::init_backend - type={:?}, source={}, playing={}",
            self.backend_type,
            self.dcl_source,
            playing
        );

        match self.backend_type {
            BackendType::ExoPlayer => self.start_native(Flavor::Exo),
            BackendType::AVPlayer => self.start_native(Flavor::Av),
            _ => self.stream_state = STREAM_STATE_NONE,
        }
    }

    #[func]
    pub fn backend_play(&mut self) {
        let now = Time::singleton().get_ticks_msec();
        if debounce(now, self.last_play_pause_ms) == Debounce::Hold {
            self.pending_play_state = 1;
            return;
        }
        self.last_play_pause_ms = now;
        self.pending_play_state = -1;
        self.apply_play();
    }

    #[func]
    pub fn backend_pause(&mut self) {
        let now = Time::singleton().get_ticks_msec();
        if debounce(now, self.last_play_pause_ms) == Debounce::Hold {
            self.pending_play_state = 0;
            return;
        }
        self.last_play_pause_ms = now;
        self.pending_play_state = -1;
        self.apply_pause();
    }

    #[func]
    pub fn backend_dispose(&mut self) {
        self.init_epoch += 1;
        self.free_native();
        self.backend_type = BackendType::Noop;
        self.source.clear();
        self.is_playing = false;
        self.source_ready = false;
        self.has_played = false;
        self.stream_state = STREAM_STATE_NONE;
        self.last_effective_volume = -1.0;
        self.pending_play_state = -1;
    }

    pub fn set_muted(&mut self, value: bool) {
        self.dcl_muted = value;
    }

    pub fn set_volume(&mut self, value: f32) {
        self.dcl_volume = value;
    }
}

impl DclAudioStream {
    fn start_native(&mut self, flavor: Flavor) {
        self.stream_state = STREAM_STATE_LOADING;
        let native = NativePlayer::create(flavor);
        // The native node's own `process` allocates a video surface as soon as the source reports
        // a video size (a scene can point an AudioStream at an HLS or MP4): nothing here renders
        // it, so it stays idle.
        let mut node = native.clone();
        node.set_process(false);
        self.base_mut().add_child(&native);
        self.native = Some(native);
        spawn_local(init_native(self.to_gd(), self.init_epoch));
    }

    fn free_native(&mut self) {
        if let Some(mut native) = self.native.take() {
            native.queue_free();
        }
    }

    fn process_pending_play_state(&mut self) {
        if self.pending_play_state == -1 {
            return;
        }
        let now = Time::singleton().get_ticks_msec();
        if debounce(now, self.last_play_pause_ms) == Debounce::Hold {
            return;
        }
        let pending = self.pending_play_state;
        self.pending_play_state = -1;
        self.last_play_pause_ms = now;
        if pending == 1 {
            self.apply_play();
        } else {
            self.apply_pause();
        }
    }

    fn update_volume(&mut self) {
        let volume = native_effective_volume(self.dcl_muted, self.dcl_volume);
        if (volume - self.last_effective_volume).abs() < 0.001 {
            return;
        }
        self.last_effective_volume = volume;
        if let Some(native) = self.native.as_mut() {
            native.bind_mut().set_volume(volume);
        }
    }

    fn update_stream_state(&mut self) {
        if !self.source_ready {
            return;
        }
        let Some(native) = self.native.as_mut() else {
            return;
        };
        let playing = native.bind_mut().is_playing();
        if playing {
            self.has_played = true;
        }
        self.stream_state = stream_state(playing, self.is_playing, self.has_played);
    }

    fn apply_play(&mut self) {
        self.is_playing = true;
        self.last_effective_volume = -1.0;
        if let Some(native) = self.native.as_mut() {
            native.bind_mut().play();
        }
    }

    fn apply_pause(&mut self) {
        self.is_playing = false;
        self.last_effective_volume = -1.0;
        if let Some(native) = self.native.as_mut() {
            native.bind_mut().pause();
        }
    }
}

async fn init_native(stream: Gd<DclAudioStream>, epoch: u32) {
    next_frame().await;
    // `backend_dispose` (entity removed, or a second `url` on the same entity) frees the player
    // and bumps the epoch: this task no longer owns it.
    if !stream.is_instance_valid() {
        return;
    }
    let (mut native, source, playing) = {
        let this = stream.bind();
        let Some(native) = this.native.clone().filter(|_| this.init_epoch == epoch) else {
            return;
        };
        (native, this.source.clone(), this.is_playing)
    };
    if !native.is_instance_valid() {
        return;
    }

    let mut handle = stream.clone();
    if !(source.starts_with("http://") || source.starts_with("https://")) {
        godot_warn!("AudioStream: only remote sources are supported, got {source}");
        let mut this = handle.bind_mut();
        this.stream_state = STREAM_STATE_ERROR;
        this.free_native();
        return;
    }
    // No texture: the video surface is not needed.
    if !native.bind_mut().set_source_url(&source) {
        godot_warn!("AudioStream: failed to set source {source}");
        let mut this = handle.bind_mut();
        this.stream_state = STREAM_STATE_ERROR;
        this.free_native();
        return;
    }
    native.bind_mut().set_looping(false);
    handle.bind_mut().source_ready = true;
    if playing {
        native.bind_mut().play();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_playing_native_player_is_playing() {
        assert_eq!(stream_state(true, true, true), STREAM_STATE_PLAYING);
    }

    #[test]
    fn asked_to_play_but_silent_is_buffering() {
        assert_eq!(stream_state(false, true, false), STREAM_STATE_LOADING);
    }

    #[test]
    fn paused_is_told_from_never_started() {
        assert_eq!(stream_state(false, false, true), STREAM_STATE_PAUSED);
        assert_eq!(stream_state(false, false, false), STREAM_STATE_READY);
    }
}
