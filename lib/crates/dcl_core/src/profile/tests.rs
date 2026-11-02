use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime},
};

use async_trait::async_trait;
use ethers_core::types::{Signature, H160};
use ethers_signers::LocalWallet;
use serde_json::{json, Value};

use super::*;
use crate::session::{
    ephemeral_auth_chain::EphemeralAuthChain,
    wallet::{ChainLink, SimpleAuthChain},
};

const ADDRESS: &str = "0x1111111111111111111111111111111111111111";

#[derive(Default)]
struct FakeTransport {
    replies: Mutex<VecDeque<Result<HttpReply, String>>>,
    sent: Mutex<Vec<HttpRequest>>,
}

impl FakeTransport {
    fn replying(replies: Vec<Result<HttpReply, String>>) -> Arc<Self> {
        Arc::new(Self {
            replies: Mutex::new(replies.into()),
            sent: Mutex::default(),
        })
    }

    fn sent(&self) -> Vec<HttpRequest> {
        self.sent.lock().unwrap().clone()
    }
}

#[async_trait]
impl Transport for FakeTransport {
    async fn send(&self, request: HttpRequest) -> Result<HttpReply, String> {
        self.sent.lock().unwrap().push(request);
        self.replies
            .lock()
            .unwrap()
            .pop_front()
            .expect("no reply queued")
    }
}

fn ok(body: Value) -> Result<HttpReply, String> {
    Ok(HttpReply {
        status: 200,
        body: body.to_string(),
    })
}

fn status(status: u16, body: &str) -> Result<HttpReply, String> {
    Ok(HttpReply {
        status,
        body: body.into(),
    })
}

struct FakeIdentity {
    address: Option<String>,
    chain: Option<EphemeralAuthChain>,
    local_guest: bool,
}

impl Identity for FakeIdentity {
    fn address(&self) -> Option<String> {
        self.address.clone()
    }
    fn auth_chain(&self) -> Option<EphemeralAuthChain> {
        self.chain.clone()
    }
    fn is_local_guest(&self) -> bool {
        self.local_guest
    }
}

fn chain() -> EphemeralAuthChain {
    let ephemeral = LocalWallet::new(&mut rand::thread_rng());
    let links = vec![
        ChainLink::new("SIGNER".into(), ADDRESS.into(), String::new()),
        ChainLink::new(
            "ECDSA_EPHEMERAL".into(),
            "ephemeral message".into(),
            "0xsig".into(),
        ),
    ];
    EphemeralAuthChain::new(
        ADDRESS.parse::<H160>().unwrap(),
        ephemeral.signer().to_bytes().to_vec(),
        SimpleAuthChain::from_chain_links(links),
        SystemTime::now() + Duration::from_secs(3600),
    )
}

fn identity(local_guest: bool) -> Arc<FakeIdentity> {
    Arc::new(FakeIdentity {
        address: Some(ADDRESS.into()),
        chain: Some(chain()),
        local_guest,
    })
}

fn service(transport: &Arc<FakeTransport>, identity: Arc<FakeIdentity>) -> ProfileService {
    ProfileService::new(
        transport.clone(),
        identity,
        Endpoints::new("https://peer.test/lambdas", "https://peer.test/content/"),
    )
}

fn catalyst_profile(version: i64) -> Value {
    json!({ "avatars": [{
        "userId": ADDRESS,
        "ethAddress": ADDRESS,
        "name": "Ana",
        "description": "hi",
        "version": version,
        "tutorialStep": 256,
        "hasClaimedName": true,
        "country": "AR",
        "links": [{ "title": "site", "url": "https://a.test" }],
        "blocked": ["0xbad"],
        "avatar": {
            "bodyShape": "urn:body",
            "wearables": ["urn:w1"],
            "emotes": [{ "slot": 0, "urn": "urn:e0" }, { "slot": 1, "urn": "" }],
            "snapshots": { "face256": "https://img.test/face.png", "body": "b" }
        }
    }]})
}

#[tokio::test]
async fn fetches_a_public_profile_from_the_lambdas() {
    let transport = FakeTransport::replying(vec![ok(catalyst_profile(7))]);
    let service = service(&transport, identity(false));
    let profile = service.profile(Some("0x2222")).await.unwrap();
    assert_eq!(
        transport.sent()[0].url,
        "https://peer.test/lambdas/profiles/0x2222"
    );
    assert_eq!(profile.name, "Ana");
    assert_eq!(profile.version, 7);
    assert!(profile.has_claimed_name);
    assert_eq!(profile.country, "AR");
    assert_eq!(profile.pronouns, "");
    assert_eq!(
        profile.links,
        vec![ProfileLink {
            title: "site".into(),
            url: "https://a.test".into()
        }]
    );
    assert_eq!(profile.profile_picture_url, "https://img.test/face.png");
}

#[tokio::test]
async fn own_address_reads_the_own_profile_and_caches_it() {
    let transport = FakeTransport::replying(vec![ok(catalyst_profile(3))]);
    let service = service(&transport, identity(false));
    let first = service
        .profile(Some(&ADDRESS.to_uppercase().replace("0X", "0x")))
        .await
        .unwrap();
    let second = service.profile(None).await.unwrap();
    assert_eq!(first, second);
    assert_eq!(transport.sent().len(), 1);
}

#[tokio::test]
async fn a_missing_own_profile_is_the_default_one() {
    let transport = FakeTransport::replying(vec![status(404, "{}")]);
    let service = service(&transport, identity(false));
    let profile = service.profile(None).await.unwrap();
    assert_eq!(profile.address, ADDRESS);
    assert_eq!(profile.version, 1);
    assert!(!profile.has_claimed_name);
}

#[tokio::test]
async fn a_missing_public_profile_is_not_found() {
    let transport = FakeTransport::replying(vec![ok(json!({ "avatars": [] }))]);
    let service = service(&transport, identity(false));
    assert_eq!(
        service.profile(Some("0x2222")).await,
        Err(ProfileError::NotFound)
    );
}

#[tokio::test]
async fn a_failing_catalyst_is_an_error_not_a_default_profile() {
    let transport = FakeTransport::replying(vec![status(503, "down")]);
    let service = service(&transport, identity(false));
    assert_eq!(
        service.profile(None).await,
        Err(ProfileError::Http {
            status: 503,
            message: "down".into()
        })
    );
}

#[tokio::test]
async fn a_local_guest_never_asks_the_catalyst() {
    let transport = FakeTransport::replying(vec![]);
    let service = service(&transport, identity(true));
    assert_eq!(service.profile(None).await.unwrap().address, ADDRESS);
    assert!(transport.sent().is_empty());
}

#[tokio::test]
async fn signed_out_has_no_own_profile() {
    let transport = FakeTransport::replying(vec![]);
    let signed_out = Arc::new(FakeIdentity {
        address: None,
        chain: None,
        local_guest: false,
    });
    let service = service(&transport, signed_out);
    assert_eq!(service.profile(None).await, Err(ProfileError::NotSignedIn));
}

#[tokio::test]
async fn names_are_read_page_by_page_and_cached() {
    let page = |names: &[&str], total: u64| {
        ok(json!({
            "elements": names.iter().map(|n| json!({ "name": n })).collect::<Vec<_>>(),
            "totalAmount": total,
        }))
    };
    let first: Vec<String> = (0..100).map(|i| format!("n{i}")).collect();
    let first_refs: Vec<&str> = first.iter().map(String::as_str).collect();
    let transport = FakeTransport::replying(vec![page(&first_refs, 101), page(&["last"], 101)]);
    let service = service(&transport, identity(false));
    let names = service.owned_names().await.unwrap();
    assert_eq!(names.len(), 101);
    assert_eq!(names[100], "last");
    let sent = transport.sent();
    assert_eq!(
        sent[0].url,
        format!("https://peer.test/lambdas/users/{ADDRESS}/names?pageNum=1&pageSize=100")
    );
    assert!(sent[1].url.ends_with("pageNum=2&pageSize=100"));
    service.owned_names().await.unwrap();
    assert_eq!(transport.sent().len(), 2);
}

#[tokio::test]
async fn names_stop_on_an_empty_page_and_fail_on_a_page_error() {
    let transport =
        FakeTransport::replying(vec![ok(json!({ "elements": [], "totalAmount": 500 }))]);
    assert!(service(&transport, identity(false))
        .owned_names()
        .await
        .unwrap()
        .is_empty());
    let transport = FakeTransport::replying(vec![Err("offline".into())]);
    assert_eq!(
        service(&transport, identity(false)).owned_names().await,
        Err(ProfileError::Network("offline".into()))
    );
}

fn field<'a>(body: &'a str, name: &str) -> &'a str {
    let marker = format!("name=\"{name}\"\r\n\r\n");
    let start = body
        .find(&marker)
        .unwrap_or_else(|| panic!("no part {name}"))
        + marker.len();
    &body[start..start + body[start..].find("\r\n--").unwrap()]
}

#[tokio::test]
async fn deploy_signs_and_posts_the_entity_with_the_version_bumped() {
    let transport = FakeTransport::replying(vec![
        ok(catalyst_profile(4)),
        ok(json!({ "creationTimestamp": 1 })),
    ]);
    let id = identity(false);
    let service = service(&transport, id.clone());
    let update = ProfileUpdate {
        name: Some("Bea".into()),
        country: Some(String::new()),
        links: Some(vec![]),
        ..ProfileUpdate::default()
    };
    let profile = service.deploy(&update).await.unwrap();
    assert_eq!((profile.name.as_str(), profile.version), ("Bea", 5));

    let post = &transport.sent()[1];
    assert_eq!(post.method, http::Method::POST);
    assert_eq!(post.url, "https://peer.test/content/entities/");
    let content_type = &post.headers[0].1;
    assert!(content_type.starts_with("multipart/form-data; boundary="));
    let body = String::from_utf8(post.body.clone()).unwrap();

    let entity_id = field(&body, "entityId");
    let entity = field(&body, entity_id);
    assert_eq!(entity_id, super::cid::entity_id(entity.as_bytes()));
    assert_eq!(field(&body, "authChain[0][type]"), "SIGNER");
    assert_eq!(field(&body, "authChain[1][type]"), "ECDSA_EPHEMERAL");
    assert_eq!(field(&body, "authChain[2][type]"), "ECDSA_SIGNED_ENTITY");
    assert_eq!(field(&body, "authChain[2][payload]"), entity_id);
    let signature: Signature = field(&body, "authChain[2][signature]").parse().unwrap();
    let signer = signature.recover(entity_id).unwrap();
    assert_eq!(
        signer,
        id.chain.as_ref().unwrap().ephemeral_wallet().address()
    );

    let entity: Value = serde_json::from_str(entity).unwrap();
    assert_eq!(entity["version"], "v3");
    assert_eq!(entity["type"], "profile");
    assert_eq!(entity["pointers"], json!([ADDRESS]));
    assert_eq!(entity["content"], json!([]));
    assert!(entity["timestamp"].as_u64().unwrap() > 0);
    let deployed = &entity["metadata"]["avatars"][0];
    assert_eq!(deployed["version"], 5);
    assert_eq!(deployed["name"], "Bea");
    assert_eq!(deployed["country"], Value::Null);
    assert_eq!(deployed["links"], Value::Null);
    assert_eq!(deployed["tutorialStep"], 256);
    assert_eq!(deployed["blocked"], json!(["0xbad"]));
    assert_eq!(deployed["avatar"]["snapshots"], Value::Null);
    assert_eq!(
        deployed["avatar"]["emotes"],
        json!([{ "slot": 0, "urn": "urn:e0" }])
    );
}

#[tokio::test]
async fn untouched_fields_survive_a_deploy_and_the_next_one_builds_on_it() {
    let transport = FakeTransport::replying(vec![
        ok(catalyst_profile(4)),
        ok(json!({ "creationTimestamp": 1 })),
        ok(json!({ "creationTimestamp": 2 })),
    ]);
    let service = service(&transport, identity(false));
    service
        .deploy(&ProfileUpdate {
            hobbies: Some("chess".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    let profile = service
        .deploy(&ProfileUpdate {
            has_claimed_name: Some(false),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(profile.version, 6);
    assert_eq!(profile.hobbies, "chess");
    assert_eq!(profile.name, "Ana");
    assert!(!profile.has_claimed_name);
}

#[tokio::test]
async fn a_rejected_deploy_leaves_the_profile_and_version_alone() {
    let transport = FakeTransport::replying(vec![
        ok(catalyst_profile(4)),
        status(400, r#"{"error":"Invalid pointer"}"#),
    ]);
    let service = service(&transport, identity(false));
    let update = ProfileUpdate {
        name: Some("Bea".into()),
        ..Default::default()
    };
    assert_eq!(
        service.deploy(&update).await,
        Err(ProfileError::Http {
            status: 400,
            message: "Invalid pointer".into()
        })
    );
    let profile = service.profile(None).await.unwrap();
    assert_eq!((profile.name.as_str(), profile.version), ("Ana", 4));
}

#[tokio::test]
async fn a_deploy_answer_without_creation_timestamp_is_rejected() {
    let transport = FakeTransport::replying(vec![ok(catalyst_profile(1)), ok(json!({}))]);
    let service = service(&transport, identity(false));
    let result = service.deploy(&ProfileUpdate::default()).await;
    assert!(matches!(result, Err(ProfileError::InvalidResponse(_))));
}

#[tokio::test]
async fn local_guest_and_disabled_deploys_stay_on_the_device() {
    let transport = FakeTransport::replying(vec![]);
    let guest = service(&transport, identity(true));
    let update = ProfileUpdate {
        name: Some("Guest".into()),
        ..Default::default()
    };
    let profile = guest.deploy(&update).await.unwrap();
    assert_eq!((profile.name.as_str(), profile.version), ("Guest", 2));
    assert!(transport.sent().is_empty());

    let transport = FakeTransport::replying(vec![ok(catalyst_profile(1))]);
    let disabled = service(&transport, identity(false));
    disabled.set_deploy_disabled(true);
    assert_eq!(disabled.deploy(&update).await.unwrap().version, 2);
    assert_eq!(transport.sent().len(), 1);
}

#[tokio::test]
async fn adopt_makes_the_engine_profile_the_base_of_the_next_deploy() {
    let transport = FakeTransport::replying(vec![ok(json!({ "creationTimestamp": 1 }))]);
    let service = service(&transport, identity(false));
    service
        .adopt(ProfileDocument::default_for(ADDRESS).with_version(9))
        .unwrap();
    let profile = service.deploy(&ProfileUpdate::default()).await.unwrap();
    assert_eq!(profile.version, 10);
    service.reset();
}

#[tokio::test]
async fn deletion_posts_a_signed_empty_metadata_request() {
    let transport = FakeTransport::replying(vec![ok(json!({ "ok": true }))]);
    let service = service(&transport, identity(false));
    service.request_account_deletion().await.unwrap();
    let request = &transport.sent()[0];
    assert_eq!(request.method, http::Method::POST);
    assert_eq!(request.url, crate::urls::account_deletion());
    assert!(request.body.is_empty());
    let header = |name: &str| {
        request
            .headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.clone())
            .unwrap()
    };
    assert_eq!(header("x-identity-metadata"), "{}");
    assert!(header("x-identity-timestamp").parse::<u64>().is_ok());
    let link: Value = serde_json::from_str(&header("x-identity-auth-chain-2")).unwrap();
    assert_eq!(link["type"], "ECDSA_SIGNED_ENTITY");
    assert!(link["payload"]
        .as_str()
        .unwrap()
        .starts_with("post:/deletion:"));
    assert!(link["payload"].as_str().unwrap().ends_with(":{}"));
}

#[tokio::test]
async fn deletion_failures_carry_the_servers_reason() {
    let refused = FakeTransport::replying(vec![ok(json!({ "ok": false, "error": "pending" }))]);
    assert_eq!(
        service(&refused, identity(false))
            .request_account_deletion()
            .await,
        Err(ProfileError::Http {
            status: 200,
            message: "pending".into()
        })
    );
    let garbage = FakeTransport::replying(vec![Ok(HttpReply {
        status: 200,
        body: "<html>".into(),
    })]);
    assert!(matches!(
        service(&garbage, identity(false))
            .request_account_deletion()
            .await,
        Err(ProfileError::InvalidResponse(_))
    ));
    let down = FakeTransport::replying(vec![status(500, r#"{"error":"boom"}"#)]);
    assert_eq!(
        service(&down, identity(false))
            .request_account_deletion()
            .await,
        Err(ProfileError::Http {
            status: 500,
            message: "boom".into()
        })
    );
}

#[test]
fn updates_name_only_what_they_carry() {
    let base = ProfileDocument::default_for(ADDRESS);
    let next = base.updated(&ProfileUpdate {
        links: Some(vec![ProfileLink {
            title: "t".into(),
            url: "u".into(),
        }]),
        pronouns: Some("they".into()),
        ..Default::default()
    });
    assert_eq!(next.profile().pronouns, "they");
    assert_eq!(next.profile().links.len(), 1);
    assert_eq!(base.profile().pronouns, "");
    assert_eq!(next.avatar(), base.avatar());
}
