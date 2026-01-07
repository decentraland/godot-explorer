use godot::prelude::*;

use godot::classes::Image;

/// Mobile device static information (doesn't change during runtime) - internal Rust struct
#[derive(Debug, Clone)]
pub struct DclMobileDeviceInfo {
    pub device_brand: String,
    pub device_model: String,
    pub os_version: String,
    pub total_ram_mb: i32,
}

/// Mobile device dynamic metrics (changes during runtime) - internal Rust struct
#[derive(Debug, Clone)]
pub struct DclMobileMetrics {
    pub memory_usage: i32,
    pub device_temperature_celsius: f32,
    pub device_thermal_state: String,
    pub battery_percent: f32,
    pub charging_state: String,
}

impl DclMobileDeviceInfo {
    pub fn from_dictionary(dict: VarDictionary) -> Self {
        Self {
            device_brand: dict
                .get("device_brand")
                .and_then(|v| v.try_to::<GString>().ok())
                .map(|s| s.to_string())
                .unwrap_or_default(),
            device_model: dict
                .get("device_model")
                .and_then(|v| v.try_to::<GString>().ok())
                .map(|s| s.to_string())
                .unwrap_or_default(),
            os_version: dict
                .get("os_version")
                .and_then(|v| v.try_to::<GString>().ok())
                .map(|s| s.to_string())
                .unwrap_or_default(),
            total_ram_mb: dict
                .get("total_ram_mb")
                .and_then(|v| v.try_to::<i32>().ok())
                .unwrap_or(-1),
        }
    }
}

impl DclMobileMetrics {
    pub fn from_dictionary(dict: VarDictionary) -> Self {
        Self {
            memory_usage: dict
                .get("memory_usage")
                .and_then(|v| v.try_to::<i32>().ok())
                .unwrap_or(-1),
            device_temperature_celsius: dict
                .get("device_temperature_celsius")
                .and_then(|v| v.try_to::<f32>().ok())
                .unwrap_or(-1.0),
            device_thermal_state: dict
                .get("thermal_state")
                .and_then(|v| v.try_to::<GString>().ok())
                .map(|s| s.to_string())
                .unwrap_or_default(),
            battery_percent: dict
                .get("battery_percent")
                .and_then(|v| v.try_to::<f32>().ok())
                .unwrap_or(-1.0),
            charging_state: dict
                .get("charging_state")
                .and_then(|v| v.try_to::<GString>().ok())
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
        }
    }
}

/// Static wrapper for the DclGodotiOS plugin that provides typed access to iOS-specific functionality
#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub struct DclIosPlugin {
    _base: Base<RefCounted>,
}

#[godot_api]
impl DclIosPlugin {
    /// Try to get the DclGodotiOS singleton
    fn try_get_singleton() -> Option<Gd<Object>> {
        let singleton =
            godot::classes::Engine::singleton().get_singleton(&StringName::from("DclGodotiOS"))?;
        Some(singleton.cast::<Object>())
    }

    /// Get static mobile device information (doesn't change during runtime) - internal use only
    pub(crate) fn get_mobile_device_info_internal() -> Option<DclMobileDeviceInfo> {
        let mut singleton = Self::try_get_singleton()?;
        let info = singleton.call("get_mobile_device_info", &[]);
        let dict = info.try_to::<VarDictionary>().ok()?;
        Some(DclMobileDeviceInfo::from_dictionary(dict))
    }

    /// Get dynamic mobile metrics (changes during runtime) - internal use only
    pub(crate) fn get_mobile_metrics_internal() -> Option<DclMobileMetrics> {
        let mut singleton = Self::try_get_singleton()?;
        let metrics = singleton.call("get_mobile_metrics", &[]);
        let dict = metrics.try_to::<VarDictionary>().ok()?;
        Some(DclMobileMetrics::from_dictionary(dict))
    }

    /// Open a URL in a webview
    #[func]
    pub fn open_webview_url(url: GString) -> bool {
        let Some(mut singleton) = Self::try_get_singleton() else {
            return false;
        };
        singleton.call("open_webview_url", &[url.to_variant()]);
        true
    }

    /// Open a URL for authentication
    #[func]
    pub fn open_auth_url(url: GString) -> bool {
        let Some(mut singleton) = Self::try_get_singleton() else {
            return false;
        };
        singleton.call("open_auth_url", &[url.to_variant()]);
        true
    }

    /// Get deeplink arguments (URL received from deeplink)
    /// Returns a Dictionary with "data" key containing the deeplink URL, or empty if no deeplink
    #[func]
    pub fn get_deeplink_args() -> VarDictionary {
        let mut dict = VarDictionary::new();
        let Some(mut singleton) = Self::try_get_singleton() else {
            dict.set("error", "No singleton returned");
            return dict;
        };

        let url_variant = singleton.call("get_deeplink_url", &[]);
        let url = url_variant
            .try_to::<GString>()
            .ok()
            .unwrap_or_else(|| GString::from(""));

        // Return dictionary with "data" key to match Android API
        dict.set("data", url);
        dict
    }

    /// Check if the iOS plugin is available
    #[func]
    pub fn is_available() -> bool {
        #[cfg(target_os = "ios")]
        {
            true
        }
        #[cfg(not(target_os = "ios"))]
        {
            false
        }
    }

    /// Get current thermal state for dynamic graphics adjustment
    /// Returns: "nominal", "fair", "serious", "critical", or empty string if unavailable
    #[func]
    pub fn get_thermal_state() -> GString {
        Self::get_mobile_metrics_internal()
            .map(|m| GString::from(m.device_thermal_state))
            .unwrap_or_default()
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
            "add_calendar_event",
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
        let result = singleton.call("share_text", &[text.to_variant()]);
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
        let result = singleton.call(
            "share_text_with_image",
            &[text.to_variant(), image.to_variant()],
        );
        result.try_to::<bool>().unwrap_or(false)
    }
}
