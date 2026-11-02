//! A video decoded by the platform: ExoPlayer on Android and AVPlayer on iOS, through the
//! `dcl-godot-android` and `DclGodotiOS` plugins. Frames reach the GPU as an `ExternalTexture`
//! over an `AHardwareBuffer` or an `IOSurface`, with no readback. The texture is resized when
//! the real video size becomes known.

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
    /// Prefix of the plugin methods: `exoPlayerPlay`, `avPlayerPlay`.
    fn prefix(self) -> &'static str {
        match self {
            Flavor::Exo => "exoPlayer",
            Flavor::Av => "avPlayer",
        }
    }

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

    fn singleton(self) -> &'static str {
        match self {
            Flavor::Exo => "dcl-godot-android",
            Flavor::Av => "DclGodotiOS",
        }
    }

    fn create_method(self) -> &'static str {
        match self {
            Flavor::Exo => "createExoPlayer",
            Flavor::Av => "createAVPlayer",
        }
    }

    fn new_frame_method(self) -> &'static str {
        match self {
            Flavor::Exo => "exoPlayerHasNewHardwareBuffer",
            Flavor::Av => "avPlayerHasNewPixelBuffer",
        }
    }

    fn acquire_method(self) -> &'static str {
        match self {
            Flavor::Exo => "exoPlayerAcquireHardwareBufferPtr",
            Flavor::Av => "avPlayerAcquireIOSurfacePtr",
        }
    }

    /// ExoPlayer positions are milliseconds, AVPlayer's seconds.
    fn position_to_native(self, seconds: f32) -> NativePosition {
        match self {
            Flavor::Exo => NativePosition::Millis((seconds * 1000.0) as i32),
            Flavor::Av => NativePosition::Seconds(seconds),
        }
    }

    fn seconds_from_native(self, native: &Variant) -> f32 {
        match self {
            Flavor::Exo => millis_to_seconds(native.try_to::<i64>().unwrap_or(0)),
            Flavor::Av => native.try_to::<f64>().unwrap_or(0.0) as f32,
        }
    }
}

/// A position as the plugin takes it.
#[derive(Debug, PartialEq)]
enum NativePosition {
    Millis(i32),
    Seconds(f32),
}

impl NativePosition {
    fn to_variant(&self) -> Variant {
        match self {
            NativePosition::Millis(ms) => ms.to_variant(),
            NativePosition::Seconds(s) => s.to_variant(),
        }
    }
}

fn millis_to_seconds(millis: i64) -> f32 {
    millis as f32 / 1000.0
}

#[derive(GodotClass)]
#[class(no_init, base=Node)]
pub struct NativePlayer {
    flavor: Flavor,
    plugin: Option<Gd<Object>>,
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
            plugin: None,
            player_id: -1,
            external_texture: None,
            video_width: 0,
            video_height: 0,
            frame_count: 0,
            reinitializing_surface: false,
            base,
        })
    }

    fn ready_player(&self) -> bool {
        self.plugin.is_some() && self.player_id > 0
    }

    fn call(&mut self, suffix: &str, extra: &[Variant]) -> Variant {
        let method = format!("{}{suffix}", self.flavor.prefix());
        let mut args = vec![self.player_id.to_variant()];
        args.extend_from_slice(extra);
        match self.plugin.as_mut() {
            Some(plugin) => plugin.call(method.as_str(), &args),
            None => Variant::nil(),
        }
    }

    fn require_player(&self) -> bool {
        if !self.ready_player() {
            godot_error!("{}: Player not initialized", self.flavor.label());
        }
        self.ready_player()
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
        let Some(singleton) = godot::classes::Engine::singleton()
            .get_singleton(&StringName::from(flavor.singleton()))
        else {
            godot_error!(
                "{}: {} plugin not found!",
                flavor.label(),
                flavor.singleton()
            );
            return;
        };
        let mut plugin = singleton.cast::<Object>();
        self.player_id = plugin
            .call(flavor.create_method(), &[])
            .try_to()
            .unwrap_or(-1);
        self.plugin = Some(plugin);
        if self.player_id <= 0 {
            godot_error!("{}: Failed to create player", flavor.label());
        }
    }

    fn exit_tree(&mut self) {
        if self.ready_player() {
            self.call("Release", &[]);
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
        if !self.require_player() {
            return false;
        }
        self.video_width = width;
        self.video_height = height;
        let result: i32 = self
            .call("InitSurface", &[width.to_variant(), height.to_variant()])
            .try_to()
            .unwrap_or(0);
        if result <= 0 {
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
        if !self.require_player() {
            return false;
        }
        let ok = self.call("SetSourceUrl", &[url.to_variant()]).booleanize();
        if !ok {
            godot_error!("{}: Failed to set source URL: {url}", self.flavor.label());
        }
        ok
    }

    pub fn set_source_local(&mut self, file_path: &str) -> bool {
        if !self.require_player() {
            return false;
        }
        let ok = self
            .call("SetSourceLocal", &[file_path.to_variant()])
            .booleanize();
        if !ok {
            godot_error!(
                "{}: Failed to set local source: {file_path}",
                self.flavor.label()
            );
        }
        ok
    }

    pub fn play(&mut self) {
        if self.require_player() {
            self.call("Play", &[]);
        }
    }

    pub fn pause(&mut self) {
        if self.require_player() {
            self.call("Pause", &[]);
        }
    }

    pub fn stop(&mut self) {
        if self.require_player() {
            self.call("Stop", &[]);
        }
    }

    pub fn set_position(&mut self, seconds: f32) {
        if self.require_player() {
            let native = self.flavor.position_to_native(seconds).to_variant();
            self.call("SetPosition", &[native]);
        }
    }

    pub fn get_position(&mut self) -> f32 {
        if !self.ready_player() {
            return 0.0;
        }
        let native = self.call("GetPosition", &[]);
        self.flavor.seconds_from_native(&native)
    }

    pub fn get_duration(&mut self) -> f32 {
        if !self.ready_player() {
            return 0.0;
        }
        let native = self.call("GetDuration", &[]);
        self.flavor.seconds_from_native(&native)
    }

    pub fn is_playing(&mut self) -> bool {
        self.ready_player() && self.call("IsPlaying", &[]).booleanize()
    }

    pub fn set_volume(&mut self, volume: f32) {
        if self.require_player() {
            self.call("SetVolume", &[volume.clamp(0.0, 1.0).to_variant()]);
        }
    }

    pub fn set_looping(&mut self, looping: bool) {
        if self.require_player() {
            self.call("SetLooping", &[looping.to_variant()]);
        }
    }

    pub fn set_playback_rate(&mut self, rate: f32) {
        if self.require_player() {
            self.call("SetPlaybackRate", &[rate.clamp(0.1, 10.0).to_variant()]);
        }
    }

    pub fn get_texture(&self) -> Option<Gd<Texture2D>> {
        self.external_texture.clone().map(|t| t.upcast())
    }

    /// Polls the player: rebuilds the surface when the video size changed, then hands the next
    /// frame to the texture. True when a new frame arrived.
    pub fn update_texture(&mut self) -> bool {
        if !self.ready_player() {
            return false;
        }
        if !self.reinitializing_surface && self.call("HasVideoSizeChanged", &[]).booleanize() {
            // The surface swap is a heavy one-off: frames are not consumed while it waits its
            // frame-budget turn.
            self.reinitializing_surface = true;
            spawn_local(reinitialize_when_budget_allows(self.to_gd()));
        }
        if self.reinitializing_surface {
            return false;
        }
        let has_frame = {
            let method = self.flavor.new_frame_method();
            match self.plugin.as_mut() {
                Some(plugin) => plugin
                    .call(method, &[self.player_id.to_variant()])
                    .booleanize(),
                None => false,
            }
        };
        if !has_frame {
            return false;
        }
        let pointer: i64 = {
            let method = self.flavor.acquire_method();
            match self.plugin.as_mut() {
                Some(plugin) => plugin
                    .call(method, &[self.player_id.to_variant()])
                    .try_to()
                    .unwrap_or(0),
                None => 0,
            }
        };
        if pointer == 0 {
            return false;
        }
        self.frame_count += 1;
        // The buffer pointer goes straight to the renderer.
        if let Some(texture) = self.external_texture.as_mut() {
            texture.set_external_buffer_id(pointer as u64);
        }
        true
    }

    fn reinitialize_to_video_size(&mut self) {
        let width: i32 = self.call("GetVideoWidth", &[]).try_to().unwrap_or(0);
        let height: i32 = self.call("GetVideoHeight", &[]).try_to().unwrap_or(0);
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
        let result: i32 = self
            .call("InitSurface", &[width.to_variant(), height.to_variant()])
            .try_to()
            .unwrap_or(0);
        if result <= 0 {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn method_names_follow_the_plugin() {
        assert_eq!(Flavor::Exo.prefix(), "exoPlayer");
        assert_eq!(Flavor::Av.prefix(), "avPlayer");
        assert_eq!(Flavor::Exo.create_method(), "createExoPlayer");
        assert_eq!(Flavor::Av.new_frame_method(), "avPlayerHasNewPixelBuffer");
        assert_eq!(
            Flavor::Exo.acquire_method(),
            "exoPlayerAcquireHardwareBufferPtr"
        );
    }

    #[test]
    fn exo_positions_are_milliseconds_and_av_seconds() {
        assert_eq!(
            Flavor::Exo.position_to_native(1.5),
            NativePosition::Millis(1500)
        );
        assert_eq!(
            Flavor::Av.position_to_native(1.5),
            NativePosition::Seconds(1.5)
        );
        assert_eq!(millis_to_seconds(2500), 2.5);
    }
}
