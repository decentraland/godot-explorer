//! Things a scene asks the user's app to do: open a link, jump somewhere, change realm, show an
//! NFT. The engine turns the scene's RPC into a request, the app answers it, and the pending map
//! here keeps each RPC's responder until then.

use std::collections::HashMap;

use crate::app::Parcel;

pub type RequestId = u32;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum SceneRequest {
    OpenExternalUrl {
        url: String,
    },
    /// A parcel in the current realm (`realm` unset) or in the named one, or only a realm (spawn).
    Teleport {
        parcel: Option<Parcel>,
        realm: Option<String>,
    },
    ChangeRealm {
        realm: String,
        message: Option<String>,
    },
    OpenNft {
        urn: String,
    },
}

/// Responders of the requests nobody has answered yet.
pub struct PendingRequests<R> {
    next_id: RequestId,
    pending: HashMap<RequestId, R>,
}

impl<R> Default for PendingRequests<R> {
    fn default() -> Self {
        Self {
            next_id: 1,
            pending: HashMap::new(),
        }
    }
}

impl<R> PendingRequests<R> {
    pub fn submit(&mut self, responder: R) -> RequestId {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        self.pending.insert(id, responder);
        id
    }

    /// The responder for `id`, once. `None` if it was already answered or never existed.
    pub fn take(&mut self, id: RequestId) -> Option<R> {
        self.pending.remove(&id)
    }

    pub fn len(&self) -> usize {
        self.pending.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_request_gets_a_distinct_id_and_is_answered_once() {
        let mut pending = PendingRequests::default();
        let a = pending.submit("first");
        let b = pending.submit("second");
        assert_ne!(a, b);
        assert_eq!(pending.len(), 2);

        assert_eq!(pending.take(a), Some("first"));
        assert_eq!(pending.take(a), None, "a request is answered once");
        assert_eq!(pending.take(b), Some("second"));
        assert!(pending.is_empty());
        assert_eq!(pending.take(99), None);
    }

    #[test]
    fn ids_never_collide_with_zero_after_wrapping() {
        let mut pending = PendingRequests::<()>::default();
        pending.next_id = u32::MAX;
        let last = pending.submit(());
        let wrapped = pending.submit(());
        assert_eq!(last, u32::MAX);
        assert_eq!(wrapped, 1);
    }
}
