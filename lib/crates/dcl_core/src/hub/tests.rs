use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use ethers_core::types::H160;
use ethers_signers::{LocalWallet, Signer};
use rand::thread_rng;

use super::*;
use crate::{
    app::{AppEvents, SignInMethod, SignInStep, State},
    session::{
        auth_identity::create_local_ephemeral, ephemeral_auth_chain::EphemeralAuthChain,
        store::SessionStore, thirdweb_flows::GuestLoginOutcome, thirdweb_guest::GuestLoginError,
    },
    OpenUrlRequest,
};

fn chain() -> (H160, EphemeralAuthChain) {
    let wallet = LocalWallet::new(&mut thread_rng());
    (wallet.address(), create_local_ephemeral(&wallet))
}

#[derive(Default)]
struct Fake {
    fail_guest: Mutex<Option<&'static str>>,
    fail_email_send: Mutex<bool>,
    good_code: Mutex<String>,
    browser_fails: Mutex<bool>,
    /// Holds `browser_identity` until released, to cancel while it is in flight.
    browser_gate: Mutex<Option<Arc<tokio::sync::Notify>>>,
    browser_entered: Arc<tokio::sync::Notify>,
}

#[async_trait]
impl SignInBackend for Fake {
    async fn guest(&self, _anchor: String) -> Result<GuestLoginOutcome, GuestLoginError> {
        if let Some(reason) = *self.fail_guest.lock().unwrap() {
            return Err(GuestLoginError {
                reason,
                http_status: Some(503),
                message: "down".into(),
            });
        }
        let (address, chain) = chain();
        Ok(GuestLoginOutcome {
            address,
            chain,
            is_new_user: true,
        })
    }

    async fn email_send(&self, _email: &str) -> Result<(), anyhow::Error> {
        if *self.fail_email_send.lock().unwrap() {
            anyhow::bail!("no mailbox");
        }
        Ok(())
    }

    async fn email_verify(
        &self,
        _email: &str,
        code: &str,
    ) -> Result<(H160, EphemeralAuthChain), anyhow::Error> {
        if code == *self.good_code.lock().unwrap() {
            Ok(chain())
        } else {
            anyhow::bail!("wrong code")
        }
    }

    async fn browser_identity(
        &self,
        _identity_id: String,
    ) -> Result<(EphemeralAuthChain, u64), anyhow::Error> {
        self.browser_entered.notify_one();
        let gate = self.browser_gate.lock().unwrap().clone();
        if let Some(gate) = gate {
            gate.notified().await;
        }
        if *self.browser_fails.lock().unwrap() {
            anyhow::bail!("expired");
        }
        Ok((chain().1, 1))
    }
}

#[derive(Default)]
struct Heard {
    states: Mutex<Vec<State>>,
    errors: Mutex<Vec<String>>,
    urls: Mutex<Vec<String>>,
    sessions: Mutex<Vec<SessionEvent>>,
    guest_reports: Mutex<Vec<GuestLoginReport>>,
    reports: Mutex<Vec<crate::hub::ReportPayload>>,
}

impl AppEvents for Heard {
    fn state_changed(&self, state: &State) {
        self.states.lock().unwrap().push(state.clone());
    }
    fn engine_required(&self) {}
    fn avatar_preview_mode(&self, _on: bool) {}
    fn scene_crashed(&self) {}
    fn auth_error(&self, code: &str) {
        self.errors.lock().unwrap().push(code.to_string());
    }
    fn open_url(&self, request: &OpenUrlRequest) {
        self.urls.lock().unwrap().push(request.url.clone());
    }
    fn report(&self, payload: &crate::hub::ReportPayload) {
        self.reports.lock().unwrap().push(payload.clone());
    }
}

impl SessionObserver for Heard {
    fn session_event(&self, event: &SessionEvent) {
        self.sessions.lock().unwrap().push(event.clone());
    }
    fn guest_login(&self, report: &GuestLoginReport) {
        self.guest_reports.lock().unwrap().push(report.clone());
    }
}

struct Setup {
    hub: Hub,
    backend: Arc<Fake>,
    heard: Arc<Heard>,
    dir: tempfile::TempDir,
}

fn setup() -> Setup {
    let dir = tempfile::tempdir().unwrap();
    let backend = Arc::new(Fake::default());
    let hub = Hub::new(dir.path(), backend.clone());
    let heard = Arc::new(Heard::default());
    hub.events().add_listener(heard.clone());
    hub.add_observer(heard.clone());
    Setup {
        hub,
        backend,
        heard,
        dir,
    }
}

fn signed_out(s: &Setup) {
    s.hub.boot();
    assert_eq!(s.hub.with_app(|a| a.state().clone()), State::SignedOut);
}

fn saved_account(s: &Setup) -> String {
    SessionStore::new(s.dir.path())
        .slot("")
        .map(|slot| slot.account)
        .unwrap_or_default()
}

fn state(s: &Setup) -> State {
    s.hub.with_app(|a| a.state().clone())
}

#[tokio::test]
async fn guest_sign_in_installs_saves_and_reports() {
    let s = setup();
    signed_out(&s);
    s.hub.sign_in_guest().await.unwrap();

    assert_eq!(state(&s), State::SignedIn);
    assert!(s.hub.with_session(|x| x.is_thirdweb_guest));
    assert!(s.hub.with_session(|x| x.try_address()).is_some());
    assert!(!saved_account(&s).is_empty());
    assert!(matches!(
        s.heard.sessions.lock().unwrap()[0],
        SessionEvent::WalletConnected { .. }
    ));
    let reports = s.heard.guest_reports.lock().unwrap();
    assert!(reports[0].success && reports[0].is_new_user);
}

#[tokio::test]
async fn a_failed_guest_sign_in_goes_back_and_names_the_reason() {
    let s = setup();
    signed_out(&s);
    *s.backend.fail_guest.lock().unwrap() = Some("timeout");
    let error = s.hub.sign_in_guest().await.unwrap_err();

    assert_eq!(error.code, "guest_timeout");
    assert_eq!(state(&s), State::SignedOut);
    assert_eq!(*s.heard.errors.lock().unwrap(), vec!["guest_timeout"]);
    assert!(saved_account(&s).is_empty());
    let report = s.heard.guest_reports.lock().unwrap()[0].clone();
    assert!(
        !report.success && report.failure_reason == "timeout" && report.http_status == Some(503)
    );
}

#[tokio::test]
async fn the_saved_account_is_restored_on_the_next_boot() {
    let s = setup();
    signed_out(&s);
    s.hub.sign_in_guest().await.unwrap();
    let address = s.hub.with_session(|x| x.address_string());

    let again = Hub::new(s.dir.path(), s.backend.clone());
    again.boot();
    assert_eq!(again.with_app(|a| a.state().clone()), State::SignedIn);
    assert_eq!(again.with_session(|x| x.address_string()), address);
}

#[tokio::test]
async fn an_ephemeral_session_never_overwrites_the_saved_one() {
    let s = setup();
    signed_out(&s);
    s.hub.set_persist_session(false);
    s.hub.sign_in_guest().await.unwrap();
    assert!(saved_account(&s).is_empty());
}

#[tokio::test]
async fn email_goes_through_the_code_step_and_keeps_it_on_a_wrong_code() {
    let s = setup();
    signed_out(&s);
    *s.backend.good_code.lock().unwrap() = "123456".into();

    s.hub.sign_in_email_start("a@b.c").await.unwrap();
    assert_eq!(
        state(&s),
        State::SigningIn {
            method: SignInMethod::Email,
            step: SignInStep::AwaitingCode
        }
    );

    let error = s
        .hub
        .sign_in_email_verify("a@b.c", "000000")
        .await
        .unwrap_err();
    assert_eq!(error.code, "invalid_code");
    assert!(matches!(state(&s), State::SigningIn { .. }));
    assert_eq!(*s.heard.errors.lock().unwrap(), vec!["invalid_code"]);

    s.hub.sign_in_email_verify("a@b.c", "123456").await.unwrap();
    assert_eq!(state(&s), State::SignedIn);
    assert!(!s.hub.with_session(|x| x.is_thirdweb_guest));
}

#[tokio::test]
async fn email_verify_needs_the_code_step() {
    let s = setup();
    signed_out(&s);
    let error = s.hub.sign_in_email_verify("a@b.c", "1").await.unwrap_err();
    assert_eq!(error.code, "invalid_state");
}

#[tokio::test]
async fn a_code_that_cannot_be_sent_returns_to_signed_out() {
    let s = setup();
    signed_out(&s);
    *s.backend.fail_email_send.lock().unwrap() = true;
    let error = s.hub.sign_in_email_start("a@b.c").await.unwrap_err();
    assert_eq!(error.code, "email_send_failed");
    assert_eq!(state(&s), State::SignedOut);
}

#[tokio::test]
async fn browser_sign_in_asks_the_shell_to_open_the_page_then_completes_from_the_link() {
    let s = setup();
    signed_out(&s);
    s.hub
        .sign_in_browser(Some("google".into()), None, Some("sid".into()))
        .unwrap();
    let url = s.heard.urls.lock().unwrap()[0].clone();
    assert!(
        url.contains("provider=google") && url.contains("s=sid"),
        "{url}"
    );
    assert!(s.hub.with_session(|x| x.has_pending_mobile_auth()));

    s.hub.complete_browser("identity-id".into()).await.unwrap();
    assert_eq!(state(&s), State::SignedIn);
    assert!(!s.hub.with_session(|x| x.has_pending_mobile_auth()));
}

#[tokio::test]
async fn the_deep_link_can_finish_a_sign_in_on_a_cold_start() {
    let s = setup();
    signed_out(&s);
    s.hub.complete_browser("identity-id".into()).await.unwrap();
    assert_eq!(state(&s), State::SignedIn);
}

#[tokio::test]
async fn a_cancelled_browser_sign_in_discards_a_late_completion() {
    let s = Arc::new(setup());
    signed_out(&s);
    let gate = Arc::new(tokio::sync::Notify::new());
    *s.backend.browser_gate.lock().unwrap() = Some(gate.clone());
    s.hub.sign_in_browser(None, None, None).unwrap();

    let running = {
        let s = s.clone();
        tokio::spawn(async move { s.hub.complete_browser("late".into()).await })
    };
    s.backend.browser_entered.notified().await;
    s.hub.cancel_sign_in();
    s.hub.sign_in_browser(None, None, None).unwrap();
    gate.notify_one();

    let error = running.await.unwrap().unwrap_err();
    assert_eq!(error.code, "cancelled");
    assert!(s.hub.with_session(|x| x.try_address()).is_none());
    assert!(matches!(state(&s), State::SigningIn { .. }));
}

#[tokio::test]
async fn a_failed_browser_completion_returns_to_signed_out() {
    let s = setup();
    signed_out(&s);
    s.hub.sign_in_browser(None, None, None).unwrap();
    *s.backend.browser_fails.lock().unwrap() = true;
    let error = s.hub.complete_browser("x".into()).await.unwrap_err();
    assert_eq!(error.code, "browser_completion_failed");
    assert_eq!(state(&s), State::SignedOut);
}

#[tokio::test]
async fn completing_a_browser_sign_in_while_signed_in_is_rejected() {
    let s = setup();
    signed_out(&s);
    s.hub.sign_in_guest().await.unwrap();
    let error = s.hub.complete_browser("x".into()).await.unwrap_err();
    assert_eq!(error.code, "invalid_state");
}

#[test]
fn wallet_sign_in_signs_a_core_made_message_and_keeps_the_key_inside() {
    let s = setup();
    signed_out(&s);
    let message = s.hub.sign_in_wallet_begin().unwrap();
    assert!(message.contains("Ephemeral address"), "{message}");

    // A real wallet signs the message with the personal-sign scheme.
    let wallet = LocalWallet::new(&mut thread_rng());
    let signature =
        futures_lite::future::block_on(wallet.sign_message(message.as_bytes())).unwrap();
    s.hub
        .sign_in_wallet_complete(
            &format!("{:#x}", wallet.address()),
            &format!("0x{signature}"),
        )
        .unwrap();

    assert_eq!(state(&s), State::SignedIn);
    assert_eq!(
        s.hub.with_session(|x| x.address_string()).to_lowercase(),
        format!("{:#x}", wallet.address())
    );
}

#[test]
fn wallet_completion_without_a_begin_is_rejected() {
    let s = setup();
    signed_out(&s);
    assert_eq!(
        s.hub
            .sign_in_wallet_complete("0x0", "0x0")
            .unwrap_err()
            .code,
        "invalid_state"
    );
}

#[test]
fn a_bad_wallet_signature_fails_the_sign_in() {
    let s = setup();
    signed_out(&s);
    s.hub.sign_in_wallet_begin().unwrap();
    let error = s
        .hub
        .sign_in_wallet_complete("0x0000000000000000000000000000000000000001", "0xnothex")
        .unwrap_err();
    assert_eq!(error.code, "wallet_signature_invalid");
    assert_eq!(state(&s), State::SignedOut);
}

#[tokio::test]
async fn sign_out_clears_the_session_and_the_saved_account() {
    let s = setup();
    signed_out(&s);
    s.hub.sign_in_guest().await.unwrap();
    s.hub.sign_out().unwrap();

    assert_eq!(state(&s), State::SignedOut);
    assert!(s.hub.with_session(|x| x.try_address()).is_none());
    assert!(saved_account(&s).is_empty());
    assert!(s
        .heard
        .sessions
        .lock()
        .unwrap()
        .contains(&SessionEvent::Logout));
    assert!(s.hub.sign_out().is_err());
}

#[test]
fn boot_without_a_saved_account_ends_signed_out_and_with_a_stale_one_too() {
    let s = setup();
    s.hub.boot();
    assert_eq!(state(&s), State::SignedOut);

    let dir = tempfile::tempdir().unwrap();
    SessionStore::new(dir.path())
        .set_slot(
            "",
            crate::session::store::SessionSlot {
                account: "garbage".into(),
                guest_profile: String::new(),
            },
        )
        .unwrap();
    let hub = Hub::new(dir.path(), Arc::new(Fake::default()));
    hub.boot();
    assert_eq!(hub.with_app(|a| a.state().clone()), State::SignedOut);
}

#[tokio::test]
async fn profile_slots_are_independent_and_keep_the_guest_profile() {
    let s = setup();
    signed_out(&s);
    SessionStore::new(s.dir.path())
        .set_slot(
            "",
            crate::session::store::SessionSlot {
                account: String::new(),
                guest_profile: "keep me".into(),
            },
        )
        .unwrap();
    s.hub.sign_in_guest().await.unwrap();
    let slot = SessionStore::new(s.dir.path()).slot("").unwrap();
    assert_eq!(slot.guest_profile, "keep me");
    assert!(SessionStore::new(s.dir.path()).slot("_other").is_none());
}

#[tokio::test]
async fn a_warm_link_finishes_the_browser_sign_in_that_this_process_started() {
    use crate::deep_link_route::{Effect, StartKind};
    let s = setup();
    signed_out(&s);
    s.hub.sign_in_browser(None, None, None).unwrap();

    let outcome = s.hub.route_deep_link(
        "decentraland://open?signin=abc123",
        StartKind::Warm,
        crate::Platform::Android,
    );
    assert_eq!(outcome.complete_sign_in.as_deref(), Some("abc123"));
    assert!(!outcome
        .routed
        .effects
        .iter()
        .any(|e| matches!(e, Effect::CompleteMobileSignIn { .. })));
    s.hub
        .complete_browser(outcome.complete_sign_in.unwrap())
        .await
        .unwrap();
    assert_eq!(state(&s), State::SignedIn);
}

#[test]
fn a_cold_link_signs_in_a_signed_out_user_and_is_dropped_for_a_signed_in_one() {
    use crate::deep_link_route::StartKind;
    let s = setup();
    signed_out(&s);
    let outcome = s.hub.route_deep_link(
        "decentraland://open?signin=cold1",
        StartKind::Cold,
        crate::Platform::Android,
    );
    assert_eq!(outcome.complete_sign_in.as_deref(), Some("cold1"));
    assert!(s.hub.resume_parked_sign_in().is_none(), "single shot");

    let booting = setup();
    let outcome = booting.hub.route_deep_link(
        "decentraland://open?signin=early",
        StartKind::Cold,
        crate::Platform::Android,
    );
    assert!(
        outcome.complete_sign_in.is_none(),
        "the saved account is not looked up yet"
    );
    booting.hub.boot();
    assert_eq!(
        booting.hub.resume_parked_sign_in().as_deref(),
        Some("early")
    );
}

#[test]
fn a_link_that_navigates_returns_the_destination_to_the_shell() {
    use crate::deep_link_route::{Navigation, StartKind};
    let s = setup();
    signed_out(&s);
    let outcome = s.hub.route_deep_link(
        "decentraland://open?position=10,20",
        StartKind::Warm,
        crate::Platform::Android,
    );
    assert!(matches!(
        outcome.routed.navigation,
        Some(Navigation::Teleport { .. })
    ));
    let desktop = s.hub.route_deep_link(
        "decentraland://open?position=10,20",
        StartKind::Warm,
        crate::Platform::Desktop,
    );
    assert!(desktop.routed.navigation.is_none());
}

#[test]
fn a_kept_report_reaches_the_listeners_with_the_tags_and_a_dropped_one_does_not() {
    use crate::report::{BuildEnvironment, ReportContext, ReportEvent, ReportSource};
    let s = setup();
    s.hub.set_report_context(
        ReportContext {
            platform: "Android".into(),
            gpu_vendor: "ARM".into(),
            ..Default::default()
        },
        BuildEnvironment::Production,
    );

    s.hub.report(ReportEvent::panic("boom"));
    let reports = s.heard.reports.lock().unwrap().clone();
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].source, ReportSource::Crash);
    assert_eq!(reports[0].tags.tag("platform"), Some("Android"));
    assert_eq!(reports[0].tags.tag("gpu_vendor"), Some("ARM"));

    s.hub
        .set_report_context(ReportContext::default(), BuildEnvironment::Development);
    s.hub.report(ReportEvent::panic("boom"));
    assert_eq!(
        s.heard.reports.lock().unwrap().len(),
        1,
        "development builds report nothing"
    );
}
