use godot::prelude::*;

use crate::godot_classes::dcl_ios_plugin::{DclMobileDeviceInfo, DclMobileMetrics};
use godot::classes::Image;

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
        singleton.call("showDecentralandMobileToast", &[]);
        true
    }

    /// Open a URL in the default browser
    #[func]
    pub fn open_url(url: GString) -> bool {
        let Some(mut singleton) = Self::try_get_singleton() else {
            return false;
        };
        singleton.call("openUrl", &[url.to_variant()]);
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
        let data = singleton.call("getLaunchIntentData", &[]);
        data.try_to::<VarDictionary>().ok().unwrap_or(no_dict)
    }

    /// Open a URL in a custom tab (for social)
    #[func]
    pub fn open_custom_tab_url(url: GString) -> bool {
        let Some(mut singleton) = Self::try_get_singleton() else {
            return false;
        };
        singleton.call("openCustomTabUrl", &[url.to_variant()]);
        true
    }

    /// Open a URL in a webview (for wallet connect)
    #[func]
    pub fn open_webview(url: GString, param: GString) -> bool {
        let Some(mut singleton) = Self::try_get_singleton() else {
            return false;
        };
        singleton.call("openWebView", &[url.to_variant(), param.to_variant()]);
        true
    }

    /// Get static mobile device information (doesn't change during runtime) - internal use only
    pub(crate) fn get_mobile_device_info_internal() -> Option<DclMobileDeviceInfo> {
        let mut singleton = Self::try_get_singleton()?;
        let info = singleton.call("getMobileDeviceInfo", &[]);
        let dict = info.try_to::<VarDictionary>().ok()?;
        Some(DclMobileDeviceInfo::from_dictionary(dict))
    }

    /// Get dynamic mobile metrics (changes during runtime) - internal use only
    pub(crate) fn get_mobile_metrics_internal() -> Option<DclMobileMetrics> {
        let mut singleton = Self::try_get_singleton()?;
        let metrics = singleton.call("getMobileMetrics", &[]);
        let dict = metrics.try_to::<VarDictionary>().ok()?;
        Some(DclMobileMetrics::from_dictionary(dict))
    }

    /// Get current thermal state for dynamic graphics adjustment
    /// Returns: "nominal", "fair", "serious", "critical", or empty string if unavailable
    #[func]
    pub fn get_thermal_state() -> GString {
        Self::get_mobile_metrics_internal()
            .map(|m| GString::from(m.device_thermal_state))
            .unwrap_or_default()
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
        let result = singleton.call(
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
        let result = singleton.call("shareText", &[text.to_variant()]);
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

        let result = singleton.call(
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
