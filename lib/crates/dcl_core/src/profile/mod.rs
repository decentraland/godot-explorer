//! The player's own profile and public profile lookups: fetch, owned names, deploy and the
//! account deletion request (port of `rn_req_profile.gd`, `ProfileService` and `NamesRequest`).

mod cid;
mod identity;
mod model;
mod multipart;
mod service;
#[cfg(test)]
mod tests;
mod transport;

pub use identity::{Identity, SessionIdentity};
pub use model::{AvatarPayload, Profile, ProfileDocument, ProfileLink, ProfileUpdate};
pub use service::{Endpoints, ProfileError, ProfileService};
pub use transport::{HttpReply, HttpRequest, ReqwestTransport, Transport};

/// The process's profile service, shared by the shell's FFI and the account lifecycle.
pub fn global() -> std::sync::Arc<ProfileService> {
    static SERVICE: std::sync::OnceLock<std::sync::Arc<ProfileService>> =
        std::sync::OnceLock::new();
    SERVICE
        .get_or_init(|| {
            std::sync::Arc::new(ProfileService::new(
                std::sync::Arc::new(ReqwestTransport::new()),
                std::sync::Arc::new(SessionIdentity),
                Endpoints::live(),
            ))
        })
        .clone()
}
