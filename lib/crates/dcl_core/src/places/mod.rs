//! Places, events, credits and notifications: the calls that need the identity key (D-10). Public
//! read-only GETs stay in the shell; each signed call here is typed, built on one internal
//! signed request, and no generic signed fetch is exported.

mod bug_report;
mod credits;
mod events;
mod favorites;
mod notifications_client;
mod signed;

pub use bug_report::{issue_types, submit_bug_report, BugReportInput, IssueType};
pub use credits::{credits_balance, CreditsBalance};
pub use events::set_event_reminder;
pub use favorites::{
    favorite_places, place_user_state, set_favorite, vote, Place, PlaceUserState, Vote,
};
pub use notifications_client::{
    mark_notifications_read, notifications, Notification, NotificationKind, NotificationSender,
    Query as NotificationsQuery,
};
pub use signed::{Reply, ReqwestTransport, SignedClient, SignedError, Transport};
