//! "Is this place real, and may I enter it", answered before anything is offloaded. Existence is
//! decided against the content server, never against the Places catalog: Places only knows what
//! somebody registered, so a deployed but unlisted scene is a legitimate destination.

mod input;
mod resolve;

use crate::app::{Destination, Parcel};

pub use input::{parse_input, Intent};
pub use resolve::{resolve, DEADLINE, PREVIEW_DEADLINE};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum DestinationKind {
    /// A Genesis City catalyst (the default realm or another DAO one).
    Genesis,
    /// A `.dcl.eth` world.
    World,
    /// A preview server on this machine or its network.
    Preview,
    /// Any other realm.
    CustomRealm,
}

/// What the world demands of this user, now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum AccessVerdict {
    /// Open, or this user is allowed, or the secret they gave checked out.
    Allowed,
    /// A shared secret is required; `resolve_destination_with_secret` retries with one.
    NeedsPassword,
    /// A wallet allow-list that does not include this user.
    NotAllowed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum ResolveFailure {
    /// The input is not a coordinate, a world, a realm or a link naming one.
    InvalidInput,
    WorldNotFound,
    ParcelOutOfBounds,
    /// The realm has no scene at the parcel.
    ParcelEmpty,
    FetchFailed,
    PreviewUnreachable,
    Timeout,
    RateLimited,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct ResolvedDestination {
    pub kind: DestinationKind,
    pub access: AccessVerdict,
    /// Set when the destination was refused; `access` is then `Allowed` unless the refusal is
    /// about access.
    pub failure: Option<ResolveFailure>,
    /// Entering is allowed: no failure and `access` is `Allowed`.
    pub ready: bool,
    /// The `.dcl.eth` name for a world, empty otherwise.
    pub world_name: String,
    pub parcel: Option<Parcel>,
    /// What the loading screen shows, from the scene the existence check already read; empty is
    /// a normal answer and the Places catalog fills the gaps.
    pub scene_id: String,
    pub title: String,
    pub creator: String,
    pub image_url: String,
    pub asset_count: u32,
    /// What `enter_world` takes.
    pub destination: Destination,
}

#[cfg(test)]
mod tests;
