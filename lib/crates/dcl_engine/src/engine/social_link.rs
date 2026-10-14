//! Engine side of the core's social contract: who is nearby and who is blocked. The avatars and
//! the blocklist are main-thread Godot objects and the core asks from its own threads, so the
//! engine mirrors both into a snapshot a few times a second.

use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use dcl_core::social::{friends, NearbyProvider};
use dcl_core::Core;
use ethers_core::types::H160;
use godot::prelude::*;

use super::settings_link::{on_process_frame, run_on_main};
use crate::godot_classes::dcl_global::DclGlobal;

const REFRESH_INTERVAL: Duration = Duration::from_millis(250);

#[derive(Default)]
struct Snapshot {
    nearby: Vec<String>,
    blocked: HashSet<H160>,
}

#[derive(Default)]
struct MirroredNearby {
    snapshot: Mutex<Snapshot>,
}

impl MirroredNearby {
    fn replace(&self, next: Snapshot) {
        *self.snapshot.lock().unwrap_or_else(|e| e.into_inner()) = next;
    }
}

impl NearbyProvider for MirroredNearby {
    fn nearby_addresses(&self) -> Vec<String> {
        let snapshot = self.snapshot.lock().unwrap_or_else(|e| e.into_inner());
        snapshot.nearby.clone()
    }

    fn is_blocked(&self, address: &str) -> bool {
        let Ok(address) = address.parse::<H160>() else {
            return false;
        };
        let snapshot = self.snapshot.lock().unwrap_or_else(|e| e.into_inner());
        snapshot.blocked.contains(&address)
    }

    /// The mirror follows at once, and the engine's own list on the main thread.
    fn blocked_changed(&self, address: &str, blocked: bool) {
        if let Ok(parsed) = address.parse::<H160>() {
            let mut snapshot = self.snapshot.lock().unwrap_or_else(|e| e.into_inner());
            if blocked {
                snapshot.blocked.insert(parsed);
            } else {
                snapshot.blocked.remove(&parsed);
            }
        }
        let address = GString::from(address);
        run_on_main(move || {
            let Some(global) = DclGlobal::try_singleton() else {
                return;
            };
            let mut blacklist = global.bind().social_blacklist.clone();
            if blocked {
                blacklist.bind_mut().add_blocked(address);
            } else {
                blacklist.bind_mut().remove_blocked(address);
            }
        });
    }
}

/// Registers the nearby provider with the core. Called once when the engine starts.
pub(super) fn register() {
    if Core::try_get().is_none() {
        godot_error!("social_link: no core to register with");
        return;
    }
    let mirror = Arc::new(MirroredNearby::default());
    friends().set_nearby_provider(mirror.clone());

    let mut last_refresh: Option<Instant> = None;
    on_process_frame("mirror_nearby", move || {
        if last_refresh.is_some_and(|at| at.elapsed() < REFRESH_INTERVAL) {
            return;
        }
        last_refresh = Some(Instant::now());
        if let Some(snapshot) = read_snapshot() {
            mirror.replace(snapshot);
        }
    });
}

/// The addresses of the remote avatars in the scene and the local blocklist. Main thread.
fn read_snapshot() -> Option<Snapshot> {
    let global = DclGlobal::try_singleton()?;
    let avatars = global.bind().avatars.clone();
    let blacklist = global.bind().social_blacklist.clone();

    let nearby = avatars
        .bind()
        .get_avatars()
        .iter_shared()
        .filter_map(|avatar| avatar.get("avatar_id").try_to::<GString>().ok())
        .map(|address| address.to_string())
        .collect();
    let blocked = blacklist
        .bind()
        .get_blocked_list()
        .iter_shared()
        .filter_map(|address| address.to_string().parse::<H160>().ok())
        .collect();
    Some(Snapshot { nearby, blocked })
}

#[cfg(test)]
mod tests {
    use super::*;

    const ADDRESS: &str = "0x00000000000000000000000000000000000000ab";

    fn mirror(nearby: &[&str], blocked: &[&str]) -> MirroredNearby {
        let mirror = MirroredNearby::default();
        mirror.replace(Snapshot {
            nearby: nearby.iter().map(|a| a.to_string()).collect(),
            blocked: blocked.iter().filter_map(|a| a.parse().ok()).collect(),
        });
        mirror
    }

    #[test]
    fn nearby_addresses_are_the_mirrored_ones() {
        let mirror = mirror(&[ADDRESS], &[]);
        assert_eq!(mirror.nearby_addresses(), vec![ADDRESS.to_string()]);
    }

    #[test]
    fn blocked_matches_any_case_and_ignores_garbage() {
        let mirror = mirror(&[], &[ADDRESS]);
        assert!(mirror.is_blocked(&ADDRESS.to_uppercase().replace("0X", "0x")));
        assert!(!mirror.is_blocked("0x0000000000000000000000000000000000000001"));
        assert!(!mirror.is_blocked("not an address"));
        assert!(!mirror.is_blocked(""));
    }
}
