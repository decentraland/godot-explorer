//! Session-level actions of the app: opening URLs through the platform and the guest wallet's
//! on-disk artifacts.

use godot::classes::{DirAccess, FileAccess, Os};
use godot::prelude::*;

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

/// Opens a URL with the system handler.
pub fn open_url(url: &str) {
    Os::singleton().shell_open(url);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_guest_artifacts_include_the_attestation_session() {
        assert!(GUEST_DEVICE_STORAGE_FILES.contains(&"user://attest_session.json"));
        assert!(GUEST_DEVICE_STORAGE_FILES.contains(&"user://device_anchor.txt"));
    }
}
