//! Platform services the native host registers for the engine (D-41): device readings, the
//! marketing SDKs and scene video. Each is installed once, at launch; without a host (desktop,
//! the emulator, tests) none is, and callers read nothing.

use std::sync::{Arc, OnceLock};

/// What does not change while the app runs.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DeviceInfo {
    pub brand: String,
    pub model: String,
    pub os_version: String,
    /// -1 when the OS does not say.
    pub total_ram_mb: i32,
    /// Chipset ids (#2936, #3033); Android only, empty on iOS.
    pub soc_model: String,
    pub soc_manufacturer: String,
    pub board_platform: String,
    pub hardware: String,
    pub processor_type: String,
    /// -1 when the OS does not say.
    pub processor_max_freq_mhz: i32,
}

/// Cheap readings, safe to poll from the game loop.
#[derive(Clone, Debug, PartialEq)]
pub struct DeviceReadings {
    /// The process footprint in MB; -1 where it is a slow read (Android: the engine reads PSS).
    pub memory_usage_mb: i32,
    /// -1 when unknown.
    pub temperature_celsius: f32,
    /// `nominal`, `fair`, `serious`, `critical` or `unknown`.
    pub thermal_state: String,
    /// -1 when unknown.
    pub battery_percent: f32,
    /// `unplugged`, `plugged`, `usb`, `wireless`, `full` or `unknown`.
    pub charging_state: String,
}

pub trait DeviceHost: Send + Sync {
    fn info(&self) -> DeviceInfo;
    fn readings(&self) -> DeviceReadings;
    /// Blocks the OS UI thread for `ms` (a QA aid for ANR reports). False where the engine's own
    /// thread is the UI thread and blocking it is enough.
    fn block_ui_thread(&self, ms: u32) -> bool;
}

/// Firebase Analytics and the install attribution sources (Android).
pub trait MarketingHost: Send + Sync {
    /// The Firebase app instance id; empty until Firebase resolved it.
    fn app_instance_id(&self) -> String;
    fn set_user_id(&self, user_id: &str);
    fn set_user_property(&self, name: &str, value: &str);
    /// `params_json` is a JSON object of primitives; true when the event was handed to Firebase.
    fn log_event(&self, name: &str, params_json: &str) -> bool;
    /// The Play install referrer as a JSON object with a `status`; the first call starts the fetch.
    fn install_referrer(&self) -> String;
    /// The GA4F deferred deep link as a JSON object with a `status`; the first call starts watching.
    fn deferred_deep_link(&self) -> String;
}

/// A platform video player per id (ExoPlayer, AVPlayer). Frames stay on the GPU: `poll_frame`
/// returns an `AHardwareBuffer*` or `IOSurfaceRef` for the engine's `ExternalTexture`.
pub trait VideoHost: Send + Sync {
    /// A new player's id; 0 or less when it could not be created.
    fn create(&self) -> i32;
    /// (Re)creates the output surface at `width x height`.
    fn init_surface(&self, player: i32, width: i32, height: i32) -> bool;
    fn set_source(&self, player: i32, source: &str, local: bool) -> bool;
    fn play(&self, player: i32);
    fn pause(&self, player: i32);
    fn stop(&self, player: i32);
    fn seek(&self, player: i32, seconds: f64);
    fn position(&self, player: i32) -> f64;
    fn duration(&self, player: i32) -> f64;
    fn is_playing(&self, player: i32) -> bool;
    fn set_volume(&self, player: i32, volume: f32);
    fn set_looping(&self, player: i32, looping: bool);
    fn set_rate(&self, player: i32, rate: f32);
    /// True once after the decoded video size changed.
    fn take_size_changed(&self, player: i32) -> bool;
    fn video_size(&self, player: i32) -> (i32, i32);
    /// The newest frame's buffer when one arrived since the last call, else 0.
    fn poll_frame(&self, player: i32) -> u64;
    fn release(&self, player: i32);
}

static DEVICE: OnceLock<Arc<dyn DeviceHost>> = OnceLock::new();
static MARKETING: OnceLock<Arc<dyn MarketingHost>> = OnceLock::new();
static VIDEO: OnceLock<Arc<dyn VideoHost>> = OnceLock::new();

/// Registers the host's device readings. Only the first call counts.
pub fn install_device(host: Arc<dyn DeviceHost>) {
    let _ = DEVICE.set(host);
}

pub fn install_marketing(host: Arc<dyn MarketingHost>) {
    let _ = MARKETING.set(host);
}

pub fn install_video(host: Arc<dyn VideoHost>) {
    let _ = VIDEO.set(host);
}

pub fn device() -> Option<&'static Arc<dyn DeviceHost>> {
    DEVICE.get()
}

pub fn marketing() -> Option<&'static Arc<dyn MarketingHost>> {
    MARKETING.get()
}

pub fn video() -> Option<&'static Arc<dyn VideoHost>> {
    VIDEO.get()
}
