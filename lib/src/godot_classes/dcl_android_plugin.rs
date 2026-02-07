use godot::prelude::*;

use crate::godot_classes::dcl_ios_plugin::{DclMobileDeviceInfo, DclMobileMetrics};
use godot::classes::Image;

#[cfg(debug_assertions)]
use std::cell::Cell;
#[cfg(debug_assertions)]
use std::time::Instant;

#[cfg(debug_assertions)]
thread_local! {
    static JNI_TIME_US: Cell<u64> = const { Cell::new(0) };
    static JNI_CALL_COUNT: Cell<i32> = const { Cell::new(0) };
}

const SINGLETON_NAME: &str = "dcl-godot-android";

/// Static wrapper for the dcl-godot-android plugin that provides typed access to Android-specific functionality
#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub struct DclAndroidPlugin {
    _base: Base<RefCounted>,
}

#[godot_api]
impl DclAndroidPlugin {
    /// Try to get the dcl-godot-android singleton
    fn try_get_singleton() -> Option<Gd<Object>> {
        let singleton =
            godot::classes::Engine::singleton().get_singleton(&StringName::from(SINGLETON_NAME))?;
        Some(singleton.cast::<Object>())
    }

    /// Wrapper around singleton.call() that accumulates JNI timing in debug builds
    #[cfg(debug_assertions)]
    fn timed_jni_call(singleton: &mut Gd<Object>, method: &str, args: &[Variant]) -> Variant {
        let start = Instant::now();
        let result = singleton.call(method, args);
        let elapsed_us = start.elapsed().as_micros() as u64;
        JNI_TIME_US.with(|c| c.set(c.get() + elapsed_us));
        JNI_CALL_COUNT.with(|c| c.set(c.get() + 1));
        result
    }

    #[cfg(not(debug_assertions))]
    #[inline(always)]
    fn timed_jni_call(singleton: &mut Gd<Object>, method: &str, args: &[Variant]) -> Variant {
        singleton.call(method, args)
    }

    /// Returns accumulated JNI call time in milliseconds since last call, then resets.
    /// In release builds, always returns 0.0 with no overhead.
    #[func]
    pub fn take_jni_time_ms() -> f64 {
        #[cfg(debug_assertions)]
        {
            let us = JNI_TIME_US.with(|c| c.replace(0));
            us as f64 / 1000.0
        }
        #[cfg(not(debug_assertions))]
        {
            0.0
        }
    }

    /// Returns accumulated JNI call count since last call, then resets.
    /// In release builds, always returns 0 with no overhead.
    #[func]
    pub fn take_jni_call_count() -> i32 {
        #[cfg(debug_assertions)]
        {
            JNI_CALL_COUNT.with(|c| c.replace(0))
        }
        #[cfg(not(debug_assertions))]
        {
            0
        }
    }

    /// Check if the dcl-godot-android plugin is available (runtime check)
    #[func]
    pub fn is_available() -> bool {
        godot::classes::Engine::singleton().has_singleton(&StringName::from(SINGLETON_NAME))
    }

    /// Show a Decentraland mobile toast notification
    #[func]
    pub fn show_decentraland_mobile_toast() -> bool {
        let Some(mut singleton) = Self::try_get_singleton() else {
            return false;
        };
        Self::timed_jni_call(&mut singleton, "showDecentralandMobileToast", &[]);
        true
    }

    /// Open a URL in the default browser
    #[func]
    pub fn open_url(url: GString) -> bool {
        let Some(mut singleton) = Self::try_get_singleton() else {
            return false;
        };
        Self::timed_jni_call(&mut singleton, "openUrl", &[url.to_variant()]);
        true
    }

    #[func]
    pub fn get_deeplink_args() -> VarDictionary {
        let mut no_dict = VarDictionary::new();
        let Some(mut singleton) = Self::try_get_singleton() else {
            no_dict.set("error", "No singleton returned");
            return no_dict;
        };

        no_dict.set("error", "No dict returned");
        let data = Self::timed_jni_call(&mut singleton, "getLaunchIntentData", &[]);
        data.try_to::<VarDictionary>().ok().unwrap_or(no_dict)
    }

    /// Open a URL in a custom tab (for social)
    #[func]
    pub fn open_custom_tab_url(url: GString) -> bool {
        let Some(mut singleton) = Self::try_get_singleton() else {
            return false;
        };
        Self::timed_jni_call(&mut singleton, "openCustomTabUrl", &[url.to_variant()]);
        true
    }

    /// Open a URL in a webview (for wallet connect)
    #[func]
    pub fn open_webview(url: GString, param: GString) -> bool {
        let Some(mut singleton) = Self::try_get_singleton() else {
            return false;
        };
        Self::timed_jni_call(
            &mut singleton,
            "openWebView",
            &[url.to_variant(), param.to_variant()],
        );
        true
    }

    /// Get static mobile device information (doesn't change during runtime) - internal use only
    pub(crate) fn get_mobile_device_info_internal() -> Option<DclMobileDeviceInfo> {
        let mut singleton = Self::try_get_singleton()?;
        let info = Self::timed_jni_call(&mut singleton, "getMobileDeviceInfo", &[]);
        let dict = info.try_to::<VarDictionary>().ok()?;
        Some(DclMobileDeviceInfo::from_dictionary(dict))
    }

    /// Get dynamic mobile metrics (changes during runtime) - internal use only
    pub(crate) fn get_mobile_metrics_internal() -> Option<DclMobileMetrics> {
        let mut singleton = Self::try_get_singleton()?;
        let metrics = Self::timed_jni_call(&mut singleton, "getMobileMetrics", &[]);
        let dict = metrics.try_to::<VarDictionary>().ok()?;
        Some(DclMobileMetrics::from_dictionary(dict))
    }

    /// Get thermal and charging state in a single JNI call
    /// Returns (thermal_state, charging_state) with defaults if unavailable
    pub(crate) fn get_thermal_and_charging_state() -> (String, String) {
        match Self::get_mobile_metrics_internal() {
            Some(m) => (m.device_thermal_state, m.charging_state),
            None => (String::new(), "unknown".to_string()),
        }
    }

    /// Get total device RAM in megabytes
    /// Returns -1 if unavailable
    #[func]
    pub fn get_total_ram_mb() -> i32 {
        Self::get_mobile_device_info_internal()
            .map(|info| info.total_ram_mb)
            .unwrap_or(-1)
    }

    /// Add a calendar event with title, description, start time, end time, and location
    /// Times are in milliseconds since Unix epoch (Jan 1, 1970)
    /// Returns true if the calendar UI was shown successfully, false otherwise
    #[func]
    pub fn add_calendar_event(
        title: GString,
        description: GString,
        start_time_millis: i64,
        end_time_millis: i64,
        location: GString,
    ) -> bool {
        let Some(mut singleton) = Self::try_get_singleton() else {
            return false;
        };
        let result = Self::timed_jni_call(
            &mut singleton,
            "addCalendarEvent",
            &[
                title.to_variant(),
                description.to_variant(),
                start_time_millis.to_variant(),
                end_time_millis.to_variant(),
                location.to_variant(),
            ],
        );
        result.try_to::<bool>().unwrap_or(false)
    }

    /// Share text using the system share sheet
    /// Returns true if the share dialog was shown successfully, false otherwise
    #[func]
    pub fn share_text(text: GString) -> bool {
        let Some(mut singleton) = Self::try_get_singleton() else {
            return false;
        };
        let result = Self::timed_jni_call(&mut singleton, "shareText", &[text.to_variant()]);
        result.try_to::<bool>().unwrap_or(false)
    }

    /// Share text with an image using the system share sheet
    /// image should be a Godot Image object
    /// Returns true if the share dialog was shown successfully, false otherwise
    #[func]
    pub fn share_text_with_image(text: GString, image: Gd<Image>) -> bool {
        let Some(mut singleton) = Self::try_get_singleton() else {
            return false;
        };

        // Extract image data for Android
        let width = image.get_width();
        let height = image.get_height();

        // Convert image to RGBA8 format if needed
        let mut rgba_image = image.clone();
        if image.get_format() != godot::classes::image::Format::RGBA8 {
            rgba_image.convert(godot::classes::image::Format::RGBA8);
        }

        // Get the pixel data as a byte array
        let pixel_data = rgba_image.get_data();

        let result = Self::timed_jni_call(
            &mut singleton,
            "shareTextWithImage",
            &[
                text.to_variant(),
                width.to_variant(),
                height.to_variant(),
                pixel_data.to_variant(),
            ],
        );
        result.try_to::<bool>().unwrap_or(false)
    }
}
