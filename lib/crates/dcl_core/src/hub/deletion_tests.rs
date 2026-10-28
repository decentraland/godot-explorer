use super::tests::{setup, signed_out, state, Setup};
use super::*;
use crate::{app::State, session::store::SessionSlot};

async fn signed_in_guest(upgraded: Option<bool>) -> Setup {
    let s = setup();
    signed_out(&s);
    s.hub.sign_in_guest().await.unwrap();
    *s.backend.upgraded.lock().unwrap() = upgraded;
    s
}

fn write_guest_files(s: &Setup) {
    for name in GUEST_DEVICE_FILES {
        std::fs::write(s.dir.path().join(name), "x").unwrap();
    }
}

fn leftovers(s: &Setup) -> Vec<&'static str> {
    GUEST_DEVICE_FILES
        .into_iter()
        .filter(|name| s.dir.path().join(name).exists())
        .collect()
}

#[tokio::test]
async fn a_plain_guest_is_deleted_on_the_device() {
    let s = signed_in_guest(Some(false)).await;
    assert_eq!(s.hub.deletion_plan().await, DeletionPlan::Guest);
}

#[tokio::test]
async fn an_upgraded_guest_goes_to_the_server_unless_a_dev_link_allows_deleting_it() {
    let s = signed_in_guest(Some(true)).await;
    assert_eq!(s.hub.deletion_plan().await, DeletionPlan::Server);

    s.hub.allow_upgraded_deletion();
    assert_eq!(
        s.hub.deletion_plan().await,
        DeletionPlan::Server,
        "production ignores the dev link"
    );

    s.hub.set_production(false);
    s.hub.allow_upgraded_deletion();
    assert_eq!(s.hub.deletion_plan().await, DeletionPlan::UpgradedGuest);
}

#[tokio::test]
async fn a_failed_upgrade_check_never_turns_a_recoverable_account_into_a_plain_guest() {
    let s = signed_in_guest(None).await;
    s.hub.with_session(|x| x.is_thirdweb_guest_upgraded = true);
    assert_eq!(s.hub.deletion_plan().await, DeletionPlan::Server);

    s.hub.with_session(|x| x.is_thirdweb_guest_upgraded = false);
    assert_eq!(
        s.hub.deletion_plan().await,
        DeletionPlan::Server,
        "a cold flag is not proof of a plain guest"
    );
}

#[tokio::test]
async fn an_account_that_is_not_a_guest_uses_the_server() {
    let s = setup();
    signed_out(&s);
    s.hub
        .with_session(|x| x.install_remote(Default::default(), 1, super::tests::chain().1));
    assert_eq!(s.hub.deletion_plan().await, DeletionPlan::Server);
}

#[tokio::test]
async fn deleting_a_guest_unlinks_wipes_the_identity_files_and_signs_out() {
    let s = signed_in_guest(Some(false)).await;
    write_guest_files(&s);
    SessionStore::new(s.dir.path())
        .set_slot(
            "",
            SessionSlot {
                account: "{a}".into(),
                guest_profile: "{look}".into(),
            },
        )
        .unwrap();

    s.hub.delete_guest(DeletionPlan::Guest).await.unwrap();

    assert_eq!(*s.backend.unlinks.lock().unwrap(), vec!["guest"]);
    assert!(leftovers(&s).is_empty());
    assert_eq!(state(&s), State::SignedOut);
    let slot = SessionStore::new(s.dir.path()).slot("").unwrap();
    assert!(slot.guest_profile.is_empty());
}

#[tokio::test]
async fn an_upgraded_guest_is_unlinked_everywhere() {
    let s = signed_in_guest(Some(true)).await;
    s.hub
        .delete_guest(DeletionPlan::UpgradedGuest)
        .await
        .unwrap();
    assert_eq!(*s.backend.unlinks.lock().unwrap(), vec!["upgraded"]);
    assert_eq!(state(&s), State::SignedOut);
}

#[tokio::test]
async fn a_server_deletion_is_not_a_guest_deletion() {
    let s = signed_in_guest(Some(true)).await;
    assert!(s.hub.delete_guest(DeletionPlan::Server).await.is_err());
    assert_eq!(state(&s), State::SignedIn);
}

#[tokio::test]
async fn a_throwaway_wallet_deletes_locally() {
    let s = setup();
    signed_out(&s);
    let wallet = ethers_signers::LocalWallet::new(&mut rand::thread_rng());
    let chain = crate::session::auth_identity::create_local_ephemeral(&wallet);
    s.hub
        .with_session(|x| x.install_local(wallet.signer().to_bytes().as_slice(), chain))
        .unwrap();
    assert_eq!(s.hub.deletion_plan().await, DeletionPlan::Local);
}
