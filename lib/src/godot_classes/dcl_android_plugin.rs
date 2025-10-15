use godot::prelude::*;

use crate::godot_classes::dcl_ios_plugin::DclMobileDeviceInfo;

/// Static wrapper for the DclAndroidPlugin (old plugin) that provides typed access to Android-specific functionality
pub struct DclAndroidPlugin;

impl DclAndroidPlugin {
    /// Try to get the DclAndroidPlugin singleton
    fn try_get_singleton() -> Option<Gd<Object>> {
        let singleton = godot::engine::Engine::singleton()
            .get_singleton(StringName::from("DclAndroidPlugin"))?;
        Some(singleton.cast::<Object>())
    }

    /// Show a Decentraland mobile toast notification
    pub fn show_decentraland_mobile_toast() -> bool {
        let Some(mut singleton) = Self::try_get_singleton() else {
            return false;
        };
        singleton.call(StringName::from("showDecentralandMobileToast"), &[]);
        true
    }

    /// Open a URL
    pub fn open_url(url: GString) -> bool {
        let Some(mut singleton) = Self::try_get_singleton() else {
            return false;
        };
        singleton.call(StringName::from("openUrl"), &[url.to_variant()]);
        true
    }

    /// Check if the old DclAndroidPlugin is available
    pub fn is_available() -> bool {
        Self::try_get_singleton().is_some()
    }
}

/// Static wrapper for the dcl-godot-android plugin (new plugin) that provides typed access to Android-specific functionality
pub struct DclGodotAndroidPlugin;

impl DclGodotAndroidPlugin {
    /// Try to get the dcl-godot-android singleton
    fn try_get_singleton() -> Option<Gd<Object>> {
        let singleton = godot::engine::Engine::singleton()
            .get_singleton(StringName::from("dcl-godot-android"))?;
        Some(singleton.cast::<Object>())
    }

    /// Open a URL in a custom tab (for social)
    pub fn open_custom_tab_url(url: GString) -> bool {
        let Some(mut singleton) = Self::try_get_singleton() else {
            return false;
        };
        singleton.call(StringName::from("openCustomTabUrl"), &[url.to_variant()]);
        true
    }

    /// Open a URL in a webview (for wallet connect)
    pub fn open_webview(url: GString, param: GString) -> bool {
        let Some(mut singleton) = Self::try_get_singleton() else {
            return false;
        };
        singleton.call(
            StringName::from("openWebView"),
            &[url.to_variant(), param.to_variant()],
        );
        true
    }

    /// Get mobile device information including memory, thermal state, battery, etc.
    pub fn get_mobile_device_info() -> Option<DclMobileDeviceInfo> {
        let mut singleton = Self::try_get_singleton()?;
        let info = singleton.call(StringName::from("getMobileDeviceInfo"), &[]);
        let dict = info.try_to::<Dictionary>().ok()?;
        Some(DclMobileDeviceInfo::from_dictionary(dict))
    }

    /// Check if the dcl-godot-android plugin is available
    pub fn is_available() -> bool {
        Self::try_get_singleton().is_some()
    }
}
