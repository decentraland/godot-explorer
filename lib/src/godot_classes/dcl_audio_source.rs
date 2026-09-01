use godot::classes::{AudioStreamPlayback, AudioStreamPlayer3D};
use godot::prelude::*;

/// Clip loading state reported by GDScript. Combined with the node's playback to
/// derive the `MediaState` of `PBAudioEvent`.
pub const CLIP_STATE_NONE: i32 = 0;
pub const CLIP_STATE_LOADING: i32 = 1;
pub const CLIP_STATE_READY: i32 = 2;
pub const CLIP_STATE_ERROR: i32 = 3;

/// Seconds of drift tolerated between the analysis playback and the audible one
/// before re-seeking. Seeking a compressed stream is not free, so this only
/// catches loops, SDK seeks and scheduling hiccups.
const ANALYSIS_DRIFT_TOLERANCE: f64 = 0.25;

#[derive(GodotClass)]
#[class(init, base=AudioStreamPlayer3D)]
pub struct DclAudioSource {
    #[var]
    dcl_audio_clip_url: GString,

    #[var]
    dcl_playing: bool,

    #[var]
    dcl_volume: f32,

    #[var]
    dcl_loop_activated: bool,

    #[var]
    dcl_pitch: f32,

    #[var]
    dcl_global: bool,

    #[var]
    dcl_current_time: f32,

    #[var]
    dcl_enable: bool,

    #[var]
    dcl_scene_id: i32,

    /// See `CLIP_STATE_*`.
    #[var]
    dcl_clip_state: i32,

    /// Last `MediaState` appended to `PBAudioEvent`, to avoid re-emitting it.
    pub last_media_state: i32,

    /// Second, silent playback of the same clip, used only to read PCM for
    /// `PBAudioAnalysis`. Godot has no per-player tap on the audible signal, and
    /// bus effects would capture the mix after distance attenuation.
    analysis_playback: Option<Gd<AudioStreamPlayback>>,

    /// The stream `analysis_playback` was instantiated from, so swapping the
    /// clip mid-playback re-instantiates instead of analysing the old one.
    analysis_stream_id: Option<InstanceId>,

    base: Base<AudioStreamPlayer3D>,
}

#[godot_api]
impl DclAudioSource {}

impl DclAudioSource {
    pub fn is_clip_playing(&self) -> bool {
        self.base().is_playing()
    }

    /// Mono PCM for the current playhead, or empty when there is nothing to
    /// analyse. The shadow playback is created lazily and kept in sync with the
    /// audible one, so pitch and seeks are followed without a second decode of
    /// the whole clip on every call.
    pub fn read_analysis_samples(&mut self, frames: i32) -> Vec<f32> {
        if !self.base().is_playing() {
            self.analysis_playback = None;
            return Vec::new();
        }

        let Some(stream) = self.base().get_stream() else {
            self.analysis_playback = None;
            return Vec::new();
        };

        let position = self.base_mut().get_playback_position() as f64;
        let pitch = self.base().get_pitch_scale();

        if self.analysis_stream_id != Some(stream.instance_id()) {
            self.analysis_playback = None;
        }

        if self.analysis_playback.is_none() {
            let Some(mut playback) = stream.clone().instantiate_playback() else {
                return Vec::new();
            };
            playback.start_ex().from_pos(position).done();
            self.analysis_playback = Some(playback);
            self.analysis_stream_id = Some(stream.instance_id());
        }

        let Some(playback) = self.analysis_playback.as_mut() else {
            return Vec::new();
        };

        if (playback.get_playback_position() - position).abs() > ANALYSIS_DRIFT_TOLERANCE {
            playback.seek_ex().time(position).done();
        }

        let buffer = playback.mix_audio(pitch, frames);
        buffer
            .as_slice()
            .iter()
            .map(|frame| (frame.x + frame.y) * 0.5)
            .collect()
    }
}
