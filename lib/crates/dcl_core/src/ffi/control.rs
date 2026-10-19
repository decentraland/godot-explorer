//! Driving the app and the engine from the shell: entering and leaving worlds, the avatar
//! preview, and the typed commands the running engine executes.

use super::{core, CoreError};
use crate::app::{AvatarState, Control, Destination, Rect};

fn invalid(e: crate::app::AppError) -> CoreError {
    CoreError::InvalidCall {
        detail: e.to_string(),
    }
}

/// The host or the shell drew the first screen: the point at which the engine may start early.
#[uniffi::export]
pub fn first_screen_drawn() -> Result<(), CoreError> {
    core()?.with_app(|app| app.first_screen_drawn());
    Ok(())
}

/// Enters a world; also valid while loading or in one, where it replaces the destination.
#[uniffi::export]
pub fn enter_world(destination: Destination) -> Result<(), CoreError> {
    core()?
        .with_app(|app| app.enter_world(destination))
        .map_err(invalid)
}

/// Leaves the world and keeps the session.
#[uniffi::export]
pub fn exit_world() -> Result<(), CoreError> {
    core()?.with_app(|app| app.exit_world()).map_err(invalid)
}

/// Shows (or moves) the avatar preview over `rect`, in physical pixels.
#[uniffi::export]
pub fn show_avatar_preview(rect: Rect) -> Result<(), CoreError> {
    core()?
        .with_app(|app| app.show_avatar_preview(rect))
        .map_err(invalid)
}

#[uniffi::export]
pub fn hide_avatar_preview() -> Result<(), CoreError> {
    core()?
        .with_app(|app| app.hide_avatar_preview())
        .map_err(invalid)
}

/// A command for the running engine; ignored when there is nothing to apply it to.
#[uniffi::export]
pub fn control(command: Control) -> Result<(), CoreError> {
    core()?.with_app(|app| app.control(command));
    Ok(())
}

/// The equipped avatar as the engine last reported it.
#[uniffi::export]
pub fn avatar_state() -> Result<Option<AvatarState>, CoreError> {
    Ok(core()?.with_app(|app| app.avatar().cloned()))
}
