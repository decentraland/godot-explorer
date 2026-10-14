use std::sync::Mutex;

use crate::app::Parcel;

/// What only the engine knows about where the player is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneFacts {
    pub realm_url: String,
    /// The parcel the player stands on; the engine reports (0, 0) while it has none.
    pub parcel: Parcel,
    /// The scene's own title from its entity definition, `None` while no scene is loaded.
    pub scene_title: Option<String>,
    /// The world name of a world realm; empty in Genesis City.
    pub realm_name: String,
}

impl Default for SceneFacts {
    fn default() -> Self {
        Self {
            realm_url: String::new(),
            parcel: Parcel::new(0, 0),
            scene_title: None,
            realm_name: String::new(),
        }
    }
}

/// What an `update` changed, so the caller can tell a move from a teleport.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SceneChange {
    pub realm_changed: bool,
    pub parcel_changed: bool,
}

/// The latest `SceneFacts`, replaced whole on every engine report.
#[derive(Default)]
pub struct SceneTracker {
    facts: Mutex<SceneFacts>,
}

impl SceneTracker {
    pub fn update(&self, facts: SceneFacts) -> SceneChange {
        let mut current = self.facts.lock().unwrap_or_else(|e| e.into_inner());
        let change = SceneChange {
            realm_changed: current.realm_url != facts.realm_url,
            parcel_changed: current.parcel != facts.parcel,
        };
        *current = facts;
        change
    }

    pub fn facts(&self) -> SceneFacts {
        self.facts.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(realm: &str, x: i32, y: i32) -> SceneFacts {
        SceneFacts {
            realm_url: realm.into(),
            parcel: Parcel::new(x, y),
            ..SceneFacts::default()
        }
    }

    #[test]
    fn update_reports_what_moved() {
        let tracker = SceneTracker::default();
        let first = tracker.update(facts("https://a", 1, 2));
        assert!(first.realm_changed && first.parcel_changed);

        let same_realm = tracker.update(facts("https://a", 3, 2));
        assert!(!same_realm.realm_changed && same_realm.parcel_changed);

        let nothing = tracker.update(facts("https://a", 3, 2));
        assert!(!nothing.realm_changed && !nothing.parcel_changed);
        assert_eq!(tracker.facts().parcel, Parcel::new(3, 2));
    }

    #[test]
    fn a_tracker_with_no_report_has_the_origin_parcel() {
        assert_eq!(SceneTracker::default().facts().parcel, Parcel::new(0, 0));
    }
}
