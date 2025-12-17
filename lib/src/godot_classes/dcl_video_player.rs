use crate::av::backend::BackendType;
use godot::engine::ImageTexture;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(init, base=AudioStreamPlayer)]
pub struct DclVideoPlayer {
    // Used to mute and restore the volume
    dcl_volume: f32,

    // Track muted state to know if we should apply volume changes
    is_muted: bool,

    #[export]
    dcl_source: GString,

    #[export]
    dcl_texture: Option<Gd<ImageTexture>>,

    base: Base<AudioStreamPlayer>,

    #[var]
    dcl_scene_id: i32,

    /// The current backend type being used
    backend_type: BackendType,

    pub resolve_resource_sender: Option<tokio::sync::oneshot::Sender<String>>,
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

    #[func]
    pub fn get_dcl_volume(&self) -> f32 {
        self.dcl_volume
    }

    #[func]
    pub fn set_dcl_volume(&mut self, value: f32) {
        self.dcl_volume = value;
        // Update the actual audio volume if not muted
        if !self.is_muted {
            let db_volume = if value <= 0.0 {
                -80.0
            } else {
                20.0 * f32::log10(value)
            };
            self.base_mut().set_volume_db(db_volume);
        }
    }

    pub fn set_muted(&mut self, value: bool) {
        self.is_muted = value;
        if value {
            self.base_mut().set_volume_db(-80.0);
        } else {
            let db_volume = if self.dcl_volume <= 0.0 {
                -80.0
            } else {
                20.0 * f32::log10(self.dcl_volume)
            };
            self.base_mut().set_volume_db(db_volume);
        }
    }

    /// Initialize the backend for this video player.
    /// This calls into GDScript to set up the appropriate backend (ExoPlayer, LiveKit, etc.)
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

        // Call the GDScript implementation to actually initialize the backend
        // Note: We use source.clone() above and pass source here to avoid borrow issues
        self.base_mut().call(
            "_init_backend_impl".into(),
            &[
                backend_type.to_variant(),
                source.to_variant(),
                playing.to_variant(),
                looping.to_variant(),
            ],
        );
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

    /// Send a play command to the backend
    #[func]
    pub fn backend_play(&mut self) {
        self.base_mut().call("_backend_play".into(), &[]);
    }

    /// Send a pause command to the backend
    #[func]
    pub fn backend_pause(&mut self) {
        self.base_mut().call("_backend_pause".into(), &[]);
    }

    /// Set the looping state on the backend
    #[func]
    pub fn backend_set_looping(&mut self, looping: bool) {
        self.base_mut()
            .call("_backend_set_looping".into(), &[looping.to_variant()]);
    }

    /// Dispose the backend and clean up resources
    #[func]
    pub fn backend_dispose(&mut self) {
        self.base_mut().call("_backend_dispose".into(), &[]);
        self.backend_type = BackendType::Noop;
    }

    /// Get the texture from the backend (for ExoPlayer this comes from ExternalTexture)
    /// Note: This needs &mut self because Godot's call() requires mutable access
    #[func]
    pub fn get_backend_texture(&mut self) -> Option<Gd<godot::engine::Texture2D>> {
        let result = self.base_mut().call("_get_backend_texture".into(), &[]);
        result.try_to::<Gd<godot::engine::Texture2D>>().ok()
    }
}
