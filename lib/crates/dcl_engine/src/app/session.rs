//! Session-level actions of the app: opening URLs through the platform, the guest wallet's
//! device-bound anchor and its on-disk artifacts.

use godot::classes::{DirAccess, Engine, FileAccess, Os};
use godot::prelude::*;

use crate::fetcher::globals;
use crate::godot_classes::dcl_android_plugin::DclAndroidPlugin;
use crate::godot_classes::dcl_global::DclGlobal;
use crate::godot_classes::dcl_ios_plugin::DclIosPlugin;

/// On-disk guest identity artifacts, owned by Rust (device anchor, thirdweb guest session) plus the
/// mobile-BFF attestation session. Wiped when a non-upgraded guest deletes their account (issue
/// #2335) and by the non-production "reset guest wallet" debug button.
pub const GUEST_DEVICE_STORAGE_FILES: [&str; 3] = [
    "user://device_anchor.txt",
    "user://thirdweb_session.json",
    "user://attest_session.json",
];

/// Increase when the local assets cache format changes, which invalidates the cache.
/// v5: emote scenes baked before the case-insensitive prop lookup lack prop geometry.
/// v7: optimized assets are raw `{hash}.opt.scn`/`.opt.res` files from the /v6 bucket.
pub const LOCAL_ASSETS_CACHE_VERSION: i64 = 7;

/// The Android intent a wallet-connect URL needs a webview for, rather than a custom tab.
fn is_wallet_connect(url: &str) -> bool {
    url.contains("provider=wallet-connect")
}

/// Opens a URL in the platform's webview (a Safari view on iOS, a custom tab on Android).
pub fn open_webview_url(url: &str) {
    if DclIosPlugin::is_available() {
        DclIosPlugin::open_webview_url(url.into());
    } else if DclAndroidPlugin::is_available() {
        DclAndroidPlugin::open_custom_tab_url(url.into());
    } else {
        Os::singleton().shell_open(url);
    }
}

/// Opens a URL. With `use_webkit` it is an auth flow: Safari auth on iOS, a webview for
/// wallet-connect and a custom tab for social on Android. Otherwise Android shows its toast and
/// opens the system handler; the desktop opens the browser.
pub fn open_url(url: &str, use_webkit: bool) {
    if use_webkit {
        if DclIosPlugin::is_available() {
            DclIosPlugin::open_safari_auth_url(url.into());
        } else if DclAndroidPlugin::is_available() {
            if is_wallet_connect(url) {
                DclAndroidPlugin::open_webview(url.into(), "".into());
            } else {
                DclAndroidPlugin::open_custom_tab_url(url.into());
            }
        } else {
            Os::singleton().shell_open(url);
        }
    } else if DclAndroidPlugin::is_available() {
        DclAndroidPlugin::show_decentraland_mobile_toast();
        DclAndroidPlugin::open_url(url.into());
    } else {
        Os::singleton().shell_open(url);
    }
}

/// Deletes every on-disk guest artifact so the next "Play as guest" derives a brand-new wallet
/// instead of rehydrating the old one. Returns how many files were removed. It does not end the
/// session: pair it with `sign_out`.
pub fn clear_guest_device_storage() -> i32 {
    let mut removed = 0;
    for path in GUEST_DEVICE_STORAGE_FILES {
        if !FileAccess::file_exists(path) {
            continue;
        }
        match DirAccess::remove_absolute(path) {
            godot::global::Error::OK => removed += 1,
            error => godot_error!("[guest] delete: failed to remove {path} ({error:?})"),
        }
    }
    removed
}

/// The anchor the thirdweb guest session and wallet derive from, shared by every guest login flow.
/// In the debug rotate mode (non-production only) it is empty, so Rust falls back to the
/// resettable per-install UUID in `user://device_anchor.txt` on every platform. Otherwise it is the
/// device-bound native anchor (Android SSAID, iOS Keychain UUID) that survives a reinstall; the
/// desktop has none and Rust uses the `user://` UUID anyway.
///
/// `has_method` is always false for Android plugin singletons (JNISingleton methods are kept apart
/// from ClassDB), so the Android call is not guarded by it (godot#106436).
pub fn get_device_anchor_id() -> GString {
    let rotate = globals::engine()
        .bind()
        .setting("debug_rotate_guest_anchor".into())
        .try_to::<bool>()
        .unwrap_or(false);
    if rotate && !DclGlobal::is_production() {
        return GString::new();
    }
    let global = DclGlobal::singleton();
    let (is_android, is_ios) = {
        let g = global.bind();
        (g.is_android, g.is_ios)
    };
    let engine = Engine::singleton();
    let anchor = if is_android {
        engine
            .get_singleton("dcl-godot-android")
            .map(|mut plugin| plugin.call("getDeviceAnchorId", &[]))
    } else if is_ios {
        engine.get_singleton("DclGodotiOS").and_then(|mut plugin| {
            plugin
                .has_method("get_device_anchor_id")
                .then(|| plugin.call("get_device_anchor_id", &[]))
        })
    } else {
        None
    };
    anchor
        .and_then(|v| v.try_to::<GString>().ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_wallet_connect_needs_the_webview() {
        assert!(is_wallet_connect(
            "https://x/auth?provider=wallet-connect&a=1"
        ));
        assert!(!is_wallet_connect("https://x/auth?provider=google"));
    }

    #[test]
    fn the_guest_artifacts_include_the_attestation_session() {
        assert!(GUEST_DEVICE_STORAGE_FILES.contains(&"user://attest_session.json"));
        assert!(GUEST_DEVICE_STORAGE_FILES.contains(&"user://device_anchor.txt"));
    }
}
