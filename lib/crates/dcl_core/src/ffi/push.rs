//! FFI for the push registration: the shell reads the token and the permission from the OS and
//! hands them over; the core decides when the `identify` goes out.

use super::{core, CoreError};
use crate::{
    analytics::{
        global::analytics,
        push::{registrar, PushRegistration},
    },
    flags,
};

/// Registers how this install can be reached by push. Sent once the `push-enabled` flag is known
/// and on; a token or permission change sends again.
#[uniffi::export]
pub fn set_push_registration(registration: PushRegistration) -> Result<(), CoreError> {
    core()?;
    let analytics = analytics().ok_or_else(|| CoreError::InvalidCall {
        detail: "analytics is not installed".into(),
    })?;
    let flag = flags::is_loaded().then(|| flags::is_enabled("push-enabled", true));
    let outcome = registrar().register(analytics, registration, flag);
    tracing::debug!("[Push] registration: {outcome:?}");
    Ok(())
}
