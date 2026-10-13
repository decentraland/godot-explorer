//! `session.json` in the data dir: the saved account and guest profile per profile slot, plus the
//! analytics user id. Values are opaque strings chosen by the host, so no engine type leaks in.

use std::{
    collections::BTreeMap,
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
};

use serde::{Deserialize, Serialize};

const FILE_NAME: &str = "session.json";

/// One writer at a time: every update is a read-modify-write of the whole file.
static WRITE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSlot {
    #[serde(default)]
    pub account: String,
    #[serde(default)]
    pub guest_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct SessionFile {
    /// Keyed by the profile suffix (`""` for the default slot, `"_name"` for a saved profile).
    #[serde(default)]
    slots: BTreeMap<String, SessionSlot>,
    #[serde(default)]
    analytics_user_id: String,
}

pub struct SessionStore {
    path: PathBuf,
}

impl SessionStore {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            path: data_dir.join(FILE_NAME),
        }
    }

    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    pub fn slot(&self, suffix: &str) -> Option<SessionSlot> {
        self.read().slots.get(suffix).cloned()
    }

    pub fn set_slot(&self, suffix: &str, slot: SessionSlot) -> std::io::Result<()> {
        self.update(|file| {
            file.slots.insert(suffix.to_string(), slot);
        })
    }

    pub fn analytics_user_id(&self) -> String {
        self.read().analytics_user_id
    }

    pub fn set_analytics_user_id(&self, id: &str) -> std::io::Result<()> {
        self.update(|file| file.analytics_user_id = id.to_string())
    }

    /// A missing or unreadable file reads as empty: a damaged file must not lock the user out of
    /// signing in again.
    fn read(&self) -> SessionFile {
        match std::fs::read_to_string(&self.path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
                tracing::warn!(
                    "session store: {} is unreadable ({e}), ignoring it",
                    FILE_NAME
                );
                SessionFile::default()
            }),
            Err(_) => SessionFile::default(),
        }
    }

    fn update(&self, change: impl FnOnce(&mut SessionFile)) -> std::io::Result<()> {
        let _guard = WRITE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut file = self.read();
        change(&mut file);
        let json = serde_json::to_vec_pretty(&file)?;

        // Write beside the target and rename, so a crash never leaves half a session on disk.
        let tmp = self.path.with_extension("json.tmp");
        let mut out = std::fs::File::create(&tmp)?;
        out.write_all(&json)?;
        out.sync_all()?;
        std::fs::rename(&tmp, &self.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("dcl_core_store_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn slots_round_trip_and_stay_independent() {
        let dir = temp_dir("slots");
        let store = SessionStore::new(&dir);
        assert!(!store.exists());
        assert_eq!(store.slot(""), None);

        let default = SessionSlot {
            account: "{a}".into(),
            guest_profile: "{p}".into(),
        };
        store.set_slot("", default.clone()).unwrap();
        store
            .set_slot(
                "_qa",
                SessionSlot {
                    account: "{qa}".into(),
                    guest_profile: String::new(),
                },
            )
            .unwrap();

        let reopened = SessionStore::new(&dir);
        assert!(reopened.exists());
        assert_eq!(reopened.slot(""), Some(default));
        assert_eq!(reopened.slot("_qa").unwrap().account, "{qa}");
    }

    #[test]
    fn analytics_id_and_slots_do_not_clobber_each_other() {
        let dir = temp_dir("analytics");
        let store = SessionStore::new(&dir);
        store.set_analytics_user_id("uuid-1").unwrap();
        store.set_slot("", SessionSlot::default()).unwrap();
        assert_eq!(store.analytics_user_id(), "uuid-1");
    }

    #[test]
    fn a_damaged_file_reads_as_empty_and_can_be_rewritten() {
        let dir = temp_dir("damaged");
        std::fs::write(dir.join(FILE_NAME), "{not json").unwrap();
        let store = SessionStore::new(&dir);
        assert_eq!(store.slot(""), None);
        store.set_analytics_user_id("x").unwrap();
        assert_eq!(store.analytics_user_id(), "x");
    }
}
