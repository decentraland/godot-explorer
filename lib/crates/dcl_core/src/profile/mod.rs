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
