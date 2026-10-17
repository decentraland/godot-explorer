//! Bug reports as Intercom tickets, through the intercom-proxy (the client never talks to Intercom).
//!
//! The contract mirrors the Unity Explorer client, so mobile and desktop tickets are shaped the
//! same: the body accepts only `ticket_attributes` and `evidence` at the top level, and every
//! attribute name must be declared on the Bug Report ticket type, or the whole ticket is rejected.

use std::fs;

use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};

use super::signed::{SignedClient, SignedError};
use crate::urls;

/// One image over this is dropped rather than sinking the report (the proxy rejects the ticket).
pub const MAX_EVIDENCE_BYTES: usize = 3 * 1024 * 1024;
pub const MAX_EVIDENCE_COUNT: usize = 3;
/// Across all images: the proxy caps the whole body at 4.5 MB and base64 grows data by 4/3.
pub const TOTAL_EVIDENCE_BYTES: usize = 3 * 1024 * 1024;
/// The ticket type's Platform list: 1 Desktop, 2 Mobile.
const PLATFORM_MOBILE: i64 = 2;
pub const DESCRIPTION_MAX_CHARS: usize = 300;

/// An Intercom option of the Bug Report ticket type, from the Unity client's list; the proxy
/// validates the ids. The label is English on purpose: it titles the ticket support reads.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct IssueType {
    pub value: String,
    pub label: String,
    pub uuid: String,
}

pub fn issue_types() -> Vec<IssueType> {
    [
        (
            "performance",
            "Performance",
            "84d3e47f-396f-40be-bb93-a8b36196cf97",
        ),
        (
            "crash",
            "Crash / Freeze",
            "10ab00f9-e944-4a7f-8b75-c8bf4e4ff270",
        ),
        ("chat", "Chat", "b2db7b2e-3634-4c9d-9f55-b732bfe41319"),
        (
            "streaming",
            "Streaming",
            "dbceaef0-c69c-409b-afb3-5e9523a4dec5",
        ),
        ("friends", "Friends", "4d3a9289-da5a-47a6-a3e0-772effdd78f0"),
        (
            "wearables",
            "Wearables/Emotes",
            "e4e9abb6-8304-48eb-a622-b2516e0a1719",
        ),
        ("profile", "Profile", "30f4a7a7-6366-4e05-9393-d1419a5b4008"),
        ("other", "Other", "30b90385-7138-4d42-99aa-87eeb1c85619"),
    ]
    .into_iter()
    .map(|(value, label, uuid)| IssueType {
        value: value.into(),
        label: label.into(),
        uuid: uuid.into(),
    })
    .collect()
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct BugReportInput {
    /// An `IssueType::value`.
    pub category: String,
    pub description: String,
    /// Files with JPEG screenshots, at most three; unreadable or oversized ones are left out.
    pub screenshots: Vec<String>,
    pub operating_system: String,
    /// Brand and model; they ride in the OS attribute, the ticket type has none of its own.
    pub device: String,
    pub graphics_card: String,
    pub ram_mb: i64,
    pub client_version: String,
    /// `x,y` while in a world, empty elsewhere.
    pub coordinates: String,
}

/// The images that fit: each gets the remaining budget divided by the images still to come, so
/// one that is small or dropped leaves more room for the rest.
pub fn fit_evidence(images: Vec<Vec<u8>>) -> Vec<Vec<u8>> {
    let images: Vec<_> = images
        .into_iter()
        .filter(|bytes| !bytes.is_empty())
        .take(MAX_EVIDENCE_COUNT)
        .collect();
    let total = images.len();
    let mut remaining = TOTAL_EVIDENCE_BYTES;
    let mut kept = Vec::new();
    for (index, bytes) in images.into_iter().enumerate() {
        let budget = MAX_EVIDENCE_BYTES.min(remaining / (total - index));
        if bytes.len() > budget {
            tracing::warn!(
                "bug report: attachment is {} bytes (budget {budget}), filing without it",
                bytes.len()
            );
            continue;
        }
        remaining -= bytes.len();
        kept.push(bytes);
    }
    kept
}

pub fn build_body(input: &BugReportInput, evidence: &[Vec<u8>]) -> Value {
    let issue = issue_types()
        .into_iter()
        .find(|issue| issue.value == input.category)
        .unwrap_or_else(|| issue_types().pop().expect("the list is not empty"));
    let description: String = input
        .description
        .trim()
        .chars()
        .take(DESCRIPTION_MAX_CHARS)
        .collect();
    let mut lines = vec![description.as_str(), "", "---"];
    let coordinates = format!("Coordinates: {}", input.coordinates);
    if !input.coordinates.is_empty() {
        lines.push(&coordinates);
    }
    let os = if input.device.trim().is_empty() {
        input.operating_system.clone()
    } else {
        format!("{} ({})", input.operating_system, input.device.trim())
    };
    let attributes = json!({
        "_default_title_": format!("Bug Report: {}", issue.label),
        "_default_description_": lines.join("\n"),
        "Issue Type": issue.uuid,
        "Operating System": os,
        "Graphic Card": input.graphics_card,
        "RAM": if input.ram_mb > 0 { format!("{} MB", input.ram_mb) } else { String::new() },
        "Client version": input.client_version,
        "Platform": PLATFORM_MOBILE,
    });
    let mut body = json!({ "ticket_attributes": attributes });
    if !evidence.is_empty() {
        body["evidence"] = Value::Array(
            evidence
                .iter()
                .map(
                    |bytes| json!({ "content_type": "image/jpeg", "data": STANDARD.encode(bytes) }),
                )
                .collect(),
        );
    }
    body
}

/// Files the report; returns the Intercom ticket id.
pub async fn submit_bug_report(
    client: &SignedClient,
    input: &BugReportInput,
) -> Result<String, SignedError> {
    if input.category.is_empty() {
        return Err(SignedError::InvalidArgument("missing issue type".into()));
    }
    if input.description.trim().is_empty() {
        return Err(SignedError::InvalidArgument("missing description".into()));
    }
    let images = input
        .screenshots
        .iter()
        .filter_map(|path| fs::read(path.trim_start_matches("file://")).ok())
        .collect();
    let body = build_body(input, &fit_evidence(images));
    let reply = client
        .request_with_metadata(
            "POST",
            &urls::intercom_tickets(),
            &body,
            &[("Origin".to_string(), urls::intercom_origin())],
        )
        .await?;
    let json: Value =
        serde_json::from_str(&reply.body).map_err(|e| SignedError::Malformed(e.to_string()))?;
    // The proxy echoes Intercom's ticket; its absence means no ticket was created.
    match json
        .get("id")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())
    {
        Some(id) => Ok(id.to_string()),
        None => Err(SignedError::Malformed(
            json.get("error")
                .and_then(Value::as_str)
                .unwrap_or("ticket not created")
                .to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::super::signed::testing::*;
    use super::*;

    fn input() -> BugReportInput {
        BugReportInput {
            category: "crash".into(),
            description: "  It froze  ".into(),
            operating_system: "Android 16".into(),
            device: "motorola moto g60".into(),
            graphics_card: "Adreno".into(),
            ram_mb: 5900,
            client_version: "2.0.0".into(),
            coordinates: "10,-4".into(),
            ..Default::default()
        }
    }

    #[test]
    fn the_ticket_attributes_match_the_contract() {
        let body = build_body(&input(), &[]);
        let attributes = &body["ticket_attributes"];
        assert_eq!(attributes["_default_title_"], "Bug Report: Crash / Freeze");
        assert_eq!(
            attributes["_default_description_"],
            "It froze\n\n---\nCoordinates: 10,-4"
        );
        assert_eq!(
            attributes["Issue Type"],
            "10ab00f9-e944-4a7f-8b75-c8bf4e4ff270"
        );
        assert_eq!(
            attributes["Operating System"],
            "Android 16 (motorola moto g60)"
        );
        assert_eq!(attributes["RAM"], "5900 MB");
        assert_eq!(attributes["Platform"], 2);
        assert!(body.get("evidence").is_none());
    }

    #[test]
    fn no_coordinates_outside_a_world_and_an_unknown_category_is_other() {
        let mut report = input();
        report.coordinates.clear();
        report.category = "nonsense".into();
        let body = build_body(&report, &[]);
        assert_eq!(
            body["ticket_attributes"]["_default_description_"],
            "It froze\n\n---"
        );
        assert_eq!(
            body["ticket_attributes"]["_default_title_"],
            "Bug Report: Other"
        );
    }

    #[test]
    fn the_description_is_cut_at_300_characters() {
        let mut report = input();
        report.description = "é".repeat(400);
        let text = build_body(&report, &[])["ticket_attributes"]["_default_description_"]
            .as_str()
            .unwrap()
            .to_string();
        assert_eq!(text.chars().take_while(|c| *c == 'é').count(), 300);
    }

    #[test]
    fn evidence_is_limited_in_count_and_in_bytes() {
        let kb = |n: usize| vec![1u8; n * 1024];
        assert_eq!(fit_evidence(vec![kb(10), kb(20), kb(30), kb(40)]).len(), 3);
        // 3 MiB shared by 3 images: 1 MiB each; the 2 MiB one does not fit its share
        let kept = fit_evidence(vec![kb(512), kb(2048), kb(100)]);
        assert_eq!(
            kept.iter().map(Vec::len).collect::<Vec<_>>(),
            vec![512 * 1024, 100 * 1024]
        );
        // a big image over the single limit is dropped, the rest stay
        assert_eq!(fit_evidence(vec![kb(4096), kb(10)]).len(), 1);
        assert!(fit_evidence(vec![vec![], vec![]]).is_empty());
    }

    #[test]
    fn evidence_travels_as_base64_jpeg() {
        let body = build_body(&input(), &[vec![1, 2, 3]]);
        assert_eq!(body["evidence"][0]["content_type"], "image/jpeg");
        assert_eq!(body["evidence"][0]["data"], "AQID");
    }

    #[tokio::test]
    async fn a_report_is_posted_signed_with_an_empty_metadata_and_the_origin() {
        let mock = Mock::ok(r#"{"id":"ticket-7"}"#);
        let id = submit_bug_report(&client(&mock), &input()).await.unwrap();
        assert_eq!(id, "ticket-7");
        let sent = mock.only_sent();
        assert_eq!(sent.method, "POST");
        assert!(sent.url.ends_with("/intercom/tickets"));
        assert_eq!(
            sent.header("Origin"),
            Some(urls::intercom_origin().as_str())
        );
        assert!(sent.header("x-identity-auth-chain-0").is_some());
        let body: Value = serde_json::from_str(sent.body.as_deref().unwrap()).unwrap();
        assert!(body["ticket_attributes"].is_object());
    }

    #[tokio::test]
    async fn a_missing_ticket_id_or_input_is_an_error() {
        let mock = Mock::ok(r#"{"error":"nope"}"#);
        let error = submit_bug_report(&client(&mock), &input())
            .await
            .unwrap_err();
        assert_eq!(error, SignedError::Malformed("nope".into()));

        let mock = Mock::ok("{}");
        let mut empty = input();
        empty.description = "  ".into();
        assert!(matches!(
            submit_bug_report(&client(&mock), &empty).await,
            Err(SignedError::InvalidArgument(_))
        ));
        empty.description = "x".into();
        empty.category.clear();
        assert!(submit_bug_report(&client(&mock), &empty).await.is_err());
    }
}
