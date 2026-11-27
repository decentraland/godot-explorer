pub mod adapter;
pub mod communication_manager;
mod consts;
pub use consts::truncate_utf8_safe;
pub mod profile;
#[cfg(feature = "use_pulse")]
pub mod pulse;
pub mod randomize_profile;
pub mod signed_login;
pub mod sink;
pub mod voice_chat;

/// Drops the query string, which on LiveKit connection strings carries the
/// room token, so an address can be logged or used as a room id.
pub(crate) fn redact_url_query(url: &str) -> &str {
    url.split('?').next().unwrap_or(url)
}

/// Token-free and still unique per connection: `Peer.room_activity` is keyed by the room id,
/// and a reconnect to the same host must not alias the previous room.
#[cfg(feature = "use_livekit")]
pub(crate) fn livekit_room_id(address: &str) -> String {
    use std::sync::atomic::{AtomicU32, Ordering};
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    format!("livekit-{}#{}", redact_url_query(address), seq)
}
