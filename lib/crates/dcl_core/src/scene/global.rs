use std::sync::{Arc, Mutex, OnceLock};

use super::{
    facts::{SceneFacts, SceneTracker},
    screenshot::ScreenshotProvider,
};
use crate::analytics::global::analytics;

static TRACKER: OnceLock<SceneTracker> = OnceLock::new();
static SCREENSHOT: Mutex<Option<Arc<dyn ScreenshotProvider>>> = Mutex::new(None);

pub fn scene_tracker() -> &'static SceneTracker {
    TRACKER.get_or_init(SceneTracker::default)
}

/// The engine's report of where the player is. Analytics follows: the realm is stamped on every
/// event and a new parcel is an "Explorer Move To Parcel".
pub fn update_scene(facts: SceneFacts) {
    let (realm, parcel) = (facts.realm_url.clone(), facts.parcel);
    let change = scene_tracker().update(facts);
    let Some(analytics) = analytics() else { return };
    if change.realm_changed {
        analytics.update_realm(&realm);
    }
    if change.parcel_changed {
        analytics.update_position(&format!("{},{}", parcel.x, parcel.y));
    }
}

/// The engine registers its capture when it starts and clears it (`None`) when it stops.
pub fn set_screenshot_provider(provider: Option<Arc<dyn ScreenshotProvider>>) {
    *SCREENSHOT.lock().unwrap_or_else(|e| e.into_inner()) = provider;
}

pub fn screenshot_provider() -> Option<Arc<dyn ScreenshotProvider>> {
    SCREENSHOT.lock().unwrap_or_else(|e| e.into_inner()).clone()
}
