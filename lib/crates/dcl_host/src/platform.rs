//! The platform services the host gives the engine (D-41): device readings, Firebase and the
//! install attribution sources, and scene video. Kotlin/Swift implement each trait and install it
//! once at launch; the engine reads them from `dcl_core::platform`. Called from the engine's main
//! thread, so an implementation must not throw: a failed call returns its empty value.

use std::sync::Arc;

use dcl_core::platform::{self as core, DeviceHost, MarketingHost, VideoHost};

#[derive(Debug, Clone, uniffi::Record)]
pub struct DeviceInfo {
    pub brand: String,
    pub model: String,
    pub os_version: String,
    pub total_ram_mb: i32,
    pub soc_model: String,
    pub soc_manufacturer: String,
    pub board_platform: String,
    pub hardware: String,
    pub processor_type: String,
    pub processor_max_freq_mhz: i32,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct DeviceReadings {
    /// -1 where it is a slow read (Android).
    pub memory_usage_mb: i32,
    pub temperature_celsius: f32,
    pub thermal_state: String,
    pub battery_percent: f32,
    pub charging_state: String,
}

#[uniffi::export(with_foreign)]
pub trait DeviceService: Send + Sync {
    fn info(&self) -> DeviceInfo;
    /// Thermal, battery and (iOS) memory; cheap enough for the game loop.
    fn readings(&self) -> DeviceReadings;
    /// Blocks the UI thread for `ms` (ANR testing); false when unsupported.
    fn block_ui_thread(&self, ms: u32) -> bool;
}

/// Android: Firebase Analytics, the Play install referrer and the GA4F deferred deep link.
#[uniffi::export(with_foreign)]
pub trait MarketingService: Send + Sync {
    fn app_instance_id(&self) -> String;
    fn set_user_id(&self, user_id: String);
    fn set_user_property(&self, name: String, value: String);
    fn log_event(&self, name: String, params_json: String) -> bool;
    /// JSON object with a `status` (`pending`, `ok`, `error`, `not_available`) and its fields.
    fn install_referrer(&self) -> String;
    /// JSON object with a `status` (`pending`, `ok`, `error`) and its fields.
    fn deferred_deep_link(&self) -> String;
}

#[derive(Debug, Clone, Copy, uniffi::Record)]
pub struct VideoSize {
    pub width: i32,
    pub height: i32,
}

/// ExoPlayer on Android, AVPlayer on iOS. Positions are seconds.
#[uniffi::export(with_foreign)]
pub trait VideoBackend: Send + Sync {
    fn create(&self) -> i32;
    fn init_surface(&self, player: i32, width: i32, height: i32) -> bool;
    fn set_source(&self, player: i32, source: String, local: bool) -> bool;
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
    fn take_size_changed(&self, player: i32) -> bool;
    fn video_size(&self, player: i32) -> VideoSize;
    /// `AHardwareBuffer*` / `IOSurfaceRef` of a frame that arrived since the last call, else 0.
    fn poll_frame(&self, player: i32) -> u64;
    fn release(&self, player: i32);
}

#[uniffi::export]
pub fn install_device_service(service: Arc<dyn DeviceService>) {
    core::install_device(Arc::new(Device(service)));
}

#[uniffi::export]
pub fn install_marketing_service(service: Arc<dyn MarketingService>) {
    core::install_marketing(Arc::new(Marketing(service)));
}

#[uniffi::export]
pub fn install_video_backend(backend: Arc<dyn VideoBackend>) {
    core::install_video(Arc::new(Video(backend)));
}

struct Device(Arc<dyn DeviceService>);

impl DeviceHost for Device {
    fn info(&self) -> core::DeviceInfo {
        let i = self.0.info();
        core::DeviceInfo {
            brand: i.brand,
            model: i.model,
            os_version: i.os_version,
            total_ram_mb: i.total_ram_mb,
            soc_model: i.soc_model,
            soc_manufacturer: i.soc_manufacturer,
            board_platform: i.board_platform,
            hardware: i.hardware,
            processor_type: i.processor_type,
            processor_max_freq_mhz: i.processor_max_freq_mhz,
        }
    }

    fn readings(&self) -> core::DeviceReadings {
        let r = self.0.readings();
        core::DeviceReadings {
            memory_usage_mb: r.memory_usage_mb,
            temperature_celsius: r.temperature_celsius,
            thermal_state: r.thermal_state,
            battery_percent: r.battery_percent,
            charging_state: r.charging_state,
        }
    }

    fn block_ui_thread(&self, ms: u32) -> bool {
        self.0.block_ui_thread(ms)
    }
}

struct Marketing(Arc<dyn MarketingService>);

impl MarketingHost for Marketing {
    fn app_instance_id(&self) -> String {
        self.0.app_instance_id()
    }

    fn set_user_id(&self, user_id: &str) {
        self.0.set_user_id(user_id.to_string());
    }

    fn set_user_property(&self, name: &str, value: &str) {
        self.0
            .set_user_property(name.to_string(), value.to_string());
    }

    fn log_event(&self, name: &str, params_json: &str) -> bool {
        self.0.log_event(name.to_string(), params_json.to_string())
    }

    fn install_referrer(&self) -> String {
        self.0.install_referrer()
    }

    fn deferred_deep_link(&self) -> String {
        self.0.deferred_deep_link()
    }
}

struct Video(Arc<dyn VideoBackend>);

impl VideoHost for Video {
    fn create(&self) -> i32 {
        self.0.create()
    }

    fn init_surface(&self, player: i32, width: i32, height: i32) -> bool {
        self.0.init_surface(player, width, height)
    }

    fn set_source(&self, player: i32, source: &str, local: bool) -> bool {
        self.0.set_source(player, source.to_string(), local)
    }

    fn play(&self, player: i32) {
        self.0.play(player)
    }

    fn pause(&self, player: i32) {
        self.0.pause(player)
    }

    fn stop(&self, player: i32) {
        self.0.stop(player)
    }

    fn seek(&self, player: i32, seconds: f64) {
        self.0.seek(player, seconds)
    }

    fn position(&self, player: i32) -> f64 {
        self.0.position(player)
    }

    fn duration(&self, player: i32) -> f64 {
        self.0.duration(player)
    }

    fn is_playing(&self, player: i32) -> bool {
        self.0.is_playing(player)
    }

    fn set_volume(&self, player: i32, volume: f32) {
        self.0.set_volume(player, volume)
    }

    fn set_looping(&self, player: i32, looping: bool) {
        self.0.set_looping(player, looping)
    }

    fn set_rate(&self, player: i32, rate: f32) {
        self.0.set_rate(player, rate)
    }

    fn take_size_changed(&self, player: i32) -> bool {
        self.0.take_size_changed(player)
    }

    fn video_size(&self, player: i32) -> (i32, i32) {
        let size = self.0.video_size(player);
        (size.width, size.height)
    }

    fn poll_frame(&self, player: i32) -> u64 {
        self.0.poll_frame(player)
    }

    fn release(&self, player: i32) {
        self.0.release(player)
    }
}

/// `ExoPlayerWrapper.nativeGetHardwareBufferPtr`: the `AHardwareBuffer*` behind a Java `HardwareBuffer`,
/// which only the NDK can read. Valid while the Java object is alive (no extra reference taken).
#[cfg(target_os = "android")]
mod hardware_buffer {
    use std::ffi::c_void;

    #[link(name = "android")]
    extern "C" {
        fn AHardwareBuffer_fromHardwareBuffer(env: *mut c_void, buffer: *mut c_void)
            -> *mut c_void;
    }

    /// # Safety
    /// Called by the JVM with a valid `JNIEnv*` and a `HardwareBuffer` reference (or null).
    #[no_mangle]
    pub unsafe extern "system" fn Java_com_dclrn_video_ExoPlayerWrapper_nativeGetHardwareBufferPtr(
        env: *mut c_void,
        _this: *mut c_void,
        buffer: *mut c_void,
    ) -> i64 {
        if buffer.is_null() {
            return 0;
        }
        AHardwareBuffer_fromHardwareBuffer(env, buffer) as i64
    }
}
