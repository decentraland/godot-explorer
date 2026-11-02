/// Get the gatekeeper URL (transformed based on environment)
pub fn gatekeeper_url() -> String {
    crate::urls::comms_gatekeeper()
}

/// Get the local/preview gatekeeper URL (transformed based on environment)
pub fn gatekeeper_url_local() -> String {
    crate::urls::comms_gatekeeper_local()
}

// Compile-time defaults for connection scenarios. Archipelago can additionally be
// toggled at runtime via the remote `archipielago` feature flag — see
// `CommunicationManager::set_archipelago_enabled`.
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

// Pulse (ENet avatar-state relay) game port. The endpoint host comes from
// crate::urls::pulse_server(), overridable via --pulse-server / PULSE_SERVER.
#[cfg(feature = "use_pulse")]
pub const PULSE_SERVER_PORT: u16 = 7777;

/// The `IncomingMessage::room_id` for everything bridged from Pulse. A load-bearing room id
/// (like the `"scene-"`/`"livekit-"` prefixes): MessageProcessor keys the per-peer transport
/// preference and the inactivity-sweep exemption on it. Unconditional (not `use_pulse`-gated)
/// so MessageProcessor's gate logic compiles in every feature combination.
pub const PULSE_ROOM_ID: &str = "pulse";

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
