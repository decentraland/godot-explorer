//! Event reminders: the user's attendance, written to the events service.

use super::signed::{build_url, SignedClient, SignedError};
use crate::urls;

/// Sets (`POST`) or clears (`DELETE`) the user's reminder on an event.
pub async fn set_event_reminder(
    client: &SignedClient,
    event_id: &str,
    on: bool,
) -> Result<(), SignedError> {
    let url = build_url(&urls::events_api(), &[event_id, "attendees"], &[])?;
    let method = if on { "POST" } else { "DELETE" };
    client.request(method, &url, None).await.map(drop)
}

#[cfg(test)]
mod tests {
    use super::super::signed::testing::*;
    use super::*;

    #[tokio::test]
    async fn reminder_posts_and_deletes_the_attendance() {
        for (on, method) in [(true, "POST"), (false, "DELETE")] {
            let mock = Mock::ok("{}");
            set_event_reminder(&client(&mock), "e/1", on).await.unwrap();
            let sent = mock.only_sent();
            assert_eq!(sent.method, method);
            assert_eq!(sent.url, format!("{}/e%2F1/attendees", urls::events_api()));
            assert_eq!(sent.header("x-identity-metadata"), Some("{}"));
        }
    }
}
