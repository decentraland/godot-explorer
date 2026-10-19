//! FFI for the `App Opened` chain: the shell reports launch, foreground, background and a tick.

use super::{core, CoreError};
use crate::analytics::session::{self, global};

fn tracker() -> Result<&'static session::SessionTracker, CoreError> {
    global(core()?).ok_or_else(|| CoreError::InvalidCall {
        detail: "analytics is not installed".into(),
    })
}

/// The process started; call once, as soon as the shell is up.
#[uniffi::export]
pub fn session_launched() -> Result<(), CoreError> {
    crate::review::global(core()?).session_started();
    session::emit(tracker()?.launched(session::unix_now(), session::monotonic_ms()));
    Ok(())
}

/// The app came back from the background.
#[uniffi::export]
pub fn session_foregrounded() -> Result<(), CoreError> {
    session::emit(tracker()?.foregrounded(session::unix_now(), session::monotonic_ms()));
    Ok(())
}

/// The app left the foreground.
#[uniffi::export]
pub fn session_backgrounded() -> Result<(), CoreError> {
    tracker()?.backgrounded(session::unix_now(), session::monotonic_ms());
    Ok(())
}

/// Every few seconds while the app runs: ends the settle window and keeps the liveness mark fresh.
#[uniffi::export]
pub fn session_tick() -> Result<(), CoreError> {
    session::emit(tracker()?.tick(session::unix_now(), session::monotonic_ms()));
    Ok(())
}
