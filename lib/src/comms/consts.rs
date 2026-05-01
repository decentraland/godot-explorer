/// Get the gatekeeper URL (transformed based on environment)
#[cfg(feature = "use_livekit")]
pub fn gatekeeper_url() -> String {
    crate::urls::comms_gatekeeper()
}

/// Get the local/preview gatekeeper URL (transformed based on environment)
#[cfg(feature = "use_livekit")]
pub fn gatekeeper_url_local() -> String {
    crate::urls::comms_gatekeeper_local()
}

// Temporary flags for testing different connection scenarios
#[cfg(feature = "use_livekit")]
pub const DISABLE_ARCHIPELAGO: bool = false;
#[cfg(feature = "use_livekit")]
pub const DISABLE_SCENE_ROOM: bool = false;

// Constants for bounded queue sizes to prevent memory exhaustion
pub const MAX_CHAT_MESSAGES: usize = 100;
pub const MAX_CHAT_MESSAGE_SIZE: usize = 200;
pub const MAX_SCENE_MESSAGES_PER_SCENE: usize = 500;
pub const MAX_SCENE_IDS: usize = 20;

// Message channel sizes
pub const MESSAGE_CHANNEL_SIZE: usize = 1000;
pub const OUTGOING_CHANNEL_SIZE: usize = 1000;
pub const PROFILE_UPDATE_CHANNEL_SIZE: usize = 100;

// Timing constants
pub const INACTIVE_PEER_THRESHOLD_SECS: u64 = 5;
pub const PROFILE_REQUEST_INTERVAL_SECS: f32 = 10.0;

// Protocol version
pub const DEFAULT_PROTOCOL_VERSION: u32 = 100;

/// Truncates a string to at most `max_bytes` while respecting UTF-8 character boundaries.
/// Returns the original string if it's already within the limit.
pub fn truncate_utf8_safe(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }

    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}
