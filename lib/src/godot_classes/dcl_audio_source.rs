use godot::classes::{AudioStreamPlayback, AudioStreamPlayer3D};
use godot::prelude::*;

/// Clip loading state reported by GDScript. Combined with the node's playback to
/// derive the `MediaState` of `PBAudioEvent`.
pub const CLIP_STATE_NONE: i32 = 0;
pub const CLIP_STATE_LOADING: i32 = 1;
pub const CLIP_STATE_READY: i32 = 2;
pub const CLIP_STATE_ERROR: i32 = 3;

/// Samples of history kept for the FFT. A power of two, so the whole buffer is
/// one window.
pub const ANALYSIS_WINDOW: usize = 2048;

/// Most frames decoded in a single call, so a long tick cannot ask for an
/// unbounded decode. Past this the playback is resynced to the audible one
/// instead of catching up.
const MAX_CATCHUP_FRAMES: i32 = (ANALYSIS_WINDOW * 4) as i32;

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

    /// Playhead of the audible player at the previous read, used to decode
    /// exactly the audio that elapsed since then.
    analysis_last_position: f64,

    /// The last `ANALYSIS_WINDOW` mono samples, oldest first.
    analysis_buffer: Vec<f32>,

    base: Base<AudioStreamPlayer3D>,
}

#[godot_api]
impl DclAudioSource {}

impl DclAudioSource {
    pub fn is_clip_playing(&self) -> bool {
        self.base().is_playing()
    }

    /// The last `ANALYSIS_WINDOW` mono samples of what is currently audible, or
    /// empty when there is nothing to analyse.
    ///
    /// The shadow playback is advanced by exactly the audio that elapsed since
    /// the previous call. Pulling a fixed block per tick instead would run it
    /// several times faster than real time, which both races the audible
    /// playhead and walks off the end of the clip — and once an
    /// `AudioStreamPlayback` goes inactive at EOF, `seek` and `mix` are both
    /// no-ops for good (`audio_stream_ogg_vorbis.cpp`), so it would never
    /// recover. Looping is emulated by GDScript replaying the clip, so the
    /// wrap arrives here as a backwards jump and is resynced.
    pub fn read_analysis_samples(&mut self, sample_rate: f32) -> Vec<f32> {
        if !self.base().is_playing() || sample_rate <= 0.0 {
            self.analysis_playback = None;
            return Vec::new();
        }

        let Some(stream) = self.base().get_stream() else {
            self.analysis_playback = None;
            return Vec::new();
        };

        let position = self.base_mut().get_playback_position() as f64;
        let pitch = self.base().get_pitch_scale().max(0.01) as f64;

        if self.analysis_stream_id != Some(stream.instance_id()) {
            self.analysis_playback = None;
            self.analysis_buffer.clear();
        }

        // `position` is stream time, so it advances by `pitch` per second of
        // real time; `mix_audio(pitch, n)` consumes `n * pitch` stream frames.
        let elapsed = position - self.analysis_last_position;
        let needed = (elapsed * sample_rate as f64 / pitch).round() as i32;
        let resync =
            self.analysis_playback.is_none() || !(0..=MAX_CATCHUP_FRAMES).contains(&needed);

        if resync {
            let Some(mut playback) = stream.clone().instantiate_playback() else {
                return Vec::new();
            };
            playback.start_ex().from_pos(position).done();
            self.analysis_playback = Some(playback);
            self.analysis_stream_id = Some(stream.instance_id());
            self.analysis_last_position = position;
            return self.analysis_window();
        }

        self.analysis_last_position = position;
        if needed == 0 {
            return self.analysis_window();
        }

        let Some(playback) = self.analysis_playback.as_mut() else {
            return Vec::new();
        };
        let mixed = playback.mix_audio(pitch as f32, needed);

        for frame in mixed.as_slice() {
            self.analysis_buffer.push((frame.x + frame.y) * 0.5);
        }
        if self.analysis_buffer.len() > ANALYSIS_WINDOW {
            let excess = self.analysis_buffer.len() - ANALYSIS_WINDOW;
            self.analysis_buffer.drain(..excess);
        }

        // Short read means the playback hit the end of the clip and went
        // inactive; drop it so the next call starts a fresh one.
        if mixed.len() < needed as usize {
            self.analysis_playback = None;
        }

        self.analysis_window()
    }

    fn analysis_window(&self) -> Vec<f32> {
        self.analysis_buffer.clone()
    }
}
