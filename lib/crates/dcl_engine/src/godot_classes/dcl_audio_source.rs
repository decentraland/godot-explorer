use godot::classes::audio_stream_player_3d::AttenuationModel;
use godot::classes::{AudioStream, AudioStreamPlayback, AudioStreamPlayer3D, IAudioStreamPlayer3D};
use godot::prelude::*;

use crate::engine::local_task::{await_promise, spawn_local};
use crate::fetcher::globals;
use crate::godot_classes::promise::PromiseError;

/// Linear volume units per second, matching Unity's `AudioSourcesPlugin.FadeSpeed`.
const FADE_SPEED: f32 = 1.0;
const MIN_VOLUME_DB: f32 = -80.0;
/// Distance where attenuation starts. Unity leaves `minDistance` at its default of 1 m; Godot's
/// default `unit_size` of 10 is +20 dB louder past a metre.
const UNIT_SIZE: f32 = 1.0;
/// Top of Godot's `attenuation_filter_cutoff_hz` range: the high-shelf ends up above the audible
/// band, which is how the filter is switched off.
const FILTER_DISABLED_HZ: f32 = 20500.0;

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

/// The gain (dB) of a faded linear volume.
pub fn gain_db(volume: f32) -> f32 {
    if volume <= 0.0 {
        MIN_VOLUME_DB
    } else {
        (20.0 * volume.log10()).max(MIN_VOLUME_DB)
    }
}

/// One step of the fade toward `target`.
pub fn fade_step(current: f32, target: f32, delta: f32) -> f32 {
    if current < target {
        target.min(current + delta * FADE_SPEED)
    } else {
        target.max(current - delta * FADE_SPEED)
    }
}

#[derive(GodotClass)]
#[class(base=AudioStreamPlayer3D)]
pub struct DclAudioSource {
    #[var(pub)]
    dcl_audio_clip_url: GString,

    #[var(pub)]
    dcl_playing: bool,

    #[var(pub)]
    dcl_volume: f32,

    #[var(pub)]
    dcl_loop_activated: bool,

    #[var(pub)]
    dcl_pitch: f32,

    #[var(pub)]
    dcl_global: bool,

    #[var(pub)]
    dcl_current_time: f32,

    #[var(pub)]
    dcl_enable: bool,

    #[var(pub)]
    dcl_scene_id: i32,

    /// See `CLIP_STATE_*`.
    #[var(pub)]
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

    last_loaded_audio_clip: String,
    valid: bool,
    time_specified: bool,
    /// Faded gain, ramping toward `dcl_volume` inside the scene and toward 0 outside.
    faded_volume: f32,
    fade_initialized: bool,
    last_sdk_volume: f32,

    base: Base<AudioStreamPlayer3D>,
}

#[godot_api]
impl IAudioStreamPlayer3D for DclAudioSource {
    fn init(base: Base<AudioStreamPlayer3D>) -> Self {
        Self {
            dcl_audio_clip_url: GString::new(),
            dcl_playing: false,
            dcl_volume: 0.0,
            dcl_loop_activated: false,
            dcl_pitch: 0.0,
            dcl_global: false,
            dcl_current_time: 0.0,
            dcl_enable: false,
            dcl_scene_id: 0,
            dcl_clip_state: CLIP_STATE_NONE,
            last_media_state: 0,
            analysis_playback: None,
            analysis_stream_id: None,
            analysis_last_position: 0.0,
            analysis_buffer: Vec::new(),
            last_loaded_audio_clip: String::new(),
            valid: false,
            time_specified: false,
            faded_volume: 0.0,
            fade_initialized: false,
            last_sdk_volume: -1.0,
            base,
        }
    }

    fn ready(&mut self) {
        let this = self.to_gd();
        self.base_mut()
            .signals()
            .finished()
            .connect_other(&this, |this: &mut Self| this.on_finished());
    }

    fn process(&mut self, delta: f64) {
        let target = if self.dcl_enable {
            self.dcl_volume
        } else {
            0.0
        };
        if (self.faded_volume - target).abs() < 1e-5 {
            return;
        }
        self.faded_volume = fade_step(self.faded_volume, target, delta as f32);
        self.apply_volume();
    }
}

#[godot_api]
impl DclAudioSource {
    #[func]
    fn on_finished(&mut self) {
        if self.dcl_loop_activated {
            self.base_mut().play();
        }
    }
}

impl DclAudioSource {
    /// Applies the SDK properties to the player. With `action_on_playing` it also plays or stops.
    pub fn apply_audio_props(&mut self, action_on_playing: bool) {
        self.sync_volume_from_sdk();
        if !self.valid {
            return;
        }
        let pitch = self.dcl_pitch;
        let model = if self.dcl_global {
            AttenuationModel::DISABLED
        } else {
            AttenuationModel::INVERSE_DISTANCE
        };
        let mut base = self.base_mut();
        base.set_pitch_scale(pitch);
        base.set_attenuation_model(model);
        base.set_unit_size(UNIT_SIZE);
        // Godot's `max_distance` hard-mutes past the limit and adds a linear ramp across the
        // whole range, so it cannot stand in for Unity's `maxDistance`.
        base.set_max_distance(0.0);
        // Godot ties a high-shelf to the attenuation multiplier: a source at 5 m would lose
        // 19 dB above 5 kHz on top of the gain. Unity's rolloff is gain-only, so push the
        // filter above hearing.
        base.set_attenuation_filter_cutoff_hz(FILTER_DISABLED_HZ);
        drop(base);

        if action_on_playing {
            let playing = self.base().is_playing();
            if playing && !self.dcl_playing {
                self.base_mut().stop();
            } else if self.dcl_playing && (!playing || self.time_specified) {
                let from = self.dcl_current_time;
                self.base_mut().play_ex().from_position(from).done();
            }
        }
    }

    /// An SDK `volume` change snaps; only entering or leaving the scene crossfades.
    fn sync_volume_from_sdk(&mut self) {
        if !self.fade_initialized {
            self.fade_initialized = true;
            // Spawned outside the scene means silent, so entering fades in instead of blasting a
            // neighbouring scene's first frames (unity-explorer#4790).
            self.faded_volume = if self.dcl_enable {
                self.dcl_volume
            } else {
                0.0
            };
        } else if self.dcl_enable && (self.dcl_volume - self.last_sdk_volume).abs() > 1e-5 {
            self.faded_volume = self.dcl_volume;
        }
        self.last_sdk_volume = self.dcl_volume;
        self.apply_volume();
    }

    fn apply_volume(&mut self) {
        let gain = gain_db(self.faded_volume);
        let mut base = self.base_mut();
        base.set_volume_db(gain);
        // Godot clamps `attenuation + volume_db` to `max_db`. Clamping at 0 would drop the SDK
        // volume wherever attenuation is positive; clamping at `volume_db` gives
        // `volume * min(1, unit_size / distance)`, which is Unity's rolloff.
        base.set_max_db(gain);
    }

    /// Loads the clip when it changed, then applies the properties. `time_specified`: the scene
    /// set a start time, so a playing source restarts from it.
    pub fn refresh_data(&mut self, time_specified: bool) {
        self.dcl_audio_clip_url =
            GString::from(&self.dcl_audio_clip_url.to_string().to_lowercase());
        self.time_specified = time_specified;
        let url = self.dcl_audio_clip_url.to_string();

        if self.last_loaded_audio_clip == url {
            self.apply_audio_props(true);
            return;
        }
        let mapping = globals::scene_runner()
            .bind()
            .get_scene_content_mapping(self.dcl_scene_id);
        self.last_loaded_audio_clip = url.clone();
        self.valid = false;

        if mapping.bind().get_hash(url.as_str().into()).is_empty() {
            self.dcl_clip_state = if url.is_empty() {
                CLIP_STATE_NONE
            } else {
                CLIP_STATE_ERROR
            };
            return;
        }
        self.dcl_clip_state = CLIP_STATE_LOADING;
        let promise = globals::content_provider()
            .bind_mut()
            .fetch_audio(url.as_str().into(), mapping);
        let source = self.to_gd();
        spawn_local(async move {
            let result = await_promise(promise).await;
            if !source.is_instance_valid() {
                return;
            }
            source.clone().bind_mut().on_audio_fetched(result, &url);
        });
    }

    fn on_audio_fetched(&mut self, result: Variant, url: &str) {
        if let Ok(error) = result.try_to::<Gd<PromiseError>>() {
            self.base_mut().stop();
            self.base_mut().set_stream(Gd::null_arg());
            self.dcl_clip_state = CLIP_STATE_ERROR;
            godot_error!("Error on fetch audio: {}", error.bind().get_error());
            return;
        }
        // A resolved promise carrying no stream is still a failure: it used to pass as READY and
        // leave a silent player behind (#2741).
        let Ok(stream) = result.try_to::<Gd<AudioStream>>() else {
            self.base_mut().stop();
            self.base_mut().set_stream(Gd::null_arg());
            self.valid = false;
            self.dcl_clip_state = CLIP_STATE_ERROR;
            godot_error!("Audio clip resolved with no stream: {url}");
            return;
        };
        self.base_mut().set_stream(&stream);
        self.valid = true;
        self.dcl_clip_state = CLIP_STATE_READY;
        self.apply_audio_props(true);
    }
}

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
