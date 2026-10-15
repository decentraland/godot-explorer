//! Client-side gate for private worlds (#1725). Fails open everywhere: a network error, a
//! malformed answer or a missing address all mean "allowed", because comms stays the real
//! enforcement and a false positive here would lock users out.

use std::{
    cell::RefCell,
    collections::HashMap,
    time::{Duration, Instant},
};

use dcl_core::{
    gates::destination::{access_of, AccessVerdict},
    realm,
};

use super::http;
use crate::engine::local_task::next_frame;

/// Long enough that the prefetch at the navigation entry point and the gate in front of the
/// realm change share one fetch, short enough that an allow-list edit is picked up next time.
const CACHE_TTL: Duration = Duration::from_secs(30);

struct Answer {
    address: String,
    allowed: bool,
    expires: Instant,
}

enum Slot {
    Answered(Answer),
    InFlight,
}

thread_local! {
    static SLOTS: RefCell<HashMap<String, Slot>> = RefCell::new(HashMap::new());
}

/// The bare `<name>.dcl.eth` when `realm_string` is a world, empty otherwise (genesis city, a
/// local preview, a custom catalyst).
pub fn world_name_from_realm(realm_string: &str) -> String {
    dcl_core::gates::destination::world_name_from_realm(realm_string)
}

/// Whether `address` may enter `world_name`. Only definitive answers are cached; concurrent
/// callers share one request.
pub async fn is_allowed(world_name: &str, address: &str) -> bool {
    let address = address.to_lowercase();
    let now = Instant::now();
    let must_fetch = SLOTS.with(|slots| match slots.borrow().get(world_name) {
        Some(Slot::Answered(answer)) if answer.address == address && answer.expires > now => {
            Some(Some(answer.allowed))
        }
        Some(Slot::InFlight) => Some(None),
        _ => None,
    });
    match must_fetch {
        Some(Some(allowed)) => return allowed,
        Some(None) => {
            // The card-open warm followed by a fast JUMP IN: wait for the first request.
            loop {
                next_frame().await;
                let settled = SLOTS.with(|slots| match slots.borrow().get(world_name) {
                    Some(Slot::InFlight) => None,
                    Some(Slot::Answered(answer)) => Some(answer.allowed),
                    None => Some(true),
                });
                if let Some(allowed) = settled {
                    return allowed;
                }
            }
        }
        None => {}
    }

    SLOTS.with(|slots| {
        slots
            .borrow_mut()
            .insert(world_name.to_string(), Slot::InFlight)
    });
    let url = format!("{}/permissions", realm::dcl_world_url(world_name));
    let answer = http::get_json(&url).await;
    SLOTS.with(|slots| {
        let mut slots = slots.borrow_mut();
        match answer {
            Some(permissions) if permissions.is_object() => {
                let allowed = access_of(&permissions, &address) != AccessVerdict::NotAllowed;
                slots.insert(
                    world_name.to_string(),
                    Slot::Answered(Answer {
                        address,
                        allowed,
                        expires: now + CACHE_TTL,
                    }),
                );
                allowed
            }
            _ => {
                tracing::warn!("world permissions unreadable for {world_name}; allowing");
                slots.remove(world_name);
                true
            }
        }
    })
}
