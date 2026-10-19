//! FFI for the notifications domain.

use super::{core, places::on_core, CoreError};
use crate::{
    analytics::session::unix_now,
    day1_notification::{self, Day1Notification},
    places::{self, Notification, NotificationsQuery},
    settings,
};

/// The user's notifications, newest first. `limit` is clamped to 50. Friend requests from blocked
/// senders are not dropped yet: the blocklist is not in the core.
#[uniffi::export]
pub async fn notifications(
    from: Option<i64>,
    limit: Option<u32>,
    only_unread: bool,
) -> Result<Vec<Notification>, CoreError> {
    let query = NotificationsQuery {
        from,
        limit,
        only_unread,
    };
    on_core(move |client| async move { places::notifications(&client, query, &|_| false).await })
        .await
}

/// Marks the notifications read and returns how many the service updated.
#[uniffi::export]
pub async fn mark_notifications_read(ids: Vec<String>) -> Result<u32, CoreError> {
    on_core(move |client| async move { places::mark_notifications_read(&client, &ids).await }).await
}

/// What the Day 1 welcome needs from the OS right now. The shell calls it whenever the permission
/// or the flags may have changed; arming again with the same id is harmless.
#[uniffi::export]
pub fn day1_notification(permission_granted: bool) -> Result<Day1Notification, CoreError> {
    let settings = settings::global(core()?.data_dir());
    Ok(day1_notification::sync(
        settings,
        permission_granted,
        unix_now(),
    ))
}
