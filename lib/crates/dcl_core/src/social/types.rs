//! Typed records of the friends domain, the shapes the shell renders.

use std::fmt;

/// A person in the friends list or in a request list. `friendship_id` is only known for requests.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct Friend {
    pub address: String,
    pub name: String,
    pub has_claimed_name: bool,
    pub profile_picture_url: String,
    pub friendship_id: String,
}

/// Pending friend requests, in both directions.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct FriendRequests {
    pub received: Vec<Friend>,
    pub sent: Vec<Friend>,
}

/// Someone standing near the player.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct NearbyUser {
    pub address: String,
    pub name: String,
    pub has_claimed_name: bool,
    pub profile_picture_url: String,
}

/// The public fields of a catalyst profile the shell renders.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PublicProfile {
    pub name: String,
    pub has_claimed_name: bool,
    pub face_url: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SocialError {
    NotSignedIn,
    /// The service could not be reached, or refused the call.
    Request {
        detail: String,
    },
}

impl SocialError {
    pub(crate) fn request(detail: impl Into<String>) -> Self {
        Self::Request {
            detail: detail.into(),
        }
    }
}

impl fmt::Display for SocialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotSignedIn => write!(f, "not signed in"),
            Self::Request { detail } => write!(f, "{detail}"),
        }
    }
}

impl std::error::Error for SocialError {}
