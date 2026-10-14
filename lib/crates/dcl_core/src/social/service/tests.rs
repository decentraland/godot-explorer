use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use super::*;
use crate::social::types::PublicProfile;

fn friend(address: &str) -> Friend {
    Friend {
        address: address.into(),
        name: format!("name-{address}"),
        ..Friend::default()
    }
}

#[derive(Default)]
struct FakeApi {
    calls: Mutex<Vec<String>>,
    fail: Mutex<Option<SocialError>>,
    streams: Mutex<Vec<UnboundedReceiver<()>>>,
    subscribes: AtomicUsize,
}

impl FakeApi {
    fn record(&self, call: String) -> Result<(), SocialError> {
        self.calls.lock().unwrap().push(call);
        match self.fail.lock().unwrap().clone() {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }
    fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
    /// Queues a stream for the next `subscribe` and returns its sender.
    fn queue_stream(&self) -> UnboundedSender<()> {
        let (tx, rx) = unbounded_channel();
        self.streams.lock().unwrap().push(rx);
        tx
    }
}

#[async_trait]
impl SocialApi for FakeApi {
    async fn friends(&self, limit: u32, offset: u32) -> Result<Vec<Friend>, SocialError> {
        self.record(format!("friends {limit} {offset}"))?;
        Ok(vec![friend("0xa")])
    }
    async fn received_requests(&self, limit: u32, offset: u32) -> Result<Vec<Friend>, SocialError> {
        self.record(format!("received {limit} {offset}"))?;
        Ok(vec![friend("0xr")])
    }
    async fn sent_requests(&self, limit: u32, offset: u32) -> Result<Vec<Friend>, SocialError> {
        self.record(format!("sent {limit} {offset}"))?;
        Ok(vec![friend("0xs")])
    }
    async fn send_request(&self, a: &str, m: Option<String>) -> Result<(), SocialError> {
        self.record(format!("send {a} {m:?}"))
    }
    async fn accept(&self, a: &str) -> Result<(), SocialError> {
        self.record(format!("accept {a}"))
    }
    async fn reject(&self, a: &str) -> Result<(), SocialError> {
        self.record(format!("reject {a}"))
    }
    async fn cancel(&self, a: &str) -> Result<(), SocialError> {
        self.record(format!("cancel {a}"))
    }
    async fn block(&self, a: &str) -> Result<(), SocialError> {
        self.record(format!("block {a}"))
    }
    async fn unblock(&self, a: &str) -> Result<(), SocialError> {
        self.record(format!("unblock {a}"))
    }
    async fn subscribe(&self) -> Result<UnboundedReceiver<()>, SocialError> {
        self.subscribes.fetch_add(1, Ordering::SeqCst);
        if self.streams.lock().unwrap().is_empty() {
            return Err(SocialError::request("no stream"));
        }
        Ok(self.streams.lock().unwrap().remove(0))
    }
}

struct FakeProfiles;

#[async_trait]
impl ProfileSource for FakeProfiles {
    async fn public_profile(&self, address: &str) -> Option<PublicProfile> {
        (address != "0xmissing").then(|| PublicProfile {
            name: format!("p-{address}"),
            has_claimed_name: true,
            face_url: format!("face-{address}"),
        })
    }
}

struct FakeNearby {
    addresses: Vec<String>,
    blocked: Vec<String>,
    changes: std::sync::Mutex<Vec<(String, bool)>>,
}

impl NearbyProvider for FakeNearby {
    fn nearby_addresses(&self) -> Vec<String> {
        self.addresses.clone()
    }
    fn is_blocked(&self, address: &str) -> bool {
        self.blocked.iter().any(|b| b == address)
    }

    fn blocked_changed(&self, address: &str, blocked: bool) {
        self.changes
            .lock()
            .unwrap()
            .push((address.to_string(), blocked));
    }
}

fn service_with(api: Option<Arc<FakeApi>>) -> (Friends, Arc<AtomicUsize>) {
    let created = Arc::new(AtomicUsize::new(0));
    let counter = created.clone();
    let friends = Friends::new(
        Box::new(move || {
            counter.fetch_add(1, Ordering::SeqCst);
            api.clone().map(|a| a as Arc<dyn SocialApi>)
        }),
        Arc::new(FakeProfiles),
        Handle::current(),
    )
    .with_resubscribe_delay(Duration::from_millis(5));
    (friends, created)
}

async fn settle() {
    for _ in 0..20 {
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

#[tokio::test]
async fn signed_out_calls_fail_with_not_signed_in() {
    let (friends, _) = service_with(None);
    assert_eq!(friends.friends(100, 0).await, Err(SocialError::NotSignedIn));
    assert_eq!(friends.accept("0xa").await, Err(SocialError::NotSignedIn));
    assert_eq!(friends.start(), Err(SocialError::NotSignedIn));
}

#[tokio::test]
async fn the_service_is_created_once_and_lazily() {
    let (friends, created) = service_with(Some(Arc::new(FakeApi::default())));
    assert_eq!(created.load(Ordering::SeqCst), 0);
    friends.friends(10, 0).await.unwrap();
    friends.friends(10, 10).await.unwrap();
    assert_eq!(created.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn friends_pass_the_page_through() {
    let api = Arc::new(FakeApi::default());
    let (friends, _) = service_with(Some(api.clone()));
    let list = friends.friends(100, 200).await.unwrap();
    assert_eq!(list, vec![friend("0xa")]);
    assert_eq!(api.calls(), vec!["friends 100 200"]);
}

#[tokio::test]
async fn requests_fetch_both_directions_in_one_page_each() {
    let api = Arc::new(FakeApi::default());
    let (friends, _) = service_with(Some(api.clone()));
    let requests = friends.friend_requests().await.unwrap();
    assert_eq!(requests.received, vec![friend("0xr")]);
    assert_eq!(requests.sent, vec![friend("0xs")]);
    assert_eq!(api.calls(), vec!["received 100 0", "sent 100 0"]);
}

#[tokio::test]
async fn a_failing_received_list_stops_before_sent() {
    let api = Arc::new(FakeApi::default());
    *api.fail.lock().unwrap() = Some(SocialError::request("down"));
    let (friends, _) = service_with(Some(api.clone()));
    assert_eq!(
        friends.friend_requests().await,
        Err(SocialError::request("down"))
    );
    assert_eq!(api.calls(), vec!["received 100 0"]);
}

#[tokio::test]
async fn actions_reach_the_api_and_an_empty_message_is_none() {
    let api = Arc::new(FakeApi::default());
    let (friends, _) = service_with(Some(api.clone()));
    friends.send_request("0xa", "").await.unwrap();
    friends.send_request("0xb", "hi").await.unwrap();
    friends.accept("0xc").await.unwrap();
    friends.reject("0xd").await.unwrap();
    friends.cancel("0xe").await.unwrap();
    friends.block("0xf").await.unwrap();
    friends.unblock("0x1").await.unwrap();
    assert_eq!(
        api.calls(),
        vec![
            "send 0xa None",
            "send 0xb Some(\"hi\")",
            "accept 0xc",
            "reject 0xd",
            "cancel 0xe",
            "block 0xf",
            "unblock 0x1",
        ]
    );
}

#[tokio::test]
async fn api_errors_propagate() {
    let api = Arc::new(FakeApi::default());
    *api.fail.lock().unwrap() = Some(SocialError::request("refused"));
    let (friends, _) = service_with(Some(api));
    assert_eq!(
        friends.block("0xa").await,
        Err(SocialError::request("refused"))
    );
}

#[tokio::test]
async fn reset_drops_the_service_and_the_next_call_recreates_it() {
    let api = Arc::new(FakeApi::default());
    let (friends, created) = service_with(Some(api));
    friends.friends(1, 0).await.unwrap();
    friends.reset();
    friends.friends(1, 0).await.unwrap();
    assert_eq!(created.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn after_reset_a_signed_out_session_fails() {
    let api = Arc::new(FakeApi::default());
    let signed_in = Arc::new(Mutex::new(true));
    let flag = signed_in.clone();
    let friends = Friends::new(
        Box::new(move || {
            flag.lock()
                .unwrap()
                .then(|| api.clone() as Arc<dyn SocialApi>)
        }),
        Arc::new(FakeProfiles),
        Handle::current(),
    );
    friends.friends(1, 0).await.unwrap();
    *signed_in.lock().unwrap() = false;
    friends.reset();
    assert_eq!(friends.friends(1, 0).await, Err(SocialError::NotSignedIn));
}

#[tokio::test]
async fn updates_notify_every_listener_until_removed() {
    let api = Arc::new(FakeApi::default());
    let stream = api.queue_stream();
    let (friends, _) = service_with(Some(api));
    let hits = Arc::new(AtomicUsize::new(0));
    let counter = hits.clone();
    let id = friends.add_friends_changed_listener(Arc::new(move || {
        counter.fetch_add(1, Ordering::SeqCst);
    }));
    friends.start().unwrap();
    stream.send(()).unwrap();
    stream.send(()).unwrap();
    settle().await;
    assert_eq!(hits.load(Ordering::SeqCst), 2);
    friends.remove_friends_changed_listener(id);
    stream.send(()).unwrap();
    settle().await;
    assert_eq!(hits.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn a_dropped_stream_resubscribes_and_notifies_once() {
    let api = Arc::new(FakeApi::default());
    let first = api.queue_stream();
    let second = api.queue_stream();
    let (friends, _) = service_with(Some(api.clone()));
    let hits = Arc::new(AtomicUsize::new(0));
    let counter = hits.clone();
    friends.add_friends_changed_listener(Arc::new(move || {
        counter.fetch_add(1, Ordering::SeqCst);
    }));
    friends.start().unwrap();
    settle().await;
    assert_eq!(hits.load(Ordering::SeqCst), 0);
    drop(first);
    settle().await;
    assert_eq!(api.subscribes.load(Ordering::SeqCst), 2);
    assert_eq!(hits.load(Ordering::SeqCst), 1);
    second.send(()).unwrap();
    settle().await;
    assert_eq!(hits.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn a_failing_subscribe_is_retried() {
    let api = Arc::new(FakeApi::default());
    let (friends, _) = service_with(Some(api.clone()));
    friends.start().unwrap();
    settle().await;
    assert!(api.subscribes.load(Ordering::SeqCst) >= 2);
}

#[tokio::test]
async fn reset_stops_the_subscription() {
    let api = Arc::new(FakeApi::default());
    let stream = api.queue_stream();
    let (friends, _) = service_with(Some(api.clone()));
    let hits = Arc::new(AtomicUsize::new(0));
    let counter = hits.clone();
    friends.add_friends_changed_listener(Arc::new(move || {
        counter.fetch_add(1, Ordering::SeqCst);
    }));
    friends.start().unwrap();
    settle().await;
    friends.reset();
    settle().await;
    let subscribes = api.subscribes.load(Ordering::SeqCst);
    let _ = stream.send(());
    settle().await;
    assert_eq!(hits.load(Ordering::SeqCst), 0);
    assert_eq!(api.subscribes.load(Ordering::SeqCst), subscribes);
}

fn nearby_friends(addresses: Vec<String>, blocked: Vec<String>) -> Friends {
    let (friends, _) = service_with(None);
    friends.set_nearby_provider(Arc::new(FakeNearby {
        addresses,
        blocked,
        changes: Default::default(),
    }));
    friends
}

#[tokio::test]
async fn nearby_without_a_provider_is_empty() {
    let (friends, _) = service_with(None);
    assert!(friends.nearby().await.is_empty());
}

#[tokio::test]
async fn nearby_resolves_profiles_in_order_and_falls_back_to_the_address() {
    let friends = nearby_friends(vec!["0xa".into(), "0xmissing".into()], vec![]);
    let users = friends.nearby().await;
    assert_eq!(
        users,
        vec![
            NearbyUser {
                address: "0xa".into(),
                name: "p-0xa".into(),
                has_claimed_name: true,
                profile_picture_url: "face-0xa".into(),
            },
            NearbyUser {
                address: "0xmissing".into(),
                name: "0xmissing".into(),
                has_claimed_name: false,
                profile_picture_url: String::new(),
            },
        ]
    );
}

#[tokio::test]
async fn nearby_skips_blocked_and_empty_addresses() {
    let friends = nearby_friends(
        vec!["0xa".into(), String::new(), "0xb".into(), "0xc".into()],
        vec!["0xb".into()],
    );
    let addresses: Vec<String> = friends
        .nearby()
        .await
        .into_iter()
        .map(|u| u.address)
        .collect();
    assert_eq!(addresses, vec!["0xa", "0xc"]);
}

#[tokio::test]
async fn nearby_caps_after_filtering_blocked() {
    let addresses: Vec<String> = (0..50).map(|i| format!("0x{i}")).collect();
    let friends = nearby_friends(addresses, vec!["0x0".into(), "0x1".into()]);
    let users = friends.nearby().await;
    assert_eq!(users.len(), NEARBY_LIMIT);
    assert_eq!(users[0].address, "0x2");
}

#[tokio::test]
async fn a_block_or_unblock_the_server_accepted_is_told_to_the_engine() {
    let api = Arc::new(FakeApi::default());
    let (friends, _) = service_with(Some(api.clone()));
    let provider = Arc::new(FakeNearby {
        addresses: vec![],
        blocked: vec![],
        changes: Default::default(),
    });
    friends.set_nearby_provider(provider.clone());

    friends.block("0xa").await.unwrap();
    friends.unblock("0xb").await.unwrap();
    assert_eq!(
        *provider.changes.lock().unwrap(),
        vec![("0xa".to_string(), true), ("0xb".to_string(), false)]
    );

    *api.fail.lock().unwrap() = Some(SocialError::request("refused"));
    assert!(friends.block("0xc").await.is_err());
    assert_eq!(
        provider.changes.lock().unwrap().len(),
        2,
        "a refused block is not told"
    );
}
