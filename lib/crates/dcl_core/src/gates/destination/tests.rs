use serde_json::{json, Value};

use super::{resolve::*, *};
use crate::{
    gates::{fetch::SignedFetchError, test_support::FakeFetch},
    realm, urls,
};

const PEER: &str = "https://peer.example.com/content";
const ADDRESS: &str = "0xabc";

fn main_about_url() -> String {
    format!("{}about", realm::normalize_realm_url(&urls::main_realm()))
}

fn about() -> Value {
    json!({
        "content": {"publicUrl": PEER},
        "comms": {"adapter": "fixed-adapter:signed-login:https://comms.example.com/handshake"},
    })
}

fn entity(id: &str, title: &str) -> Value {
    json!({
        "id": id,
        "content": [],
        "metadata": {"display": {"title": title}},
    })
}

async fn go(fetch: &FakeFetch, input: &str) -> ResolvedDestination {
    resolve(fetch, ADDRESS, input, None).await
}

fn genesis_fetch(scene: Value) -> FakeFetch {
    FakeFetch::default()
        .get(&main_about_url(), Ok(about()))
        .post(&format!("{PEER}/entities/active"), Ok(scene))
}

#[tokio::test]
async fn coordinates_resolve_against_the_default_realm_and_read_the_scene() {
    let fetch = genesis_fetch(json!([entity("bafk1", "Genesis Plaza")]));
    let resolved = go(&fetch, "3,-4").await;

    assert!(resolved.ready);
    assert_eq!(resolved.kind, DestinationKind::Genesis);
    assert_eq!(resolved.title, "Genesis Plaza");
    assert_eq!(resolved.scene_id, "bafk1");
    assert_eq!(
        resolved.destination,
        Destination {
            realm: None,
            parcel: Some(Parcel::new(3, -4))
        }
    );
    assert_eq!(
        fetch.posts(),
        vec![(
            format!("{PEER}/entities/active"),
            r#"{"pointers":["3,-4"]}"#.to_string()
        )]
    );
}

#[tokio::test]
async fn an_empty_or_out_of_bounds_parcel_is_refused() {
    let empty = go(&genesis_fetch(json!([])), "3,3").await;
    assert_eq!(empty.failure, Some(ResolveFailure::ParcelEmpty));
    assert!(!empty.ready);

    let far = go(&genesis_fetch(json!([entity("bafk1", "x")])), "9999,0").await;
    assert_eq!(far.failure, Some(ResolveFailure::ParcelOutOfBounds));
}

#[tokio::test]
async fn a_realm_that_does_not_describe_itself_is_a_fetch_failure() {
    let offline = FakeFetch::default();
    assert_eq!(
        go(&offline, "0,0").await.failure,
        Some(ResolveFailure::FetchFailed)
    );
    let no_content = FakeFetch::default().get(&main_about_url(), Ok(json!({"content": {}})));
    assert_eq!(
        go(&no_content, "0,0").await.failure,
        Some(ResolveFailure::FetchFailed)
    );
}

#[tokio::test]
async fn a_custom_realm_is_not_gated_on_bounds_and_names_itself() {
    let fetch = FakeFetch::default()
        .get("https://realm.example.com/about", Ok(about()))
        .post(&format!("{PEER}/entities/active"), Ok(json!([])));
    let resolved = go(&fetch, "https://realm.example.com 900,900").await;
    assert_eq!(resolved.kind, DestinationKind::CustomRealm);
    assert!(
        resolved.ready,
        "an empty parcel in a custom realm lands on grass"
    );
    assert_eq!(
        resolved.destination.realm.as_deref(),
        Some("https://realm.example.com")
    );
}

fn world_base() -> String {
    realm::dcl_world_url("arena.dcl.eth")
}

fn world_fetch(permissions: Value) -> FakeFetch {
    FakeFetch::default()
        .get(&format!("{}/about", world_base()), Ok(about()))
        .get(&format!("{}/permissions", world_base()), Ok(permissions))
        .get(
            &format!("{}/scenes", world_base()),
            Ok(json!({"scenes": [
                {"parcels": ["0,0"], "entity": entity("bafk1", "Lobby")},
                {"parcels": ["1,0", "2,0"], "entity": entity("bafk2", "Arena")},
            ]})),
        )
}

#[tokio::test]
async fn an_open_world_lands_on_its_last_scene_or_the_one_at_the_parcel() {
    let fetch = world_fetch(json!({}));
    let spawn = go(&fetch, "Arena.dcl.eth").await;
    assert!(spawn.ready);
    assert_eq!(
        (spawn.kind, spawn.title.as_str()),
        (DestinationKind::World, "Arena")
    );
    assert_eq!(spawn.world_name, "arena.dcl.eth");
    assert_eq!(
        spawn.destination,
        Destination {
            realm: Some("arena.dcl.eth".into()),
            parcel: None
        }
    );

    let at = go(&fetch, "arena.dcl.eth 0,0").await;
    assert_eq!((at.ready, at.title.as_str()), (true, "Lobby"));
    assert_eq!(at.destination.parcel, Some(Parcel::new(0, 0)));

    let nothing = go(&fetch, "arena.dcl.eth 5,5").await;
    assert_eq!(nothing.failure, Some(ResolveFailure::ParcelEmpty));
}

#[tokio::test]
async fn a_world_url_is_the_same_world_as_its_name() {
    let fetch = world_fetch(json!({}));
    let by_url = go(&fetch, &format!("{}/", world_base())).await;
    assert_eq!(by_url.kind, DestinationKind::World);
    assert_eq!(by_url.world_name, "arena.dcl.eth");
}

#[tokio::test]
async fn a_world_that_does_not_answer_is_not_found() {
    let fetch = FakeFetch::default();
    assert_eq!(
        go(&fetch, "ghost.dcl.eth").await.failure,
        Some(ResolveFailure::WorldNotFound)
    );
}

#[tokio::test]
async fn an_allow_list_lets_listed_wallets_and_the_owner_in() {
    let listed = |wallets: Value, owner: &str| json!({"owner": owner, "permissions": {"access": {"type": "allow-list", "wallets": wallets}}});
    let denied = go(
        &world_fetch(listed(json!(["0xdef"]), "0x999")),
        "arena.dcl.eth",
    )
    .await;
    assert_eq!(denied.access, AccessVerdict::NotAllowed);
    assert!(!denied.ready && denied.failure.is_none());

    for permissions in [
        listed(json!(["0xABC"]), "0x999"),
        listed(json!([]), "0xAbC"),
        json!({"permissions": {"access": {"type": "allow-list", "communities": ["c1"]}}}),
    ] {
        assert!(go(&world_fetch(permissions), "arena.dcl.eth").await.ready);
    }
}

#[tokio::test]
async fn unlisted_access_kinds_fail_open() {
    for access in [
        json!({"type": "unrestricted"}),
        json!({"type": "nft-ownership", "nft": "x"}),
        json!({"type": "something-new"}),
        json!("odd"),
    ] {
        let permissions = json!({"permissions": {"access": access}});
        assert!(go(&world_fetch(permissions), "arena.dcl.eth").await.ready);
    }
    // An unreadable /permissions is an open world.
    let fetch = FakeFetch::default()
        .get(&format!("{}/about", world_base()), Ok(about()))
        .get(&format!("{}/scenes", world_base()), Ok(json!({})));
    assert!(go(&fetch, "arena.dcl.eth").await.ready);
}

fn secret_world() -> FakeFetch {
    world_fetch(json!({"permissions": {"access": {"type": "shared-secret"}}}))
}

#[tokio::test]
async fn a_password_world_asks_then_proves_the_secret_on_the_handshake() {
    let asked = go(&secret_world(), "arena.dcl.eth").await;
    assert_eq!(asked.access, AccessVerdict::NeedsPassword);
    assert!(!asked.ready);

    let fetch = secret_world().signed(Ok(()));
    let proven = resolve(&fetch, ADDRESS, "arena.dcl.eth", Some("hunter2")).await;
    assert!(proven.ready);
    assert_eq!(proven.title, "Arena");
    let calls = fetch.signed_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "https://comms.example.com/handshake");
    let meta: Value = serde_json::from_str(&calls[0].1).unwrap();
    assert_eq!(meta["secret"], "hunter2");
    assert_eq!(
        meta["origin"],
        "https://worlds-content-server.decentraland.org"
    );
}

#[tokio::test]
async fn a_rejected_secret_asks_again_unless_the_server_is_rate_limiting() {
    let wrong = secret_world().signed(Err(SignedFetchError::Rejected("InvalidAccessError".into())));
    let resolved = resolve(&wrong, ADDRESS, "arena.dcl.eth", Some("nope")).await;
    assert_eq!(resolved.access, AccessVerdict::NeedsPassword);
    assert_eq!(resolved.failure, None);

    let limited =
        secret_world().signed(Err(SignedFetchError::Rejected("Too many attempts".into())));
    let resolved = resolve(&limited, ADDRESS, "arena.dcl.eth", Some("nope")).await;
    assert_eq!(resolved.failure, Some(ResolveFailure::RateLimited));

    let signed_out = secret_world().signed(Err(SignedFetchError::NoIdentity));
    let resolved = resolve(&signed_out, "", "arena.dcl.eth", Some("x")).await;
    assert_eq!(resolved.failure, Some(ResolveFailure::FetchFailed));
}

#[tokio::test]
async fn a_world_with_nowhere_to_prove_the_secret_is_not_enterable() {
    let fetch = FakeFetch::default()
        .get(
            &format!("{}/about", world_base()),
            Ok(json!({"content": {"publicUrl": PEER}})),
        )
        .get(
            &format!("{}/permissions", world_base()),
            Ok(json!({"permissions": {"access": {"type": "shared-secret"}}})),
        );
    let resolved = resolve(&fetch, ADDRESS, "arena.dcl.eth", Some("x")).await;
    assert_eq!(resolved.failure, Some(ResolveFailure::FetchFailed));
    assert!(fetch.signed_calls().is_empty());
}

#[tokio::test]
async fn a_preview_names_its_scene_through_pointers_or_its_urn() {
    let url = "http://192.168.1.20:8000";
    let unreachable = go(&FakeFetch::default(), url).await;
    assert_eq!(
        unreachable.failure,
        Some(ResolveFailure::PreviewUnreachable)
    );
    assert_eq!(unreachable.kind, DestinationKind::Preview);

    let pointers = FakeFetch::default()
        .get(
            &format!("{url}/about"),
            Ok(json!({"content": {"publicUrl": "http://192.168.1.20:8000/content"}})),
        )
        .post(
            "http://192.168.1.20:8000/content/entities/active",
            Ok(json!([entity("bafkp", "Local")])),
        );
    let resolved = go(&pointers, &format!("{url} 1,1")).await;
    assert_eq!((resolved.ready, resolved.title.as_str()), (true, "Local"));

    let id = format!("ba{}", "a".repeat(57));
    let urn = format!(
        "urn:decentraland:entity:{id}?=&baseUrl=http://192.168.1.20:8000/content/contents/"
    );
    let by_urn = FakeFetch::default()
        .get(
            &format!("{url}/about"),
            Ok(json!({"configurations": {"scenesUrn": [urn]}})),
        )
        .get(
            &format!("http://192.168.1.20:8000/content/contents/{id}"),
            Ok(entity(&id, "From urn")),
        );
    let resolved = go(&by_urn, url).await;
    assert_eq!(
        (resolved.ready, resolved.title.as_str()),
        (true, "From urn")
    );

    let bare = FakeFetch::default().get(&format!("{url}/about"), Ok(json!({})));
    let resolved = go(&bare, url).await;
    assert!(resolved.ready && resolved.title.is_empty());
}

#[tokio::test]
async fn input_that_names_nothing_is_invalid() {
    let fetch = FakeFetch::default();
    for input in ["", "hello", "two words", "decentraland://events?id=1"] {
        assert_eq!(
            go(&fetch, input).await.failure,
            Some(ResolveFailure::InvalidInput),
            "{input:?}"
        );
    }
    assert!(fetch.gets().is_empty());
}

#[tokio::test(start_paused = true)]
async fn the_deadline_covers_the_whole_resolve() {
    let slow = go(&FakeFetch::hanging(), "0,0").await;
    assert_eq!(slow.failure, Some(ResolveFailure::Timeout));
    let preview = go(&FakeFetch::hanging(), "http://10.0.0.1:8000").await;
    assert_eq!(preview.failure, Some(ResolveFailure::Timeout));
}

#[test]
fn world_names_come_from_names_and_urls_only() {
    let base = urls::worlds_content_server();
    for (realm, expected) in [
        ("Arena.dcl.eth", "arena.dcl.eth"),
        (&format!("{base}Arena.dcl.eth/"), "arena.dcl.eth"),
        (&format!("{base}my%20world.dcl.eth"), "my world.dcl.eth"),
        ("https://realm.example.com", ""),
        ("http://192.168.1.2:8000", ""),
        (&urls::main_realm(), ""),
        ("arena.eth", ""),
    ] {
        assert_eq!(world_name_from_realm(realm), expected, "{realm}");
    }
}

#[test]
fn parcel_verdicts_check_bounds_before_existence() {
    let p = Parcel::new;
    let check = |parcel, id| genesis_failure(parcel, (-150, -150), (163, 158), id);
    assert_eq!(check(p(0, 0), "bafk"), None);
    assert_eq!(check(p(0, 0), ""), Some(ResolveFailure::ParcelEmpty));
    assert_eq!(
        check(p(164, 0), "bafk"),
        Some(ResolveFailure::ParcelOutOfBounds)
    );
    assert_eq!(
        check(p(0, -151), ""),
        Some(ResolveFailure::ParcelOutOfBounds)
    );
    assert_eq!(check(p(-150, 158), "bafk"), None);
}
