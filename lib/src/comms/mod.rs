pub mod adapter;
pub mod communication_manager;
mod consts;
pub use consts::truncate_utf8_safe;
pub mod profile;
#[cfg(feature = "use_pulse")]
pub mod pulse;
pub mod randomize_profile;
pub mod signed_login;
pub mod voice_chat;

/// Drops the query string, which on LiveKit connection strings carries the
/// room token, so an address can be logged or used as a room id.
pub(crate) fn redact_url_query(url: &str) -> &str {
    url.split('?').next().unwrap_or(url)
}
