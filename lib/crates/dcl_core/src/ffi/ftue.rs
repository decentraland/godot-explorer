//! FFI for the first-time guidance in the world (`crate::ftue`): the shell reports what the player
//! did, the core decides what shows next, remembers it and records the analytics.

use super::{core, CoreError};
use crate::{
    ftue::{self, ControlsFtueElement, EntryContext, FtueTuning, SceneEntryFtue, TutorialStep},
    settings::{self, Settings},
};

fn store() -> Result<&'static Settings, CoreError> {
    Ok(settings::global(core()?.data_dir()))
}

fn address() -> Result<Option<String>, CoreError> {
    Ok(core()?
        .hub()
        .with_session(|s| s.try_address().map(|a| format!("{a:#x}"))))
}

/// The remote tunables of the overlay and the tutorial.
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
            address: address()?,
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

/// START (`true`) or SKIP on the welcome card.
#[uniffi::export]
pub fn tutorial_welcome_answered(start: bool) -> Result<(), CoreError> {
    ftue::welcome_answered(store()?, start);
    Ok(())
}

#[uniffi::export]
pub fn tutorial_step_entered(step: TutorialStep, is_replay: bool) {
    ftue::step_entered(step, is_replay);
}

#[uniffi::export]
pub fn tutorial_skipped(step: TutorialStep, is_replay: bool) -> Result<(), CoreError> {
    ftue::skipped(store()?, step, is_replay);
    Ok(())
}

#[uniffi::export]
pub fn tutorial_completed(is_replay: bool) -> Result<(), CoreError> {
    ftue::completed(store()?, is_replay);
    Ok(())
}

/// Settings › Gameplay › Replay.
#[uniffi::export]
pub fn tutorial_replay_requested() {
    ftue::replay_requested();
}

/// A profile-less account finished its avatar on this install: the tutorial's "new player".
#[uniffi::export]
pub fn ftue_mark_new_player() -> Result<(), CoreError> {
    let address = address()?.ok_or_else(|| CoreError::InvalidCall {
        detail: "no account is signed in".to_string(),
    })?;
    ftue::mark_new_player(store()?, &address);
    Ok(())
}
