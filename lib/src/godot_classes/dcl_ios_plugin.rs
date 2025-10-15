use godot::prelude::*;

/// Mobile device static information (doesn't change during runtime)
#[derive(GodotClass, Debug, Clone)]
#[class(base=RefCounted)]
pub struct DclMobileDeviceInfo {
    #[var]
    pub device_brand: GString,
    #[var]
    pub device_model: GString,
    #[var]
    pub os_version: GString,
    #[var]
    pub total_ram_mb: i32,
}

/// Mobile device dynamic metrics (changes during runtime)
#[derive(GodotClass, Debug, Clone)]
#[class(base=RefCounted)]
pub struct DclMobileMetrics {
    #[var]
    pub memory_usage: i32,
    #[var]
    pub device_temperature_celsius: f32,
    #[var]
    pub device_thermal_state: GString,
    #[var]
    pub battery_percent: f32,
    #[var]
    pub charging_state: GString,
}

#[godot_api]
impl IRefCounted for DclMobileDeviceInfo {
    fn init(_base: Base<RefCounted>) -> Self {
        Self {
            device_brand: GString::new(),
            device_model: GString::new(),
            os_version: GString::new(),
            total_ram_mb: -1,
        }
    }
}

#[godot_api]
impl IRefCounted for DclMobileMetrics {
    fn init(_base: Base<RefCounted>) -> Self {
        Self {
            memory_usage: -1,
            device_temperature_celsius: -1.0,
            device_thermal_state: GString::new(),
            battery_percent: -1.0,
            charging_state: GString::from("unknown"),
        }
    }
}

impl DclMobileDeviceInfo {
    pub fn from_dictionary(dict: Dictionary) -> Self {
        Self {
            device_brand: dict
                .get("device_brand")
                .and_then(|v| v.try_to::<GString>().ok())
                .unwrap_or_default(),
            device_model: dict
                .get("device_model")
                .and_then(|v| v.try_to::<GString>().ok())
                .unwrap_or_default(),
            os_version: dict
                .get("os_version")
                .and_then(|v| v.try_to::<GString>().ok())
                .unwrap_or_default(),
            total_ram_mb: dict
                .get("total_ram_mb")
                .and_then(|v| v.try_to::<i32>().ok())
                .unwrap_or(-1),
        }
    }
}

impl DclMobileMetrics {
    pub fn from_dictionary(dict: Dictionary) -> Self {
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
                .unwrap_or_default(),
            battery_percent: dict
                .get("battery_percent")
                .and_then(|v| v.try_to::<f32>().ok())
                .unwrap_or(-1.0),
            charging_state: dict
                .get("charging_state")
                .and_then(|v| v.try_to::<GString>().ok())
                .unwrap_or(GString::from("unknown")),
        }
    }
}

/// Static wrapper for the DclGodotiOS plugin that provides typed access to iOS-specific functionality
pub struct DclIosPlugin;

impl DclIosPlugin {
    /// Try to get the DclGodotiOS singleton
    fn try_get_singleton() -> Option<Gd<Object>> {
        let singleton =
            godot::engine::Engine::singleton().get_singleton(StringName::from("DclGodotiOS"))?;
        Some(singleton.cast::<Object>())
    }

    /// Get static mobile device information (doesn't change during runtime)
    pub fn get_mobile_device_info() -> Option<DclMobileDeviceInfo> {
        let mut singleton = Self::try_get_singleton()?;
        let info = singleton.call(StringName::from("get_mobile_device_info"), &[]);
        let dict = info.try_to::<Dictionary>().ok()?;
        Some(DclMobileDeviceInfo::from_dictionary(dict))
    }

    /// Get dynamic mobile metrics (changes during runtime)
    pub fn get_mobile_metrics() -> Option<DclMobileMetrics> {
        let mut singleton = Self::try_get_singleton()?;
        let metrics = singleton.call(StringName::from("get_mobile_metrics"), &[]);
        let dict = metrics.try_to::<Dictionary>().ok()?;
        Some(DclMobileMetrics::from_dictionary(dict))
    }

    /// Open a URL in a webview
    pub fn open_webview_url(url: GString) -> bool {
        let Some(mut singleton) = Self::try_get_singleton() else {
            return false;
        };
        singleton.call(StringName::from("open_webview_url"), &[url.to_variant()]);
        true
    }

    /// Open a URL for authentication
    pub fn open_auth_url(url: GString) -> bool {
        let Some(mut singleton) = Self::try_get_singleton() else {
            return false;
        };
        singleton.call(StringName::from("open_auth_url"), &[url.to_variant()]);
        true
    }

    /// Check if the iOS plugin is available
    pub fn is_available() -> bool {
        Self::try_get_singleton().is_some()
    }
}
