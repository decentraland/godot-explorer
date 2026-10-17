//! FFI for the review prompt policy: the shell reports the triggers and the guard rails, the core
//! decides and counts; the store's card is launched by the shell.

use super::{core, CoreError};
use crate::{
    analytics::{global::analytics, session::unix_now},
    review::{self, Rail, Verdict},
};

/// A poor connection is a rail of its own: the core knows it, the shell does not.
fn with_connection(mut rails: Vec<Rail>) -> Vec<Rail> {
    if !crate::connection::global().is_good() {
        rails.push(Rail::Connection);
    }
    rails
}

/// A trigger arrived: whether to wait the beat and fire. `rails` are the guard rails that are up.
#[uniffi::export]
pub fn review_check(rails: Vec<Rail>) -> Result<bool, CoreError> {
    let rails = with_connection(rails);
    let verdict = review::global(core()?).verdict(unix_now(), &rails);
    tracing::debug!("[Review] {verdict:?}");
    Ok(matches!(verdict, Verdict::Ask(_)))
}

/// The beat passed: asks again and, when still fine, counts the shot and reports it. Returns the
/// shot number to launch the card for, or nothing.
#[uniffi::export]
pub fn review_fire(trigger: String, rails: Vec<Rail>) -> Result<Option<u32>, CoreError> {
    let rails = with_connection(rails);
    let fired = review::global(core()?).fire(&trigger, unix_now(), &rails);
    Ok(fired.map(|(shot, event)| {
        if let Some(analytics) = analytics() {
            analytics.queue_event(event);
        }
        shot
    }))
}
