//! The scene the player is in: what the engine reports, what the core derives from it, and the
//! screenshot seam. The engine owns the scene and realm facts and pushes them in; the shell asks
//! for the derived `CurrentScene`.

mod current;
mod facts;
mod global;
mod screenshot;

pub use current::{
    current_scene, parse_place, place_by_names_url, place_by_position_url, CurrentScene, Place,
    PlaceSource, ReqwestPlaceSource,
};
pub use facts::{SceneChange, SceneFacts, SceneTracker};
pub use global::{scene_tracker, screenshot_provider, set_screenshot_provider, update_scene};
pub use screenshot::{capture_screenshot, ScreenshotError, ScreenshotProvider};
