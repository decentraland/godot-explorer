//! A video decoded by the platform: ExoPlayer on Android and AVPlayer on iOS, through the video
//! backend the native host registers with the core (D-41). Frames reach the GPU as an
//! `ExternalTexture` over an `AHardwareBuffer` or an `IOSurface`, with no readback. The texture is
//! resized when the real video size becomes known.

use std::sync::Arc;

use dcl_core::platform::VideoHost;
use godot::classes::{ExternalTexture, INode, Node, Os, Texture2D};
use godot::prelude::*;

use crate::engine::frame_budget;
use crate::engine::local_task::spawn_local;

/// The platform player a node drives.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Flavor {
    Exo,
    Av,
}

impl Flavor {
    fn label(self) -> &'static str {
        match self {
            Flavor::Exo => "ExoPlayer",
            Flavor::Av => "AVPlayer",
        }
    }

    fn os_name(self) -> &'static str {
        match self {
            Flavor::Exo => "Android",
            Flavor::Av => "iOS",
        }
    }
}

#[derive(GodotClass)]
#[class(no_init, base=Node)]
pub struct NativePlayer {
    flavor: Flavor,
    host: Option<&'static Arc<dyn VideoHost>>,
    player_id: i32,
    external_texture: Option<Gd<ExternalTexture>>,
    video_width: i32,
    video_height: i32,
    /// Frames handed to the texture: the video player waits for one before showing the texture,
    /// which holds garbage until then.
    pub frame_count: u64,
    /// Texture updates are skipped while the surface is rebuilt, or it flickers grey.
    reinitializing_surface: bool,
    base: Base<Node>,
}

impl NativePlayer {
    pub fn create(flavor: Flavor) -> Gd<Self> {
        Gd::from_init_fn(|base| Self {
            flavor,
            host: None,
            player_id: -1,
            external_texture: None,
            video_width: 0,
            video_height: 0,
            frame_count: 0,
            reinitializing_surface: false,
            base,
        })
    }

    /// The backend and this node's player, once both exist.
    fn player(&self) -> Option<(&'static Arc<dyn VideoHost>, i32)> {
        self.host
            .filter(|_| self.player_id > 0)
            .map(|host| (host, self.player_id))
    }

    fn ready_player(&self) -> bool {
        self.player().is_some()
    }

    fn require_player(&self) -> Option<(&'static Arc<dyn VideoHost>, i32)> {
        let player = self.player();
        if player.is_none() {
            godot_error!("{}: Player not initialized", self.flavor.label());
        }
        player
    }
}

#[godot_api]
impl INode for NativePlayer {
    fn ready(&mut self) {
        let flavor = self.flavor;
        if Os::singleton().get_name() != flavor.os_name() {
            godot_warn!("{}: Only supported on {}", flavor.label(), flavor.os_name());
            return;
        }
        let Some(host) = dcl_core::platform::video() else {
            godot_error!(
                "{}: no video backend registered by the host",
                flavor.label()
            );
            return;
        };
        self.player_id = host.create();
        self.host = Some(host);
        if self.player_id <= 0 {
            godot_error!("{}: Failed to create player", flavor.label());
        }
    }

    fn exit_tree(&mut self) {
        if let Some((host, id)) = self.player() {
            host.release(id);
            self.player_id = -1;
        }
    }

    fn process(&mut self, _delta: f64) {
        self.update_texture();
    }
}

impl NativePlayer {
    /// Creates the surface and the texture at `width x height`; call before setting a source.
    pub fn init_texture(&mut self, width: i32, height: i32) -> bool {
        let Some((host, id)) = self.require_player() else {
            return false;
        };
        self.video_width = width;
        self.video_height = height;
        if !host.init_surface(id, width, height) {
            godot_error!("{}: Failed to initialize surface", self.flavor.label());
            return false;
        }
        let mut texture = ExternalTexture::new_gd();
        texture.set_size(Vector2::new(width as f32, height as f32));
        self.external_texture = Some(texture);
        godot_print!(
            "{}: Initialized in GPU mode ({width}x{height})",
            self.flavor.label()
        );
        true
    }

    pub fn set_source_url(&mut self, url: &str) -> bool {
        let Some((host, id)) = self.require_player() else {
            return false;
        };
        let ok = host.set_source(id, url, false);
        if !ok {
            godot_error!("{}: Failed to set source URL: {url}", self.flavor.label());
        }
        ok
    }

    pub fn set_source_local(&mut self, file_path: &str) -> bool {
        let Some((host, id)) = self.require_player() else {
            return false;
        };
        let ok = host.set_source(id, file_path, true);
        if !ok {
            godot_error!(
                "{}: Failed to set local source: {file_path}",
                self.flavor.label()
            );
        }
        ok
    }

    pub fn play(&mut self) {
        if let Some((host, id)) = self.require_player() {
            host.play(id);
        }
    }

    pub fn pause(&mut self) {
        if let Some((host, id)) = self.require_player() {
            host.pause(id);
        }
    }

    pub fn stop(&mut self) {
        if let Some((host, id)) = self.require_player() {
            host.stop(id);
        }
    }

    pub fn set_position(&mut self, seconds: f32) {
        if let Some((host, id)) = self.require_player() {
            host.seek(id, seconds as f64);
        }
    }

    pub fn get_position(&mut self) -> f32 {
        self.player()
            .map_or(0.0, |(host, id)| host.position(id) as f32)
    }

    pub fn get_duration(&mut self) -> f32 {
        self.player()
            .map_or(0.0, |(host, id)| host.duration(id) as f32)
    }

    pub fn is_playing(&mut self) -> bool {
        self.player().is_some_and(|(host, id)| host.is_playing(id))
    }

    pub fn set_volume(&mut self, volume: f32) {
        if let Some((host, id)) = self.require_player() {
            host.set_volume(id, volume.clamp(0.0, 1.0));
        }
    }

    pub fn set_looping(&mut self, looping: bool) {
        if let Some((host, id)) = self.require_player() {
            host.set_looping(id, looping);
        }
    }

    pub fn set_playback_rate(&mut self, rate: f32) {
        if let Some((host, id)) = self.require_player() {
            host.set_rate(id, rate.clamp(0.1, 10.0));
        }
    }

    pub fn get_texture(&self) -> Option<Gd<Texture2D>> {
        self.external_texture.clone().map(|t| t.upcast())
    }

    /// Polls the player: rebuilds the surface when the video size changed, then hands the next
    /// frame to the texture. True when a new frame arrived.
    pub fn update_texture(&mut self) -> bool {
        let Some((host, id)) = self.player() else {
            return false;
        };
        if !self.reinitializing_surface && host.take_size_changed(id) {
            // The surface swap is a heavy one-off: frames are not consumed while it waits its
            // frame-budget turn.
            self.reinitializing_surface = true;
            spawn_local(reinitialize_when_budget_allows(self.to_gd()));
        }
        if self.reinitializing_surface {
            return false;
        }
        let pointer = host.poll_frame(id);
        if pointer == 0 {
            return false;
        }
        self.frame_count += 1;
        // The buffer pointer goes straight to the renderer.
        if let Some(texture) = self.external_texture.as_mut() {
            texture.set_external_buffer_id(pointer);
        }
        true
    }

    fn reinitialize_to_video_size(&mut self) {
        let Some((host, id)) = self.player() else {
            self.reinitializing_surface = false;
            return;
        };
        let (width, height) = host.video_size(id);
        if width > 0 && height > 0 && (width != self.video_width || height != self.video_height) {
            self.reinitialize_surface(width, height);
        }
        self.reinitializing_surface = false;
    }

    /// Rebuilds the surface and the texture at the new size. The external buffer is cleared first:
    /// resizing with the old buffer referenced fails the GPU allocation (VK_ERROR_OUT_OF_DEVICE_MEMORY
    /// on Vulkan) over a size mismatch.
    fn reinitialize_surface(&mut self, width: i32, height: i32) {
        self.reinitializing_surface = true;
        self.video_width = width;
        self.video_height = height;
        if let Some(texture) = self.external_texture.as_mut() {
            texture.set_external_buffer_id(0);
        }
        let initialized = self
            .player()
            .is_some_and(|(host, id)| host.init_surface(id, width, height));
        if !initialized {
            godot_error!(
                "{}: Failed to reinitialize surface with new dimensions",
                self.flavor.label()
            );
            self.reinitializing_surface = false;
            return;
        }
        let texture = self
            .external_texture
            .get_or_insert_with(ExternalTexture::new_gd);
        texture.set_size(Vector2::new(width as f32, height as f32));
        // The next frame from the new surface is valid.
        self.reinitializing_surface = false;
    }
}

async fn reinitialize_when_budget_allows(player: Gd<NativePlayer>) {
    frame_budget::acquire(Some(player.instance_id()), false).await;
    if player.is_instance_valid() && player.bind().ready_player() {
        player.clone().bind_mut().reinitialize_to_video_size();
    }
}
