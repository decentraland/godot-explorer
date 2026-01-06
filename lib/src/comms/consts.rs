#[cfg(feature = "use_livekit")]
pub const GATEKEEPER_URL: &str = "https://comms-gatekeeper.decentraland.org/get-scene-adapter";

#[cfg(feature = "use_livekit")]
pub const PREVIEW_GATEKEEPER_URL: &str =
    "https://comms-gatekeeper-local.decentraland.org/get-scene-adapter";

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
