//! End-to-end test of the social client against a running social-service, with two fresh
//! identities. Ignored by default because it needs the service:
//!
//! ```sh
//! SOCIAL_E2E_API_URL=http://127.0.0.1:5001 SOCIAL_E2E_STREAM_URL=ws://127.0.0.1:8085/v1/stream \
//!   cargo test --lib social::e2e_tests -- --ignored --nocapture
//! ```

use std::sync::Arc;
use std::time::Duration;

use ethers_signers::LocalWallet;
use rand::thread_rng;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::auth::auth_identity::create_local_ephemeral;
use crate::social::social_service_manager::{ConnectionState, SocialServiceManager};
use crate::social::types::FriendshipStreamEvent;

const EVENT_TIMEOUT: Duration = Duration::from_secs(10);

// FriendshipStatus codes the UI uses.
const REQUEST_SENT: i32 = 0;
const REQUEST_RECEIVED: i32 = 1;
const ACCEPTED: i32 = 3;

fn user() -> (SocialServiceManager, String) {
    let wallet = create_local_ephemeral(&LocalWallet::new(&mut thread_rng()));
    let address = format!("{:#x}", wallet.signer());
    let manager = SocialServiceManager::with_urls(
        Arc::new(wallet),
        std::env::var("SOCIAL_E2E_API_URL").expect("SOCIAL_E2E_API_URL"),
        std::env::var("SOCIAL_E2E_STREAM_URL").expect("SOCIAL_E2E_STREAM_URL"),
    );
    (manager, address)
}

/// Waits for the first event matching `predicate`, skipping the others.
async fn next_matching<T: std::fmt::Debug>(
    receiver: &mut UnboundedReceiver<T>,
    predicate: impl Fn(&T) -> bool,
) -> T {
    tokio::time::timeout(EVENT_TIMEOUT, async {
        loop {
            let event = receiver.recv().await.expect("the stream ended");
            if predicate(&event) {
                return event;
            }
        }
    })
    .await
    .expect("timed out waiting for an event")
}

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn friends_and_blocks_work_over_http_and_the_stream() {
    let (alice, alice_address) = user();
    let (bob, bob_address) = user();

    assert_eq!(bob.connection_state(), ConnectionState::Disconnected);
    let mut alice_updates = alice.subscribe_to_friendship_updates().await.unwrap();
    let mut bob_updates = bob.subscribe_to_friendship_updates().await.unwrap();
    assert_eq!(bob.connection_state(), ConnectionState::Connected);

    // Request with a message: Bob sees it live and in his received requests.
    let request = alice
        .send_friend_request(&bob_address, Some("hi bob".to_string()))
        .await
        .unwrap();
    assert_eq!(request.address, bob_address);

    match next_matching(&mut bob_updates, |_| true).await {
        FriendshipStreamEvent::Request {
            address, message, ..
        } => {
            assert_eq!(address, alice_address);
            assert_eq!(message.as_deref(), Some("hi bob"));
        }
        other => panic!("expected a request, got {other:?}"),
    }

    let received = bob.get_pending_friendship_requests(None).await.unwrap();
    assert_eq!(received.results.len(), 1);
    assert_eq!(received.results[0].friend.address, alice_address);
    assert_eq!(received.results[0].message.as_deref(), Some("hi bob"));

    let sent = alice.get_sent_friendship_requests(None).await.unwrap();
    assert_eq!(sent.results[0].friend.address, bob_address);

    let status = alice.get_friendship_status(&bob_address).await.unwrap();
    assert_eq!(status.status, REQUEST_SENT);
    let status = bob.get_friendship_status(&alice_address).await.unwrap();
    assert_eq!(status.status, REQUEST_RECEIVED);

    // Accept: Alice sees it live, both list each other as friends.
    bob.accept_friend_request(&alice_address).await.unwrap();
    next_matching(&mut alice_updates, |event| {
        matches!(event, FriendshipStreamEvent::Accept { address } if *address == bob_address)
    })
    .await;

    let friends = alice.get_friends(None).await.unwrap();
    assert_eq!(
        friends
            .results
            .iter()
            .map(|f| f.address.clone())
            .collect::<Vec<_>>(),
        vec![bob_address.clone()]
    );
    assert_eq!(friends.total, 1);
    let friends = bob.get_friends(None).await.unwrap();
    assert_eq!(friends.results[0].address, alice_address);
    assert_eq!(
        alice
            .get_friendship_status(&bob_address)
            .await
            .unwrap()
            .status,
        ACCEPTED
    );
    assert!(alice
        .get_mutual_friends(&bob_address, None)
        .await
        .unwrap()
        .results
        .is_empty());

    // The connectivity stream subscribes on the same connection.
    let _connectivity = bob
        .subscribe_to_friend_connectivity_updates()
        .await
        .unwrap();

    // Subscribing again hands the live subscription to the new receiver: the old one ends.
    let mut bob_updates_again = bob.subscribe_to_friendship_updates().await.unwrap();
    assert!(
        tokio::time::timeout(EVENT_TIMEOUT, bob_updates.recv())
            .await
            .unwrap()
            .is_none(),
        "the replaced receiver should end"
    );

    // Delete: Bob sees it on the new receiver.
    alice.delete_friendship(&bob_address).await.unwrap();
    next_matching(&mut bob_updates_again, |event| {
        matches!(event, FriendshipStreamEvent::Delete { address } if *address == alice_address)
    })
    .await;
    assert!(alice.get_friends(None).await.unwrap().results.is_empty());

    // Reject and cancel reach the other side too.
    alice.send_friend_request(&bob_address, None).await.unwrap();
    bob.reject_friend_request(&alice_address).await.unwrap();
    next_matching(&mut alice_updates, |event| {
        matches!(event, FriendshipStreamEvent::Reject { address } if *address == bob_address)
    })
    .await;

    alice.send_friend_request(&bob_address, None).await.unwrap();
    alice.cancel_friend_request(&bob_address).await.unwrap();
    next_matching(&mut bob_updates_again, |event| {
        matches!(event, FriendshipStreamEvent::Cancel { address } if *address == alice_address)
    })
    .await;

    // Blocks: Bob learns live that Alice blocked and then unblocked him.
    let mut bob_blocks = bob.subscribe_to_block_updates().await.unwrap();
    alice.block_user(&bob_address).await.unwrap();
    let block = next_matching(&mut bob_blocks, |_| true).await;
    assert_eq!(block.address, alice_address);
    assert!(block.is_blocked);

    let blocked = alice.get_blocked_users(None).await.unwrap();
    assert_eq!(blocked.results[0].profile.address, bob_address);
    assert!(blocked.results[0].blocked_at.is_some());
    assert_eq!(
        alice.get_blocking_status().await.unwrap().blocked_users,
        vec![bob_address.clone()]
    );
    assert_eq!(
        bob.get_blocking_status().await.unwrap().blocked_by_users,
        vec![alice_address.clone()]
    );

    // A blocked user cannot send a request: the HTTP status reaches the caller.
    let error = bob
        .send_friend_request(&alice_address, None)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("403"), "{error}");

    alice.unblock_user(&bob_address).await.unwrap();
    let unblock = next_matching(&mut bob_blocks, |_| true).await;
    assert!(!unblock.is_blocked);
    assert!(alice
        .get_blocking_status()
        .await
        .unwrap()
        .blocked_users
        .is_empty());

    // Invalid requests come back as errors with the status.
    let error = alice
        .send_friend_request(&alice_address, None)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("400"), "{error}");

    // Dropping the client closes the stream: once the queued events are read, it ends.
    drop(bob);
    tokio::time::timeout(EVENT_TIMEOUT, async {
        while bob_updates_again.recv().await.is_some() {}
    })
    .await
    .expect("dropping the client should end its subscriptions");
}
