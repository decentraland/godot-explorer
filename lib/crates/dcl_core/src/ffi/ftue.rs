//! FFI for the first-time guidance in the world (`crate::ftue`): the shell reports what the player
//! did, the core decides what shows next, remembers it and records the analytics.

use super::{core, CoreError};
use crate::{
    ftue::{self, ControlsFtueElement, EntryContext, FtueTuning, SceneEntryFtue},
    settings::{self, Settings},
};

fn store() -> Result<&'static Settings, CoreError> {
    Ok(settings::global(core()?.data_dir()))
}

/// The remote tunables of the overlay.
#[uniffi::export]
pub fn ftue_tuning() -> FtueTuning {
    ftue::tuning()
}

/// Once per scene entry, after the loading cover, the first HUD report and any modal. `blocked`:
/// a menu is open, the phone is in portrait or the HUD is hidden.
#[uniffi::export]
pub fn ftue_scene_entered(blocked: bool) -> Result<SceneEntryFtue, CoreError> {
    let core = core()?;
    let hud = core.with_app(|app| app.last_hud().cloned());
    Ok(ftue::scene_entered(
        store()?,
        EntryContext {
            hud: hud.as_ref(),
            sessions: crate::review::global(core).sessions(),
            blocked,
        },
    ))
}

/// The player closed the controls overlay with a tap after `seconds_shown`.
#[uniffi::export]
pub fn controls_ftue_dismissed(
    elements: Vec<ControlsFtueElement>,
    seconds_shown: f64,
) -> Result<(), CoreError> {
    ftue::controls_dismissed(store()?, &elements, seconds_shown);
    Ok(())
}
