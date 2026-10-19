use std::sync::Arc;

use serde_json::json;

use super::*;
use crate::{
    app::Parcel,
    gates::{
        campaign::{CampaignFallback, CampaignKind},
        device::{ReportVerdict, SupportModal},
        state::GatesState,
        test_support::FakeFetch,
        version::UpdateDecision,
    },
};

const NOW: i64 = 1_700_000_000;
const DAY: i64 = 86_400;

fn now() -> i64 {
    NOW
}

fn setup(fetch: FakeFetch, platform: Platform) -> (Gates, Arc<FakeFetch>) {
    setup_with(fetch, platform, StateStore::in_memory())
}

fn setup_with(fetch: FakeFetch, platform: Platform, store: StateStore) -> (Gates, Arc<FakeFetch>) {
    let fetch = Arc::new(fetch);
    let gates = Gates::with_clock(fetch.clone(), store, platform, "1.15.0", now);
    (gates, fetch)
}

fn facts(ram: i32) -> DeviceFacts {
    DeviceFacts {
        total_ram_mb: ram,
        soc_model: "SM6125".into(),
        ..Default::default()
    }
}

fn versions(min: i64, recommended: i64) -> serde_json::Value {
    json!({"ok": true, "data": {
        "android": {"minimalRequiredVersionNumber": min, "recommendedVersionNumber": recommended},
        "ios": {"minimalRequiredVersionNumber": min, "recommendedVersionNumber": recommended},
    }})
}

#[tokio::test]
async fn the_update_gate_reads_the_v2_track_and_honours_the_snooze() {
    let url = backend_urls::app_versions();
    let (gates, fetch) = setup(
        FakeFetch::default().get(&url, Ok(versions(0, 999_999))),
        Platform::Android,
    );

    assert_eq!(
        gates.version_gate().await.decision,
        UpdateDecision::Optional
    );
    gates.snooze_version_gate().unwrap();
    assert_eq!(gates.version_gate().await.decision, UpdateDecision::Proceed);
    assert_eq!(gates.store.get().version_gate_snooze_until, NOW + DAY);
    assert_eq!(fetch.gets(), vec![url.clone(), url]);
}

#[tokio::test]
async fn a_required_update_ignores_the_snooze() {
    let url = backend_urls::app_versions();
    let store = StateStore::in_memory();
    store
        .update(|_| GatesState {
            version_gate_snooze_until: NOW + DAY,
            ..Default::default()
        })
        .unwrap();
    let (gates, _) = setup_with(
        FakeFetch::default().get(&url, Ok(versions(999_999, 999_999))),
        Platform::Ios,
        store,
    );
    assert_eq!(
        gates.version_gate().await.decision,
        UpdateDecision::Required
    );
}

#[tokio::test]
async fn a_low_ram_android_is_end_of_support_stamped_once_and_skips_the_update_gate() {
    let (gates, fetch) = setup(FakeFetch::default(), Platform::Android);

    let support = gates.device_support(&facts(2048)).await.unwrap();
    assert_eq!(support.status, SupportStatus::EndOfSupport);
    assert_eq!(support.verdict, ReportVerdict::EndOfSupport);
    assert_eq!(support.modal, Some(SupportModal::EndOfSupport));
    assert_eq!(
        gates.store.get().end_of_device_support_first_detected_unix,
        NOW
    );

    // The excluded device never asks whether it should update.
    assert_eq!(gates.version_gate().await.decision, UpdateDecision::Proceed);
    assert!(
        fetch.gets().is_empty(),
        "no network for a RAM verdict or the update gate"
    );

    gates
        .device_support_modal_shown(SupportModal::EndOfSupport)
        .unwrap();
    assert_eq!(gates.store.get().end_of_device_support_modal_shown_count, 1);
    assert_eq!(
        gates.device_support(&facts(2048)).await.unwrap().modal,
        None
    );
}

#[tokio::test]
async fn the_chipset_lookup_runs_once_per_run_and_below_minspec_shows_once() {
    let fetch = FakeFetch::default().get(
        &format!("{}?soc=SM6125", backend_urls::device_support()),
        Ok(json!({"ok": true, "data": {"decision": "below-minspec"}})),
    );
    let (gates, fetch) = setup(fetch, Platform::Android);

    let first = gates.device_support(&facts(6144)).await.unwrap();
    assert_eq!(first.status, SupportStatus::BelowMinspec);
    assert_eq!(first.modal, Some(SupportModal::BelowMinspec));
    assert_eq!(first.soc, "SM6125");
    assert_eq!(
        gates.store.get().end_of_device_support_first_detected_unix,
        0
    );

    gates
        .device_support_modal_shown(SupportModal::BelowMinspec)
        .unwrap();
    let second = gates.device_support(&facts(6144)).await.unwrap();
    assert_eq!(
        (second.status, second.modal),
        (SupportStatus::BelowMinspec, None)
    );
    assert_eq!(fetch.gets().len(), 1);
}

#[tokio::test]
async fn ios_and_desktop_never_ask_the_server() {
    let (ios, fetch) = setup(FakeFetch::default(), Platform::Ios);
    let phone = DeviceFacts {
        model: "iPhone12,1".into(),
        total_ram_mb: 1024,
        ..Default::default()
    };
    let support = ios.device_support(&phone).await.unwrap();
    assert_eq!(
        (support.status, support.verdict, support.modal),
        (SupportStatus::Supported, ReportVerdict::BelowMinspec, None)
    );
    let (desktop, _) = setup(FakeFetch::default(), Platform::Desktop);
    let support = desktop.device_support(&phone).await.unwrap();
    assert_eq!(support.verdict, ReportVerdict::NotChecked);
    assert!(fetch.gets().is_empty());
}

fn campaigns_body() -> serde_json::Value {
    json!({"ok": true, "data": {"campaigns": {
        "summer-26": {"target": {"type": "genesis", "position": "-9,-9"}},
        "arena": {"target": {"type": "world", "name": "arena.dcl.eth"}},
    }}})
}

#[tokio::test]
async fn a_captured_token_resolves_to_its_target_and_is_spent_once() {
    let url = backend_urls::campaigns();
    let (gates, fetch) = setup(
        FakeFetch::default().get(&url, Ok(campaigns_body())),
        Platform::Android,
    );

    assert_eq!(
        gates.campaign_resolution().await.fallback,
        Some(CampaignFallback::NoToken)
    );
    assert!(fetch.gets().is_empty(), "no token, no fetch");

    assert!(!gates.capture_campaign_token("Not A Token").unwrap());
    assert!(gates.capture_campaign_token(" summer-26 ").unwrap());
    assert!(
        !gates.capture_campaign_token("arena").unwrap(),
        "the first token stays"
    );

    let resolved = gates.campaign_resolution().await;
    let campaign = resolved.campaign.unwrap();
    assert_eq!(campaign.kind, CampaignKind::Genesis);
    assert_eq!(campaign.destination.parcel, Some(Parcel::new(-9, -9)));
    assert_eq!(resolved.fallback, None);

    gates.consume_campaign().unwrap();
    gates.consume_campaign().unwrap();
    assert_eq!(
        gates.campaign_resolution().await.fallback,
        Some(CampaignFallback::AlreadyConsumed)
    );
    assert_eq!(fetch.gets().len(), 1);
}

#[tokio::test]
async fn an_old_token_is_expired_and_an_offline_fetch_is_unavailable() {
    let store = StateStore::in_memory();
    store
        .update(|_| GatesState {
            campaign_token: "summer-26".into(),
            campaign_token_captured_at: NOW - 8 * DAY,
            ..Default::default()
        })
        .unwrap();
    let (gates, _) = setup_with(FakeFetch::default(), Platform::Android, store);
    assert_eq!(
        gates.campaign_resolution().await.fallback,
        Some(CampaignFallback::ExpiredToken)
    );

    let (offline, _) = setup(
        FakeFetch::with_get(Err("offline".into())),
        Platform::Android,
    );
    offline.capture_campaign_token("summer-26").unwrap();
    assert_eq!(
        offline.campaign_resolution().await.fallback,
        Some(CampaignFallback::ResolverUnavailable)
    );
}

#[tokio::test]
async fn the_state_survives_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(state::FILE);
    let (first, _) = setup_with(
        FakeFetch::default(),
        Platform::Android,
        StateStore::open(path.clone()),
    );
    first.device_support(&facts(1024)).await.unwrap();
    first
        .device_support_modal_shown(SupportModal::EndOfSupport)
        .unwrap();
    first.capture_campaign_token("summer-26").unwrap();

    let (second, _) = setup_with(
        FakeFetch::default(),
        Platform::Android,
        StateStore::open(path),
    );
    let state = second.store.get();
    assert_eq!(state.end_of_device_support_first_detected_unix, NOW);
    assert_eq!(state.end_of_device_support_modal_shown_count, 1);
    assert_eq!(state.campaign_token, "summer-26");
}
