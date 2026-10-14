//! Deleting the account. A guest has nothing on the server to ask for: its thirdweb user is
//! removed and the identity files on the device are wiped, so the next "Play as guest" mints a
//! fresh wallet. Everyone else files the server's deletion request.

use std::sync::atomic::Ordering;

use crate::session::store::SessionSlot;

use super::{signin::SignInError, Hub};

/// The guest identity files under the data dir, plus the attestation session (issue #2335).
pub const GUEST_DEVICE_FILES: [&str; 3] = [
    "device_anchor.txt",
    "thirdweb_session.json",
    "attest_session.json",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeletionPlan {
    /// A throwaway wallet: nothing outside the device.
    Local,
    /// A guest that never linked a login: unlink its thirdweb user.
    Guest,
    /// An upgraded guest, deletable only behind the developer link gate: unlink every login.
    UpgradedGuest,
    /// The server's deletion request.
    Server,
}

impl Hub {
    /// Lets developer builds delete an upgraded (email-linked) guest. Production ignores it.
    pub fn allow_upgraded_deletion(&self) {
        if !self.production.load(Ordering::Relaxed) {
            self.upgraded_deletion.store(true, Ordering::Relaxed);
        }
    }

    /// How the signed-in account is deleted. The upgrade state is a fresh query: the cached flag
    /// is cold outside the upgrade card, and a recoverable account must never be misread as a
    /// plain guest. When the query fails the cached flag decides.
    pub async fn deletion_plan(&self) -> DeletionPlan {
        let (disposable, thirdweb, cached) = self.with_session(|s| {
            (
                s.is_disposable_guest(),
                s.is_thirdweb_guest,
                s.is_thirdweb_guest_upgraded,
            )
        });
        if disposable {
            return DeletionPlan::Local;
        }
        if !thirdweb {
            return DeletionPlan::Server;
        }
        let upgraded = match self.backend.guest_upgraded(self.anchor()).await {
            Ok(upgraded) => upgraded,
            Err(e) => {
                tracing::warn!("deletion: the upgrade check failed, using the cached flag: {e}");
                cached
            }
        };
        match (upgraded, self.upgraded_deletion.load(Ordering::Relaxed)) {
            (false, _) => DeletionPlan::Guest,
            (true, true) => DeletionPlan::UpgradedGuest,
            (true, false) => DeletionPlan::Server,
        }
    }

    /// Deletes a guest on the device and signs out. The unlink is best effort: when it fails the
    /// player still loses the local identity, as in 1.x.
    pub async fn delete_guest(&self, plan: DeletionPlan) -> Result<(), SignInError> {
        match plan {
            DeletionPlan::Guest => log_failure(self.backend.unlink_guest(self.anchor()).await),
            DeletionPlan::UpgradedGuest => {
                log_failure(self.backend.unlink_upgraded(self.anchor()).await)
            }
            DeletionPlan::Local => {}
            DeletionPlan::Server => {
                return Err(SignInError::new(
                    "invalid_state",
                    "a server-side deletion is not a guest deletion",
                ))
            }
        }
        self.clear_guest_device_storage();
        self.sign_out()
    }

    fn anchor(&self) -> String {
        self.device_anchor
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Wipes the guest identity files and the cached guest look, so the fresh wallet starts clean.
    fn clear_guest_device_storage(&self) {
        for name in GUEST_DEVICE_FILES {
            match std::fs::remove_file(self.data_dir.join(name)) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => tracing::error!("deletion: cannot remove {name}: {e}"),
            }
        }
        let suffix = self.suffix();
        let account = self
            .store
            .slot(&suffix)
            .map(|slot| slot.account)
            .unwrap_or_default();
        if let Err(e) = self.store.set_slot(
            &suffix,
            SessionSlot {
                account,
                guest_profile: String::new(),
            },
        ) {
            tracing::error!("deletion: cannot clear the cached guest look: {e}");
        }
    }
}

fn log_failure(result: Result<(), anyhow::Error>) {
    if let Err(e) = result {
        tracing::warn!("deletion: the thirdweb unlink failed: {e}");
    }
}
