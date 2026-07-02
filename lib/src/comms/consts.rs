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

#[cfg(test)]
mod tests {
    use super::truncate_utf8_safe;

    #[test]
    fn returns_original_when_within_limit() {
        assert_eq!(truncate_utf8_safe("hello", 10), "hello");
        assert_eq!(truncate_utf8_safe("hello", 5), "hello");
    }

    #[test]
    fn truncates_ascii_at_byte_limit() {
        assert_eq!(truncate_utf8_safe("hello world", 5), "hello");
    }

    #[test]
    fn never_splits_a_multibyte_char() {
        // "é" is 2 bytes, "🌐" is 4 bytes. Truncating mid-character must back off
        // to the previous char boundary rather than produce invalid UTF-8.
        let s = "aé🌐b"; // bytes: 1 + 2 + 4 + 1 = 8
                         // Limit 2 lands on the second byte of "é" -> must drop "é" entirely.
        assert_eq!(truncate_utf8_safe(s, 2), "a");
        // Limit 5 lands inside "🌐" -> back off to after "é".
        assert_eq!(truncate_utf8_safe(s, 5), "aé");
        // The result is always valid UTF-8 (no panic, str slicing succeeded).
        assert!(truncate_utf8_safe(s, 4).is_char_boundary(truncate_utf8_safe(s, 4).len()));
    }

    #[test]
    fn handles_zero_limit() {
        assert_eq!(truncate_utf8_safe("abc", 0), "");
    }
}
