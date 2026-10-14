//! What the gates remember between launches, in `gates.json` under the core data dir.
//!
//! Starts fresh: nothing is imported from the settings file the Godot client wrote
//! (`version_gate_snooze_until`, `end_of_device_support_*`, `below_minspec_modal_shown`,
//! `campaign_*`). A user who already dismissed a modal sees it once more after the migration,
//! and an install that had not yet consumed its campaign token loses it.

use std::{
    io,
    path::{Path, PathBuf},
    sync::Mutex,
};

use serde::{Deserialize, Serialize};

pub const FILE: &str = "gates.json";

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GatesState {
    /// Unix seconds until which a recommended update stays dismissed.
    pub version_gate_snooze_until: i64,
    /// Unix seconds of the first session the device was found end-of-support; 0 until then.
    pub end_of_device_support_first_detected_unix: i64,
    /// Which tier of the end-of-support schedule is next; 0 means never shown.
    pub end_of_device_support_modal_shown_count: u32,
    pub below_minspec_modal_shown: bool,
    /// The ad-campaign token of this install and when it was captured (unix seconds).
    pub campaign_token: String,
    pub campaign_token_captured_at: i64,
    /// One campaign per install: set once a launch booted into its target.
    pub campaign_consumed: bool,
}

pub struct StateStore {
    path: Option<PathBuf>,
    state: Mutex<GatesState>,
}

impl StateStore {
    /// Reads `path`; a missing file is a fresh state, an unreadable one is too, with a warning.
    pub fn open(path: PathBuf) -> Self {
        let state = match std::fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
                tracing::warn!(
                    "[Gates] {} is not valid ({e}), starting fresh",
                    path.display()
                );
                GatesState::default()
            }),
            Err(e) if e.kind() == io::ErrorKind::NotFound => GatesState::default(),
            Err(e) => {
                tracing::warn!(
                    "[Gates] cannot read {} ({e}), starting fresh",
                    path.display()
                );
                GatesState::default()
            }
        };
        Self {
            path: Some(path),
            state: Mutex::new(state),
        }
    }

    /// A store that never touches the disk.
    pub fn in_memory() -> Self {
        Self {
            path: None,
            state: Mutex::new(GatesState::default()),
        }
    }

    pub fn get(&self) -> GatesState {
        self.lock().clone()
    }

    /// Replaces the state with `change(current)` and writes it. The change is kept in memory
    /// even when the write fails, so the session still behaves; the error is for the caller.
    pub fn update(&self, change: impl FnOnce(&GatesState) -> GatesState) -> io::Result<()> {
        let mut state = self.lock();
        let next = change(&state);
        *state = next.clone();
        match &self.path {
            Some(path) => write(path, &next),
            None => Ok(()),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, GatesState> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// Through a temp file and a rename, so a kill mid-write never leaves half a file.
fn write(path: &Path, state: &GatesState) -> io::Result<()> {
    let text = serde_json::to_string_pretty(state)?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_file_starts_fresh_and_updates_persist() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE);
        let store = StateStore::open(path.clone());
        assert_eq!(store.get(), GatesState::default());

        store
            .update(|s| GatesState {
                below_minspec_modal_shown: true,
                campaign_token: "summer-26".into(),
                ..s.clone()
            })
            .unwrap();

        let reopened = StateStore::open(path).get();
        assert!(reopened.below_minspec_modal_shown);
        assert_eq!(reopened.campaign_token, "summer-26");
        assert_eq!(reopened.end_of_device_support_modal_shown_count, 0);
    }

    #[test]
    fn a_corrupt_file_starts_fresh() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE);
        std::fs::write(&path, "{not json").unwrap();
        assert_eq!(StateStore::open(path).get(), GatesState::default());
    }

    #[test]
    fn unknown_and_missing_fields_are_tolerated() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE);
        std::fs::write(
            &path,
            r#"{"campaign_consumed": true, "from_the_future": 1}"#,
        )
        .unwrap();
        let state = StateStore::open(path).get();
        assert!(state.campaign_consumed);
        assert_eq!(state.version_gate_snooze_until, 0);
    }

    #[test]
    fn a_failed_write_still_updates_the_session_and_reports() {
        let store = StateStore {
            path: Some(PathBuf::from("/nonexistent-dir-for-gates/gates.json")),
            state: Mutex::new(GatesState::default()),
        };
        let result = store.update(|s| GatesState {
            campaign_consumed: true,
            ..s.clone()
        });
        assert!(result.is_err());
        assert!(store.get().campaign_consumed);
    }
}
