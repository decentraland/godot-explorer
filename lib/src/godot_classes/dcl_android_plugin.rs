use godot::prelude::*;

use crate::godot_classes::dcl_ios_plugin::{DclMobileDeviceInfo, DclMobileMetrics};
use godot::engine::Image;

/// Static wrapper for the DclAndroidPlugin (old plugin) that provides typed access to Android-specific functionality
#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub struct DclAndroidPlugin {
    _base: Base<RefCounted>,
}

#[godot_api]
impl DclAndroidPlugin {
    /// Try to get the DclAndroidPlugin singleton
    fn try_get_singleton() -> Option<Gd<Object>> {
        let singleton = godot::engine::Engine::singleton()
            .get_singleton(StringName::from("DclAndroidPlugin"))?;
        Some(singleton.cast::<Object>())
    }

    /// Show a Decentraland mobile toast notification
    #[func]
    pub fn show_decentraland_mobile_toast() -> bool {
        let Some(mut singleton) = Self::try_get_singleton() else {
            return false;
        };
        singleton.call(StringName::from("showDecentralandMobileToast"), &[]);
        true
    }

    /// Open a URL
    #[func]
    pub fn open_url(url: GString) -> bool {
        let Some(mut singleton) = Self::try_get_singleton() else {
            return false;
        };
        singleton.call(StringName::from("openUrl"), &[url.to_variant()]);
        true
    }

    /// Check if the old DclAndroidPlugin is available
    #[func]
    pub fn is_available() -> bool {
        Self::try_get_singleton().is_some()
    }
}

/// Static wrapper for the dcl-godot-android plugin (new plugin) that provides typed access to Android-specific functionality
#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub struct DclGodotAndroidPlugin {
    _base: Base<RefCounted>,
}

#[godot_api]
impl DclGodotAndroidPlugin {
    /// Try to get the dcl-godot-android singleton
    fn try_get_singleton() -> Option<Gd<Object>> {
        let singleton = godot::engine::Engine::singleton()
            .get_singleton(StringName::from("dcl-godot-android"))?;
        Some(singleton.cast::<Object>())
    }

    #[func]
    pub fn get_deeplink_args() -> Dictionary {
        let mut no_dict = Dictionary::new();
        let Some(mut singleton) = Self::try_get_singleton() else {
            no_dict.set("error", "No singleton returned");
            return no_dict;
        };

        no_dict.set("error", "No dict returned");
        let data = singleton.call(StringName::from("getLaunchIntentData"), &[]);
        data.try_to::<Dictionary>().ok().unwrap_or(no_dict)
    }

    /// Open a URL in a custom tab (for social)
    #[func]
    pub fn open_custom_tab_url(url: GString) -> bool {
        let Some(mut singleton) = Self::try_get_singleton() else {
            return false;
        };
        singleton.call(StringName::from("openCustomTabUrl"), &[url.to_variant()]);
        true
    }

    /// Open a URL in a webview (for wallet connect)
    #[func]
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

    /// Get static mobile device information (doesn't change during runtime) - internal use only
    pub(crate) fn get_mobile_device_info_internal() -> Option<DclMobileDeviceInfo> {
        let mut singleton = Self::try_get_singleton()?;
        let info = singleton.call(StringName::from("getMobileDeviceInfo"), &[]);
        let dict = info.try_to::<Dictionary>().ok()?;
        Some(DclMobileDeviceInfo::from_dictionary(dict))
    }

    /// Get dynamic mobile metrics (changes during runtime) - internal use only
    pub(crate) fn get_mobile_metrics_internal() -> Option<DclMobileMetrics> {
        let mut singleton = Self::try_get_singleton()?;
        let metrics = singleton.call(StringName::from("getMobileMetrics"), &[]);
        let dict = metrics.try_to::<Dictionary>().ok()?;
        Some(DclMobileMetrics::from_dictionary(dict))
    }

    /// Check if the dcl-godot-android plugin is available
    #[func]
    pub fn is_available() -> bool {
        Self::try_get_singleton().is_some()
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
            StringName::from("addCalendarEvent"),
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
        let result = singleton.call(StringName::from("shareText"), &[text.to_variant()]);
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
        if image.get_format() != godot::engine::image::Format::RGBA8 {
            rgba_image.convert(godot::engine::image::Format::RGBA8);
        }

        // Get the pixel data as a byte array
        let pixel_data = rgba_image.get_data();

        let result = singleton.call(
            StringName::from("shareTextWithImage"),
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
