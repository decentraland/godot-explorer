use godot::classes::image::Format;
use godot::classes::{
    AudioStreamGenerator, AudioStreamGeneratorPlayback, AudioStreamPlayer, IAudioStreamPlayer,
    Image, ImageTexture, Os, Texture2D, Time,
};
use godot::prelude::*;

use crate::av::backend::BackendType;
use crate::engine::local_task::{await_promise, next_frame, spawn_local};
use crate::fetcher::globals;
use crate::media::native_player::{Flavor, NativePlayer};
use crate::media::{debounce, native_effective_volume, Debounce};

/// Video state constants (matching the SDK VideoState), updated here and polled by the scene
/// runner.
pub const VIDEO_STATE_NONE: i32 = 0;
pub const VIDEO_STATE_LOADING: i32 = 1;
pub const VIDEO_STATE_READY: i32 = 2;
pub const VIDEO_STATE_PLAYING: i32 = 3;
pub const VIDEO_STATE_BUFFERING: i32 = 4;
pub const VIDEO_STATE_SEEKING: i32 = 5;
pub const VIDEO_STATE_PAUSED: i32 = 6;
pub const VIDEO_STATE_ERROR: i32 = 7;

/// Seconds without a LiveKit frame before the stream reads as buffering.
const LIVEKIT_BUFFERING_THRESHOLD: f64 = 2.0;
/// Seconds without a LiveKit frame before the stream reads as stopped.
const LIVEKIT_STOPPED_THRESHOLD: f64 = 10.0;

thread_local! {
    /// A black texture shared by every video player until its first valid frame.
    static BLACK_PLACEHOLDER: std::cell::RefCell<Option<Gd<ImageTexture>>> =
        const { std::cell::RefCell::new(None) };
}

fn black_placeholder() -> Gd<ImageTexture> {
    BLACK_PLACEHOLDER.with(|slot| {
        slot.borrow_mut()
            .get_or_insert_with(|| {
                let mut image = Image::create(2, 2, false, Format::RGBA8).expect("2x2 image");
                image.fill(Color::BLACK);
                ImageTexture::create_from_image(&image).expect("placeholder texture")
            })
            .clone()
    })
}

/// The state of a native player, as the SDK sees it. `is_playing` takes priority over the
/// duration: a live HLS stream reports duration 0 while playing, which would otherwise keep the
/// state at LOADING and the texture hidden.
pub fn native_video_state(current: i32, duration: f32, is_playing: bool) -> i32 {
    if is_playing {
        VIDEO_STATE_PLAYING
    } else if duration <= 0.0 {
        VIDEO_STATE_LOADING
    } else if current == VIDEO_STATE_LOADING {
        VIDEO_STATE_READY
    } else if current != VIDEO_STATE_READY {
        VIDEO_STATE_PAUSED
    } else {
        current
    }
}

/// How a LiveKit stream reads given the time since its last frame: `(state, got_frame_flag)`.
pub fn livekit_state(current: i32, since_last_frame: f64) -> (i32, Option<bool>) {
    match current {
        VIDEO_STATE_PLAYING if since_last_frame > LIVEKIT_BUFFERING_THRESHOLD => {
            (VIDEO_STATE_BUFFERING, None)
        }
        // No frames for a long time: the stream is stopped, show black.
        VIDEO_STATE_BUFFERING if since_last_frame > LIVEKIT_STOPPED_THRESHOLD => {
            (VIDEO_STATE_PAUSED, Some(false))
        }
        other => (other, None),
    }
}

/// A video component's player: LiveKit streams, ExoPlayer on Android, AVPlayer on iOS, or
/// nothing.
#[derive(GodotClass)]
#[class(init, base=AudioStreamPlayer)]
pub struct DclVideoPlayer {
    // The video's own volume (0.0-1.0)
    #[var(pub)]
    dcl_volume: f32,

    // Muted state (e.g., when not in current scene)
    #[var(pub)]
    dcl_muted: bool,

    #[export]
    #[var(pub)]
    dcl_source: GString,

    #[export]
    #[var(pub)]
    dcl_texture: Option<Gd<ImageTexture>>,

    base: Base<AudioStreamPlayer>,

    #[var(pub)]
    dcl_scene_id: i32,

    /// The current backend type being used
    backend_type: BackendType,

    pub resolve_resource_sender: Option<tokio::sync::oneshot::Sender<String>>,

    /// Current video state (see VIDEO_STATE_* constants)
    #[var(pub)]
    video_state: i32,

    /// Current playback position in seconds
    #[var(pub)]
    video_position: f64,

    /// Total video length in seconds (-1 if unknown)
    #[var(pub)]
    video_length: f64,

    /// Timestamp of last received frame (for LiveKit buffering detection)
    /// This is set by Rust when frames arrive
    #[var(pub)]
    last_frame_time: f64,

    /// The native player node of the ExoPlayer and AVPlayer backends.
    native: Option<Gd<NativePlayer>>,
    /// Bumped by `backend_dispose`: an init that resumes with a stale generation belongs to a
    /// backend disposed meanwhile and stops.
    backend_generation: u32,
    source: String,
    /// Whether the scene wants it playing.
    pub is_playing: bool,
    is_looping: bool,
    /// -1 means not applied yet.
    last_effective_volume: f32,
    last_play_pause_ms: u64,
    /// -1 none, 0 pause, 1 play.
    pending_play_state: i32,
    /// A valid frame was received: until then a native texture holds garbage.
    has_received_frame: bool,
}

#[godot_api]
impl IAudioStreamPlayer for DclVideoPlayer {
    fn process(&mut self, _delta: f64) {
        if self.backend_type == BackendType::Noop {
            return;
        }
        self.process_pending_play_state();
        self.update_effective_volume();
        self.update_video_state();

        // A native frame needs both a PLAYING state and a delivered GPU frame, else there is a
        // gap where it plays but the ExternalTexture holds garbage (pink).
        if !self.has_received_frame && self.video_state == VIDEO_STATE_PLAYING {
            if let Some(native) = self.native.as_ref() {
                if native.bind().frame_count > 0 {
                    self.has_received_frame = true;
                }
            }
        }
    }
}

#[godot_api]
impl DclVideoPlayer {
    #[func]
    fn resolve_resource(&mut self, file_path: GString) {
        let Some(sender) = self.resolve_resource_sender.take() else {
            return;
        };
        let _ = sender.send(file_path.to_string());
    }

    /// Set the video's own volume (0.0-1.0)
    pub fn set_volume(&mut self, value: f32) {
        self.dcl_volume = value;
    }

    /// Set the muted state (e.g., when not in current scene)
    pub fn set_muted(&mut self, value: bool) {
        self.dcl_muted = value;
    }

    /// Initialize the backend for this video player.
    #[func]
    pub fn init_backend(
        &mut self,
        backend_type: i32,
        source: GString,
        playing: bool,
        looping: bool,
    ) {
        self.backend_type = match backend_type {
            0 => BackendType::LiveKit,
            1 => BackendType::ExoPlayer,
            2 => BackendType::AVPlayer,
            _ => BackendType::Noop,
        };
        self.dcl_source = source.clone();

        tracing::debug!(
            "DclVideoPlayer::init_backend - type={:?}, source={}, playing={}, looping={}",
            self.backend_type,
            self.dcl_source,
            playing,
            looping
        );

        // A previous backend is cleaned up first; it resets the type, so keep the new one.
        let requested = self.backend_type;
        self.backend_dispose();
        self.backend_type = requested;
        self.source = source.to_string();
        self.is_playing = playing;
        self.is_looping = looping;

        match requested {
            BackendType::LiveKit => self.init_livekit_backend(),
            BackendType::ExoPlayer => self.start_native_backend(Flavor::Exo),
            BackendType::AVPlayer => self.start_native_backend(Flavor::Av),
            BackendType::Noop => self.init_noop_backend(),
        }
    }

    /// Get the current backend type as an integer (for GDScript interop)
    #[func]
    pub fn get_backend_type(&self) -> i32 {
        self.backend_type.to_gd_int()
    }

    /// Check if this is a LiveKit backend
    pub fn is_livekit(&self) -> bool {
        self.backend_type == BackendType::LiveKit
    }

    /// Check if this backend uses ExternalTexture (ExoPlayer/AVPlayer).
    ///
    /// ExternalTexture in Godot does not create an sRGB texture view, so
    /// the `source_color` shader hint has no effect — hardware sRGB→linear
    /// conversion never happens. Materials using these textures need
    /// `ALBEDO_TEXTURE_FORCE_SRGB = true` to get the conversion in the shader.
    ///
    /// In contrast, LiveKit uses ImageTexture which does have an sRGB view,
    /// so `source_color` already handles the conversion and FORCE_SRGB must
    /// be false to avoid double conversion.
    pub fn uses_external_texture(&self) -> bool {
        matches!(
            self.backend_type,
            BackendType::ExoPlayer | BackendType::AVPlayer
        )
    }

    /// Send a play command to the backend, debounced.
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

    /// Send a pause command to the backend, debounced.
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

    /// Set the looping state on the backend
    #[func]
    pub fn backend_set_looping(&mut self, looping: bool) {
        self.is_looping = looping;
        if let Some(native) = self.native.as_mut() {
            native.bind_mut().set_looping(looping);
        }
    }

    /// Seek to a specific position in seconds (a LiveKit stream is live: no seeking)
    #[func]
    pub fn backend_seek(&mut self, position: f32) {
        if let Some(native) = self.native.as_mut() {
            native.bind_mut().set_position(position);
        }
    }

    /// Set the playback rate (1.0 = normal speed; not for LiveKit)
    #[func]
    pub fn backend_set_playback_rate(&mut self, rate: f32) {
        if let Some(native) = self.native.as_mut() {
            native.bind_mut().set_playback_rate(rate);
        }
    }

    /// Dispose the backend and clean up resources
    #[func]
    pub fn backend_dispose(&mut self) {
        self.backend_generation += 1;
        if let Some(mut native) = self.native.take() {
            native.queue_free();
        }
        self.backend_type = BackendType::Noop;
        self.source.clear();
        self.has_received_frame = false;
        // Reset: a new source triggers a state change event.
        self.video_state = VIDEO_STATE_NONE;
        self.video_position = 0.0;
        self.video_length = -1.0;
        self.last_effective_volume = -1.0;
    }

    /// The texture of the backend. Before the first valid frame an ExternalTexture may hold GPU
    /// garbage, so a black placeholder is returned; after it the backend texture always, so pause
    /// and buffering keep the last frame.
    #[func]
    pub fn get_backend_texture(&mut self) -> Option<Gd<Texture2D>> {
        if !self.has_received_frame {
            return Some(black_placeholder().upcast());
        }
        match self.backend_type {
            BackendType::ExoPlayer | BackendType::AVPlayer => {
                self.native.as_ref().and_then(|n| n.bind().get_texture())
            }
            // The LiveKit texture is set from Rust.
            BackendType::LiveKit => self.dcl_texture.clone().map(|t| t.upcast()),
            BackendType::Noop => None,
        }
    }

    /// Sets the generator's sample rate for a LiveKit stream, clearing stale audio.
    pub fn init_livekit_audio(&mut self, sample_rate: u32) {
        if self.backend_type != BackendType::LiveKit {
            return;
        }
        if let Some(stream) = self.base().get_stream() {
            if let Ok(mut generator) = stream.try_cast::<AudioStreamGenerator>() {
                generator.set_mix_rate(sample_rate as f32);
                self.clear_audio_buffer();
            }
        }
    }

    pub fn clear_audio_buffer(&mut self) {
        if self.backend_type != BackendType::LiveKit {
            return;
        }
        if let Some(playback) = self.base().get_stream_playback() {
            if let Ok(mut playback) = playback.try_cast::<AudioStreamGeneratorPlayback>() {
                playback.clear_buffer();
            }
        }
    }

    /// Pushes PCM of a LiveKit stream.
    pub fn stream_buffer(&mut self, data: PackedVector2Array) {
        if self.backend_type != BackendType::LiveKit {
            return;
        }
        if !self.base().is_playing() {
            self.base_mut().play();
        }
        if let Some(playback) = self.base().get_stream_playback() {
            if let Ok(mut playback) = playback.try_cast::<AudioStreamGeneratorPlayback>() {
                if playback.get_frames_available() as usize >= data.len() {
                    playback.push_buffer(&data);
                }
            }
        }
    }
}

impl DclVideoPlayer {
    fn init_livekit_backend(&mut self) {
        godot_print!(
            "VideoPlayer: Initializing LiveKit backend for {}",
            self.source
        );
        // Rust sets PLAYING when frames arrive.
        self.video_state = VIDEO_STATE_LOADING;
        // Audio through an AudioStreamGenerator; video frames are pushed from Rust.
        let mut generator = AudioStreamGenerator::new_gd();
        generator.set_mix_rate(48000.0);
        generator.set_buffer_length(1.5);
        self.base_mut().set_stream(&generator);
        self.base_mut().play();
    }

    fn init_noop_backend(&mut self) {
        godot_print!("VideoPlayer: Using Noop backend (video playback not available)");
        self.video_state = VIDEO_STATE_NONE;
        self.video_position = 0.0;
        self.video_length = -1.0;
    }

    fn start_native_backend(&mut self, flavor: Flavor) {
        let (os, label) = match flavor {
            Flavor::Exo => ("Android", "ExoPlayer"),
            Flavor::Av => ("iOS", "AVPlayer"),
        };
        if Os::singleton().get_name() != os {
            godot_warn!("{label} backend only available on {os}, falling back to Noop");
            self.backend_type = BackendType::Noop;
            self.init_noop_backend();
            return;
        }
        godot_print!(
            "VideoPlayer: Initializing {label} backend for {}",
            self.source
        );
        self.video_state = VIDEO_STATE_LOADING;

        let native = NativePlayer::create(flavor);
        self.base_mut().add_child(&native);
        self.native = Some(native);
        let generation = self.backend_generation;
        spawn_local(init_native_backend(self.to_gd(), flavor, generation));
    }

    fn process_pending_play_state(&mut self) {
        if self.pending_play_state < 0 {
            return;
        }
        let now = Time::singleton().get_ticks_msec();
        if debounce(now, self.last_play_pause_ms) == Debounce::Hold {
            return;
        }
        if self.pending_play_state == 1 {
            self.apply_play();
        } else {
            self.apply_pause();
        }
        self.pending_play_state = -1;
    }

    fn apply_play(&mut self) {
        self.is_playing = true;
        // Force a volume recalculation.
        self.last_effective_volume = -1.0;
        if let Some(native) = self.native.as_mut() {
            native.bind_mut().play();
        }
        // LiveKit: Rust resumes updating the texture while `is_playing`.
    }

    fn apply_pause(&mut self) {
        self.is_playing = false;
        // Force a volume recalculation (mutes the LiveKit audio).
        self.last_effective_volume = -1.0;
        if let Some(native) = self.native.as_mut() {
            native.bind_mut().pause();
        }
        if self.backend_type == BackendType::LiveKit {
            // Freeze the last frame: Rust stops updating the texture while paused.
            self.video_state = VIDEO_STATE_PAUSED;
        }
    }

    fn update_effective_volume(&mut self) {
        match self.backend_type {
            BackendType::ExoPlayer | BackendType::AVPlayer => {
                let volume = native_effective_volume(self.dcl_muted, self.dcl_volume);
                if (volume - self.last_effective_volume).abs() < 0.001 {
                    return;
                }
                if let Some(native) = self.native.as_mut() {
                    self.last_effective_volume = volume;
                    native.bind_mut().set_volume(volume);
                }
            }
            BackendType::LiveKit => {
                // Godot's buses handle master and scene volume: only the video's own applies.
                let volume = if self.dcl_muted || !self.is_playing {
                    0.0
                } else {
                    self.dcl_volume
                };
                if (volume - self.last_effective_volume).abs() < 0.001 {
                    return;
                }
                self.last_effective_volume = volume;
                let db = if volume <= 0.0 {
                    -80.0
                } else {
                    (20.0 * volume.log10()).max(-80.0)
                };
                self.base_mut().set_volume_db(db);
            }
            BackendType::Noop => {}
        }
    }

    /// Updates the state variables the scene runner polls to generate CRDT events.
    fn update_video_state(&mut self) {
        match self.backend_type {
            BackendType::ExoPlayer | BackendType::AVPlayer => {
                let Some(native) = self.native.as_mut() else {
                    return;
                };
                let (duration, position, playing) = {
                    let mut native = native.bind_mut();
                    (
                        native.get_duration(),
                        native.get_position(),
                        native.is_playing(),
                    )
                };
                self.video_position = position as f64;
                if duration > 0.0 {
                    self.video_length = duration as f64;
                }
                self.video_state = native_video_state(self.video_state, duration, playing);
            }
            BackendType::LiveKit => {
                // Rust sets PLAYING when frames arrive; stalls are detected here. A user pause is
                // managed by apply_pause/apply_play.
                if self.is_playing && self.last_frame_time > 0.0 {
                    let now = Time::singleton().get_ticks_msec() as f64 / 1000.0;
                    let (state, flag) = livekit_state(self.video_state, now - self.last_frame_time);
                    self.video_state = state;
                    if let Some(received) = flag {
                        self.has_received_frame = received;
                    }
                }
                if self.video_state == VIDEO_STATE_PLAYING {
                    self.has_received_frame = true;
                }
            }
            BackendType::Noop => {}
        }
    }

    /// The absolute path of a local video once downloaded; empty on error (the state is ERROR).
    async fn fetch_local_video(player: &Gd<Self>) -> String {
        let (source, scene_id) = {
            let this = player.bind();
            (this.source.clone(), this.dcl_scene_id)
        };
        let mapping = globals::scene_runner()
            .bind()
            .get_scene_content_mapping(scene_id);
        let hash = mapping.bind().get_hash(source.as_str().into()).to_string();
        if hash.is_empty() {
            godot_error!("VideoPlayer: Could not find hash for local file: {source}");
            player.clone().bind_mut().video_state = VIDEO_STATE_ERROR;
            return String::new();
        }
        let promise = globals::content_provider()
            .bind_mut()
            .fetch_video(hash.as_str().into(), mapping);
        let result = await_promise(promise).await;
        if let Ok(error) = result.try_to::<Gd<crate::godot_classes::promise::PromiseError>>() {
            godot_error!(
                "VideoPlayer: Error fetching video: {}",
                error.bind().get_error()
            );
            player.clone().bind_mut().video_state = VIDEO_STATE_ERROR;
            return String::new();
        }
        let cache_path = globals::content_provider()
            .bind()
            .get_cache_file_path(hash.as_str().into())
            .to_string();
        cache_path.replace("user:/", &Os::singleton().get_user_data_dir().to_string())
    }
}

/// The part of the native backend setup that waits a frame for the child to be ready.
async fn init_native_backend(player: Gd<DclVideoPlayer>, flavor: Flavor, generation: u32) {
    next_frame().await;
    if !player.is_instance_valid() || player.bind().backend_generation != generation {
        // A source change disposed this backend while it was getting ready.
        return;
    }
    let label = match flavor {
        Flavor::Exo => "ExoPlayer",
        Flavor::Av => "AVPlayer",
    };
    let (mut native, source, looping, playing) = {
        let this = player.bind();
        let Some(native) = this.native.clone() else {
            return;
        };
        (
            native,
            this.source.clone(),
            this.is_looping,
            this.is_playing,
        )
    };

    // The texture starts at an initial size; it is resized when the video loads.
    if !native.bind_mut().init_texture(640, 360) {
        godot_error!("VideoPlayer: Failed to initialize {label} texture");
        let mut this = player.clone();
        let mut this = this.bind_mut();
        this.video_state = VIDEO_STATE_ERROR;
        native.queue_free();
        this.native = None;
        return;
    }

    if !(source.starts_with("http://") || source.starts_with("https://")) {
        // A local file is fetched first.
        let path = DclVideoPlayer::fetch_local_video(&player).await;
        if path.is_empty()
            || !player.is_instance_valid()
            || player.bind().backend_generation != generation
        {
            return;
        }
        // AVPlayer needs a file extension to know the format.
        let path = if flavor == Flavor::Av {
            ensure_file_has_extension(&source, &path)
        } else {
            path
        };
        let ok = native.bind_mut().set_source_local(&path);
        let mut this = player.clone();
        let mut this = this.bind_mut();
        if !ok {
            godot_error!("VideoPlayer: Failed to set {label} local source: {path}");
            this.video_state = VIDEO_STATE_ERROR;
            return;
        }
        native.bind_mut().set_looping(this.is_looping);
        if this.is_playing {
            native.bind_mut().play();
        }
        return;
    }

    let ok = native.bind_mut().set_source_url(&source);
    if !ok {
        godot_error!("VideoPlayer: Failed to set {label} source: {source}");
        let mut this = player.clone();
        let mut this = this.bind_mut();
        this.video_state = VIDEO_STATE_ERROR;
        native.queue_free();
        this.native = None;
        return;
    }
    native.bind_mut().set_looping(looping);
    if playing {
        native.bind_mut().play();
    }
}

/// `absolute` with the source's extension: a copy is made when needed.
fn ensure_file_has_extension(source: &str, absolute: &str) -> String {
    let Some(extension) = std::path::Path::new(source)
        .extension()
        .and_then(|e| e.to_str())
    else {
        return absolute.to_string();
    };
    if extension.is_empty() {
        return absolute.to_string();
    }
    let with_extension = format!("{absolute}.{extension}");
    if std::path::Path::new(&with_extension).exists() {
        return with_extension;
    }
    match std::fs::copy(absolute, &with_extension) {
        Ok(_) => with_extension,
        Err(error) => {
            godot_warn!(
                "VideoPlayer: Failed to create file with extension, trying without: {error}"
            );
            absolute.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_native_player_that_plays_is_playing_even_for_live_streams() {
        assert_eq!(
            native_video_state(VIDEO_STATE_LOADING, 0.0, true),
            VIDEO_STATE_PLAYING
        );
    }

    #[test]
    fn no_duration_means_loading() {
        assert_eq!(
            native_video_state(VIDEO_STATE_NONE, 0.0, false),
            VIDEO_STATE_LOADING
        );
    }

    #[test]
    fn loading_with_a_duration_becomes_ready_then_paused() {
        assert_eq!(
            native_video_state(VIDEO_STATE_LOADING, 10.0, false),
            VIDEO_STATE_READY
        );
        assert_eq!(
            native_video_state(VIDEO_STATE_READY, 10.0, false),
            VIDEO_STATE_READY
        );
        assert_eq!(
            native_video_state(VIDEO_STATE_PLAYING, 10.0, false),
            VIDEO_STATE_PAUSED
        );
    }

    #[test]
    fn livekit_stalls_read_as_buffering_then_stopped() {
        assert_eq!(
            livekit_state(VIDEO_STATE_PLAYING, 1.0),
            (VIDEO_STATE_PLAYING, None)
        );
        assert_eq!(
            livekit_state(VIDEO_STATE_PLAYING, 3.0),
            (VIDEO_STATE_BUFFERING, None)
        );
        assert_eq!(
            livekit_state(VIDEO_STATE_BUFFERING, 5.0),
            (VIDEO_STATE_BUFFERING, None)
        );
        assert_eq!(
            livekit_state(VIDEO_STATE_BUFFERING, 11.0),
            (VIDEO_STATE_PAUSED, Some(false))
        );
    }

    #[test]
    fn extension_is_kept_when_there_is_none() {
        assert_eq!(ensure_file_has_extension("video", "/tmp/hash"), "/tmp/hash");
    }
}
