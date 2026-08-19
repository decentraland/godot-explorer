use ethers_core::types::H160;
use ethers_signers::LocalWallet;
use godot::prelude::*;
use rand::thread_rng;
use std::time::{Instant, UNIX_EPOCH};
use tokio::task::JoinHandle;

use crate::avatars::dcl_user_profile::DclUserProfile;
use crate::comms::profile::UserProfile;
use crate::dcl::scene_apis::RpcResultSender;
use crate::godot_classes::dcl_global::DclGlobal;
use crate::godot_classes::promise::Promise;
use crate::http_request::request_response::RequestResponse;
use crate::scene_runner::tokio_runtime::TokioRuntime;

use super::auth_identity::{
    complete_mobile_auth, create_ephemeral_from_external_signature, create_local_ephemeral,
    generate_ephemeral_for_signing, start_mobile_auth,
};
use super::decentraland_auth_server::{do_request, CreateRequest};
use super::device_anchor;
use super::ephemeral_auth_chain::EphemeralAuthChain;
use super::remote_wallet::RemoteWallet;
use super::thirdweb_guest;
use super::wallet::{AsH160, Wallet};

enum CurrentWallet {
    Remote(RemoteWallet),
    Local { wallet: Wallet, keys: Vec<u8> },
}

#[derive(GodotClass)]
#[class(base=Node)]
pub struct DclPlayerIdentity {
    wallet: Option<CurrentWallet>,
    ephemeral_auth_chain: Option<EphemeralAuthChain>,

    profile: Option<Gd<DclUserProfile>>,

    try_connect_account_handle: Option<JoinHandle<()>>,

    /// Pending mobile auth state, stored between start_mobile_connect_account
    /// and complete_mobile_connect_account (when deep link arrives).
    ///
    /// Process-local and best-effort: a cold start (OS kill during the browser hop) legitimately
    /// loses it, which is why `complete_mobile_connect_account` does not require it. Read via
    /// `has_pending_mobile_auth` only to tell "this process started the flow" — i.e. the lobby
    /// is already showing the spinner — from "the deep link arrived out of the blue".
    pending_mobile_auth: Option<()>,

    /// Set when the user cancels a browser sign-in this process started — the Cancel on
    /// AUTH_BROWSER_OPEN, via `abort_try_connect_account`, gated on `mobile_auth_started`.
    ///
    /// A cold start cannot have this set — the process is new — which is exactly what makes
    /// it able to tell "the user cancelled" apart from "the OS killed us mid-browser" now
    /// that `complete_mobile_connect_account` no longer requires `pending_mobile_auth`
    /// (#2644). Without it, a deep link returning after a cancel is indistinguishable from
    /// the cold start and completes the sign-in the user just refused.
    ///
    /// Cleared by `start_mobile_connect_account` and by `logout`, so neither a deliberate new
    /// attempt nor the next account inherits it.
    mobile_auth_cancelled: bool,

    /// True once `start_mobile_connect_account` ran in this process, i.e. this process is the
    /// one that sent the user to the browser.
    ///
    /// Gates `mobile_auth_cancelled`, because `abort_try_connect_account` is shared with
    /// sign-in flows that never open our browser hop — the Android native WalletConnect path
    /// (`login.gd:_on_button_wallet_connect_pressed`) shows the same AUTH_BROWSER_OPEN screen,
    /// Cancel included, without starting a mobile auth. Without this gate, cancelling MetaMask
    /// would discard a late `?signin=` token from an earlier process: exactly the #2644 loss
    /// this branch exists to stop.
    ///
    /// Set synchronously, unlike `pending_mobile_auth`, which is only set when the spawned
    /// `start_mobile_auth` resolves — so it is already true during the browser-opening window.
    mobile_auth_started: bool,

    /// Monotonic browser sign-in attempt token, mirroring `lobby.gd`'s `_guest_login_attempt`.
    /// Bumped when an attempt starts and when one is cancelled, captured by the completion
    /// task, and re-checked on the main thread before the result is applied.
    ///
    /// Tracking the completion's `JoinHandle` is not enough on its own: `abort()` only unwinds
    /// a task at a yield point, and the only `.await` in that task is the identity fetch —
    /// everything after it is synchronous. So a fetch that resolved just before the user
    /// tapped Cancel would still install the wallet and emit `wallet_connected`, signing the
    /// user into the account they just refused.
    mobile_auth_attempt: u32,

    #[var]
    is_guest: bool,

    /// `true` only when the active wallet is a silent persistent thirdweb guest
    /// (created via `async_create_guest_account`). Distinct from `is_guest`,
    /// which stays reserved for disposable LocalWallet sessions — a thirdweb
    /// guest is `is_guest = false` but `is_thirdweb_guest = true`. Gates the
    /// "Upgrade to OTP" affordance, which only makes sense for this account
    /// type. Cleared on every other account path so it never leaks across an
    /// account switch.
    is_thirdweb_guest: bool,

    /// `true` when the active thirdweb guest has at least one linked auth method
    /// beyond the silent `guest` login (email/social/passkey) — i.e. it has
    /// already been upgraded. Best-effort cache refreshed from thirdweb via
    /// `async_refresh_thirdweb_upgrade_state` (and set on a successful link).
    /// Only meaningful while `is_thirdweb_guest` is `true`; cleared on every
    /// account switch so it never leaks. Lets the UI hide the Upgrade affordance
    /// for guests that are already linked.
    is_thirdweb_guest_upgraded: bool,

    base: Base<Node>,
}

#[godot_api]
impl INode for DclPlayerIdentity {
    fn init(base: Base<Node>) -> Self {
        Self {
            wallet: None,
            ephemeral_auth_chain: None,
            profile: None,
            base,
            is_guest: false,
            is_thirdweb_guest: false,
            is_thirdweb_guest_upgraded: false,
            try_connect_account_handle: None,
            pending_mobile_auth: None,
            mobile_auth_cancelled: false,
            mobile_auth_started: false,
            mobile_auth_attempt: 0,
        }
    }
}

#[godot_api]
impl DclPlayerIdentity {
    #[signal]
    fn logout();

    #[signal]
    fn wallet_connected(address: GString, chain_id: u64, is_guest: bool);

    #[signal]
    fn profile_changed(new_profile: Gd<DclUserProfile>);

    #[signal]
    fn auth_error(error_message: GString);

    /// One per silent thirdweb guest-login attempt, for analytics. `outcome` is
    /// "success" | "failure". On success `failure_reason` is "" and `http_status`
    /// is -1; `is_new_user` says whether a NEW wallet was minted. On failure
    /// `failure_reason` is a stable code (see thirdweb_guest::GuestLoginError)
    /// and `http_status` is the upstream status or -1. `duration_ms` is the
    /// whole attempt's wall-clock time. Emitted deferred from the login task so
    /// GDScript (analytics_controller.gd) can forward it to Segment.
    #[signal]
    fn guest_login_outcome(
        outcome: GString,
        is_new_user: bool,
        failure_reason: GString,
        http_status: i64,
        duration_ms: i64,
    );

    #[func]
    fn try_set_remote_wallet(
        &mut self,
        address_string: GString,
        chain_id: u64,
        ephemeral_auth_chain: GString,
    ) -> bool {
        let address = address_string
            .to_string()
            .as_str()
            .as_h160()
            .expect("invalid wallet address");

        let ephemeral_auth_chain = match serde_json::from_str(&ephemeral_auth_chain.to_string()) {
            Ok(p) => p,
            Err(e) => {
                tracing::error!(
                    "error {e} invalid data ephemeral_auth_chain {:?}",
                    ephemeral_auth_chain
                );
                self.base_mut().call_deferred(
                    "_error_getting_wallet",
                    &["Error parsing ephemeral_auth_chain".to_variant()],
                );
                return false;
            }
        };

        self._update_remote_wallet(address, chain_id, ephemeral_auth_chain);
        true
    }

    fn _update_remote_wallet(
        &mut self,
        account_address: H160,
        chain_id: u64,
        ephemeral_auth_chain: EphemeralAuthChain,
    ) {
        self.wallet = Some(CurrentWallet::Remote(RemoteWallet::new(
            account_address,
            chain_id,
        )));
        self.ephemeral_auth_chain = Some(ephemeral_auth_chain);

        let address = self.get_address();
        self.base_mut().call_deferred(
            "emit_signal",
            &[
                "wallet_connected".to_variant(),
                format!("{:#x}", address).to_variant(),
                chain_id.to_variant(),
                false.to_variant(),
            ],
        );
        self.is_guest = false;
        // Default for any remote connect (WalletConnect / social). The thirdweb
        // guest path re-sets this to `true` via a deferred setter right after.
        self.is_thirdweb_guest = false;
        self.is_thirdweb_guest_upgraded = false;
    }

    /// Deferred setter for the thirdweb-guest marker. Run on the main thread
    /// from `async_create_guest_account`'s success branch so the flag flips
    /// only after `try_set_remote_wallet` (also deferred) has installed the
    /// wallet and reset the marker to `false`.
    #[func]
    fn _set_thirdweb_guest_flag(&mut self, value: bool) {
        self.is_thirdweb_guest = value;
    }

    /// Whether the active account is a silent persistent thirdweb guest. Gates
    /// the Settings "Upgrade to OTP" affordance from GDScript.
    #[func]
    fn is_thirdweb_guest(&self) -> bool {
        self.is_thirdweb_guest
    }

    /// Deferred setter for the upgraded marker. Called from the thirdweb thread
    /// after a profiles query (`async_refresh_thirdweb_upgrade_state`) or a
    /// successful email link, so the write lands on the main thread.
    #[func]
    fn _set_thirdweb_guest_upgraded_flag(&mut self, value: bool) {
        self.is_thirdweb_guest_upgraded = value;
    }

    /// Whether the active thirdweb guest has already linked a non-`guest` auth
    /// method (email/social). Best-effort cache — call
    /// `async_refresh_thirdweb_upgrade_state` to confirm against thirdweb. Lets
    /// the UI hide the Upgrade affordance for already-upgraded guests.
    #[func]
    fn is_thirdweb_guest_upgraded(&self) -> bool {
        self.is_thirdweb_guest_upgraded
    }

    fn _update_local_wallet(
        &mut self,
        local_wallet_bytes: &[u8],
        ephemeral_auth_chain: EphemeralAuthChain,
    ) {
        let local_wallet = Wallet::new_from_inner(Box::new(
            LocalWallet::from_bytes(local_wallet_bytes).unwrap(),
        ));

        self.wallet = Some(CurrentWallet::Local {
            wallet: local_wallet,
            keys: Vec::from_iter(local_wallet_bytes.iter().cloned()),
        });

        self.ephemeral_auth_chain = Some(ephemeral_auth_chain);

        let address = format!("{:#x}", self.get_address());

        self.base_mut().call_deferred(
            "emit_signal",
            &[
                "wallet_connected".to_variant(),
                address.to_variant(),
                1_u64.to_variant(),
                true.to_variant(),
            ],
        );
        self.is_guest = true;
        self.is_thirdweb_guest = false;
        self.is_thirdweb_guest_upgraded = false;
        self.profile = None;
    }

    #[func]
    fn _error_getting_wallet(&mut self, error_str: GString) {
        tracing::error!("error getting wallet {:?}", error_str);
        self.base_mut()
            .emit_signal("auth_error", &[error_str.to_variant()]);
    }

    /// Creates a random throwaway wallet that lives only as long as the install.
    /// "Disposable" because every cold start mints a brand-new wallet — no
    /// persistence, no recovery. Reserved for dev / hidden behind double-tap
    /// in non-prod. For silent + persistent guest, use `async_create_guest_account`.
    #[func]
    fn create_disposable_account(&mut self) {
        let local_wallet = LocalWallet::new(&mut thread_rng());
        let local_wallet_bytes = local_wallet.signer().to_bytes().to_vec();
        let ephemeral_auth_chain = create_local_ephemeral(&local_wallet);
        self._update_local_wallet(local_wallet_bytes.as_slice(), ephemeral_auth_chain);
    }

    /// Silent guest login backed by thirdweb. Returns a Promise that resolves
    /// with the wallet address string on success, or rejects with an error
    /// message. The same `device_anchor_id` always resolves to the same
    /// wallet address — pass the SSAID (Android) / Keychain UUID (iOS), or
    /// leave empty to use the desktop UUID file fallback.
    #[func]
    fn async_create_guest_account(&mut self, device_anchor_id: GString) -> Gd<Promise> {
        let (promise, get_promise) = Promise::make_to_async();
        let instance_id = self.base().instance_id();

        let Some(handle) = TokioRuntime::static_clone_handle() else {
            let mut promise_clone = promise.clone();
            promise_clone
                .bind_mut()
                .reject("Tokio runtime not initialized".into());
            return promise;
        };

        let anchor_input = device_anchor_id.to_string();

        handle.spawn(async move {
            let started = Instant::now();
            let result = perform_thirdweb_guest_login(anchor_input).await;
            let duration_ms = started.elapsed().as_millis() as i64;
            let Some(mut promise) = get_promise() else {
                tracing::error!("thirdweb guest_login: promise dropped");
                return;
            };

            // Reused by both arms to reach the node on the main thread: the
            // success arm installs the wallet, and both emit the analytics signal.
            let mut identity = Gd::<DclPlayerIdentity>::try_from_instance_id(instance_id).ok();

            match result {
                Ok(outcome) => {
                    let address_str = format!("{:#x}", outcome.address);
                    let ephemeral_chain_json = serde_json::to_string(&outcome.chain)
                        .expect("serialize ephemeral auth chain");

                    if let Some(identity) = identity.as_mut() {
                        // Thirdweb wallets are real custodial wallets — same model
                        // as WalletConnect / Apple / Google as far as Decentraland
                        // is concerned. The thirdweb "guest" label is just the auth
                        // method (no social link) and doesn't affect `is_guest`,
                        // which stays reserved for disposable LocalWallet sessions.
                        identity.call_deferred(
                            "try_set_remote_wallet",
                            &[
                                address_str.clone().to_variant(),
                                1_u64.to_variant(),
                                ephemeral_chain_json.to_variant(),
                            ],
                        );
                        // Flip the thirdweb-guest marker after the (also
                        // deferred) wallet install, which resets it to `false`.
                        // Both run on the main thread in submission order.
                        identity.call_deferred("_set_thirdweb_guest_flag", &[true.to_variant()]);
                        identity.call_deferred(
                            "emit_signal",
                            &[
                                "guest_login_outcome".to_variant(),
                                "success".to_variant(),
                                outcome.is_new_user.to_variant(),
                                GString::from("").to_variant(),
                                (-1_i64).to_variant(),
                                duration_ms.to_variant(),
                            ],
                        );
                    }

                    promise
                        .bind_mut()
                        .resolve_with_data(address_str.to_variant());
                }
                Err(e) => {
                    tracing::error!("thirdweb guest_login failed: {:?}", e);
                    if let Some(identity) = identity.as_mut() {
                        identity.call_deferred(
                            "emit_signal",
                            &[
                                "guest_login_outcome".to_variant(),
                                "failure".to_variant(),
                                false.to_variant(),
                                e.reason.to_variant(),
                                e.http_status.map(i64::from).unwrap_or(-1).to_variant(),
                                duration_ms.to_variant(),
                            ],
                        );
                    }
                    promise
                        .bind_mut()
                        .reject(GString::from(&format!("Guest login failed: {}", e)));
                }
            }
        });

        promise
    }

    /// Step 1 of "Upgrade to OTP": sends a one-time code to `email`. No token
    /// is needed. Resolves with `true` on success, rejects with a friendly
    /// message otherwise. Only meaningful when `is_thirdweb_guest()` is true.
    #[func]
    fn async_link_email_start(&mut self, email: GString) -> Gd<Promise> {
        let (promise, get_promise) = Promise::make_to_async();

        let Some(handle) = TokioRuntime::static_clone_handle() else {
            let mut promise_clone = promise.clone();
            promise_clone
                .bind_mut()
                .reject("Tokio runtime not initialized".into());
            return promise;
        };

        let email = email.to_string();

        handle.spawn(async move {
            let result = thirdweb_guest::email_initiate(&email).await;
            let Some(mut promise) = get_promise() else {
                tracing::error!("thirdweb email_initiate: promise dropped");
                return;
            };

            match result {
                Ok(()) => {
                    promise.bind_mut().resolve_with_data(true.to_variant());
                }
                Err(e) => {
                    tracing::error!("thirdweb email_initiate failed: {:?}", e);
                    promise
                        .bind_mut()
                        .reject(GString::from(&format!("Could not send code: {}", e)));
                }
            }
        });

        promise
    }

    /// Step 2 of "Upgrade to OTP": verifies the `code`, then links the email
    /// identity into the existing guest wallet so the SAME address becomes
    /// email-recoverable. `device_anchor_id` is used to refresh the guest JWT
    /// (the persisted one may be expired) — pass the same value the lobby uses.
    /// Resolves with the (unchanged) guest wallet address string on success.
    #[func]
    fn async_link_email_verify(
        &mut self,
        email: GString,
        code: GString,
        device_anchor_id: GString,
    ) -> Gd<Promise> {
        let (promise, get_promise) = Promise::make_to_async();
        let instance_id = self.base().instance_id();

        let Some(handle) = TokioRuntime::static_clone_handle() else {
            let mut promise_clone = promise.clone();
            promise_clone
                .bind_mut()
                .reject("Tokio runtime not initialized".into());
            return promise;
        };

        let email = email.to_string();
        let code = code.to_string();
        let anchor = device_anchor_id.to_string();

        handle.spawn(async move {
            let result = perform_link_email(anchor, email, code).await;
            let Some(mut promise) = get_promise() else {
                tracing::error!("thirdweb link_email: promise dropped");
                return;
            };

            match result {
                Ok(address) => {
                    // The guest now has an email linked — it's upgraded. Update
                    // the cached marker so the UI hides the affordance without a
                    // round trip to re-query profiles.
                    if let Ok(mut identity) =
                        Gd::<DclPlayerIdentity>::try_from_instance_id(instance_id)
                    {
                        identity.call_deferred(
                            "_set_thirdweb_guest_upgraded_flag",
                            &[true.to_variant()],
                        );
                    }
                    promise
                        .bind_mut()
                        .resolve_with_data(format!("{:#x}", address).to_variant());
                }
                Err(e) => {
                    tracing::error!("thirdweb link_email failed: {:?}", e);
                    promise
                        .bind_mut()
                        .reject(GString::from(&format!("Could not verify code: {}", e)));
                }
            }
        });

        promise
    }

    /// Native email login: verifies the OTP `code` sent to `email`, then mints
    /// a local ephemeral keypair and delegates signing to the email wallet.
    /// Unlike `async_link_email_verify` (which merges the email into an existing
    /// guest), this signs in directly as the email identity — no guest session is
    /// required or created. Resolves with the email wallet address string on
    /// success; rejects with a human-readable error otherwise.
    #[func]
    fn async_login_email_verify(&mut self, email: GString, code: GString) -> Gd<Promise> {
        let (promise, get_promise) = Promise::make_to_async();
        let instance_id = self.base().instance_id();

        let Some(handle) = TokioRuntime::static_clone_handle() else {
            let mut promise_clone = promise.clone();
            promise_clone
                .bind_mut()
                .reject("Tokio runtime not initialized".into());
            return promise;
        };

        let email = email.to_string();
        let code = code.to_string();

        handle.spawn(async move {
            let result = perform_email_login(email, code).await;
            let Some(mut promise) = get_promise() else {
                tracing::warn!("thirdweb email_login: promise dropped");
                return;
            };

            match result {
                Ok((address, ephemeral_auth_chain)) => {
                    let address_str = format!("{:#x}", address);
                    let ephemeral_chain_json = serde_json::to_string(&ephemeral_auth_chain)
                        .expect("serialize ephemeral auth chain");

                    if let Ok(mut identity) =
                        Gd::<DclPlayerIdentity>::try_from_instance_id(instance_id)
                    {
                        identity.call_deferred(
                            "try_set_remote_wallet",
                            &[
                                address_str.clone().to_variant(),
                                1_u64.to_variant(),
                                ephemeral_chain_json.to_variant(),
                            ],
                        );
                    }

                    promise
                        .bind_mut()
                        .resolve_with_data(address_str.to_variant());
                }
                Err(e) => {
                    tracing::warn!("thirdweb email_login failed: {:?}", e);
                    promise
                        .bind_mut()
                        .reject(GString::from(&format!("Could not verify code: {}", e)));
                }
            }
        });

        promise
    }

    /// Queries thirdweb for the account's linked auth methods and refreshes the
    /// cached `is_thirdweb_guest_upgraded` flag. Resolves with `true` when the
    /// guest already has a non-`guest` profile (email/social), `false` when it
    /// only has the silent id-login. Re-derives a fresh guest token from the
    /// device anchor so the query is authorized even if the persisted token aged
    /// out. Rejects on network error — the caller should keep the last-known
    /// state rather than assume "not upgraded".
    #[func]
    fn async_refresh_thirdweb_upgrade_state(&mut self, device_anchor_id: GString) -> Gd<Promise> {
        let (promise, get_promise) = Promise::make_to_async();
        let instance_id = self.base().instance_id();

        let Some(handle) = TokioRuntime::static_clone_handle() else {
            let mut promise_clone = promise.clone();
            promise_clone
                .bind_mut()
                .reject("Tokio runtime not initialized".into());
            return promise;
        };

        let anchor_input = device_anchor_id.to_string();

        handle.spawn(async move {
            let result = async {
                let session = thirdweb_guest::refresh_guest_session(&anchor_input).await?;
                let types = thirdweb_guest::get_linked_profile_types(&session.token).await?;
                Ok::<bool, anyhow::Error>(thirdweb_guest::account_is_upgraded(&types))
            }
            .await;

            let Some(mut promise) = get_promise() else {
                return;
            };

            match result {
                Ok(upgraded) => {
                    if let Ok(mut identity) =
                        Gd::<DclPlayerIdentity>::try_from_instance_id(instance_id)
                    {
                        identity.call_deferred(
                            "_set_thirdweb_guest_upgraded_flag",
                            &[upgraded.to_variant()],
                        );
                    }
                    promise.bind_mut().resolve_with_data(upgraded.to_variant());
                }
                Err(e) => {
                    tracing::warn!("thirdweb refresh upgrade state failed: {:?}", e);
                    promise.bind_mut().reject(GString::from(&format!(
                        "Could not check upgrade state: {}",
                        e
                    )));
                }
            }
        });

        promise
    }

    /// Deletes the current thirdweb guest account server-side by unlinking its
    /// sole `guest` profile (issue #2335). For a non-upgraded guest the silent
    /// id-login is the only identity, so removing it deletes the thirdweb user,
    /// which frees the deterministic `sessionId` → the next `guest_login` from
    /// the same device anchor mints a BRAND-NEW wallet. A fresh guest JWT is
    /// re-derived from the anchor so the unlink is authorized even if the
    /// persisted token aged out.
    ///
    /// Best-effort: resolves `true` on success, `false` on failure — it never
    /// rejects — so the GDScript caller can always continue to wipe local
    /// storage and sign out regardless of the network outcome. Only call this
    /// for a confirmed non-upgraded guest; `unlink_guest_profile` additionally
    /// refuses if any non-`guest` profile is linked, as a safety net.
    #[func]
    fn async_delete_guest_account(&mut self, device_anchor_id: GString) -> Gd<Promise> {
        let (promise, get_promise) = Promise::make_to_async();

        let Some(handle) = TokioRuntime::static_clone_handle() else {
            let mut promise_clone = promise.clone();
            promise_clone
                .bind_mut()
                .reject("Tokio runtime not initialized".into());
            return promise;
        };

        let anchor = device_anchor_id.to_string();

        handle.spawn(async move {
            let result = perform_delete_guest_account(anchor).await;
            let Some(mut promise) = get_promise() else {
                tracing::error!("thirdweb delete_guest: promise dropped");
                return;
            };

            match result {
                Ok(()) => {
                    tracing::info!("thirdweb delete_guest: guest account deleted");
                    promise.bind_mut().resolve_with_data(true.to_variant());
                }
                Err(e) => {
                    // Non-fatal: the caller wipes local state + signs out anyway.
                    tracing::warn!("thirdweb delete_guest failed (non-fatal): {:?}", e);
                    promise.bind_mut().resolve_with_data(false.to_variant());
                }
            }
        });

        promise
    }

    /// Like `async_delete_guest_account` but for an **upgraded** guest: unlinks
    /// BOTH the `guest` and the linked `email` profile (double unlink), fully
    /// deleting the recoverable thirdweb user rather than merely stripping it
    /// back to a bare guest. A fresh guest JWT is re-derived from the anchor (a
    /// guest login on an upgraded account returns that same user's token).
    ///
    /// This bypasses the `unlink_guest_profile` safety refusal, so it MUST only
    /// be called behind the non-prod + `enable-upgraded-deletion` deeplink gate
    /// (enforced in GDScript). Best-effort: resolves `true` on success, `false`
    /// on failure — it never rejects — so the caller can still wipe local
    /// storage and sign out regardless of the network outcome.
    #[func]
    fn async_delete_upgraded_account(&mut self, device_anchor_id: GString) -> Gd<Promise> {
        let (promise, get_promise) = Promise::make_to_async();

        let Some(handle) = TokioRuntime::static_clone_handle() else {
            let mut promise_clone = promise.clone();
            promise_clone
                .bind_mut()
                .reject("Tokio runtime not initialized".into());
            return promise;
        };

        let anchor = device_anchor_id.to_string();

        handle.spawn(async move {
            let result = perform_delete_upgraded_account(anchor).await;
            let Some(mut promise) = get_promise() else {
                tracing::error!("thirdweb delete_upgraded: promise dropped");
                return;
            };

            match result {
                Ok(()) => {
                    tracing::info!("thirdweb delete_upgraded: upgraded account deleted");
                    promise.bind_mut().resolve_with_data(true.to_variant());
                }
                Err(e) => {
                    // Non-fatal: the caller wipes local state + signs out anyway.
                    tracing::warn!("thirdweb delete_upgraded failed (non-fatal): {:?}", e);
                    promise.bind_mut().resolve_with_data(false.to_variant());
                }
            }
        });

        promise
    }

    #[func]
    fn try_connect_account(&mut self) {
        let Some(handle) = TokioRuntime::static_clone_handle() else {
            panic!("tokio runtime not initialized")
        };

        let instance_id = self.base().instance_id();
        let sender = DclGlobal::singleton()
            .bind()
            .get_dcl_tokio_rpc()
            .bind()
            .get_sender();

        let try_connect_account_handle = handle.spawn(async move {
            let wallet = RemoteWallet::with_auth_identity(sender).await;
            let Ok(mut this) = Gd::<DclPlayerIdentity>::try_from_instance_id(instance_id) else {
                return;
            };

            match wallet {
                Ok((wallet, ephemeral_auth_chain)) => {
                    let ephemeral_auth_chain_json_str =
                        serde_json::to_string(&ephemeral_auth_chain)
                            .expect("serialize ephemeral auth chain");

                    this.call_deferred(
                        "try_set_remote_wallet",
                        &[
                            format!("{:#x}", wallet.address()).to_variant(),
                            wallet.chain_id().to_variant(),
                            ephemeral_auth_chain_json_str.to_variant(),
                        ],
                    );
                }
                Err(err) => {
                    tracing::error!("error getting wallet {:?}", err);
                    this.call_deferred("_error_getting_wallet", &["Unknown error".to_variant()]);
                }
            }
        });

        self.try_connect_account_handle = Some(try_connect_account_handle);
    }

    #[func]
    fn abort_try_connect_account(&mut self) {
        if let Some(handle) = self.try_connect_account_handle.take() {
            handle.abort();
        }
        // Also clear any pending mobile auth
        self.pending_mobile_auth = None;
        // Remember the cancel for the rest of this process. Clearing the flag above is not
        // enough to make the cancel stick: a deep link arriving afterwards would look exactly
        // like the cold start #2644 rescues, and complete the sign-in anyway.
        //
        // Only when this process actually opened the browser, though — this abort is shared
        // with the native WalletConnect path, which shows the same Cancel without starting a
        // mobile auth, and marking that as cancelled would throw away a legitimate late token.
        // Gating on `mobile_auth_started` rather than `pending_mobile_auth` still covers the
        // case that needs it most, where start_mobile_connect_account's un-abortable spawn has
        // not set the pending flag yet.
        self.mobile_auth_cancelled = self.mobile_auth_started;
        // Retire any in-flight attempt: a completion that already got past its only await
        // cannot be aborted, so it has to be told its result is no longer wanted.
        self.mobile_auth_attempt = self.mobile_auth_attempt.wrapping_add(1);
    }

    /// Starts mobile auth flow. Opens browser and returns immediately.
    /// The app should wait for a deep link with signin identity ID,
    /// then call complete_mobile_connect_account with that ID.
    #[func]
    fn start_mobile_connect_account(
        &mut self,
        provider: GString,
        user_id: GString,
        session_id: GString,
    ) {
        // This process is now the one that owns the browser hop, and a deliberate new attempt
        // retires any earlier cancel.
        self.mobile_auth_started = true;
        self.mobile_auth_cancelled = false;
        self.mobile_auth_attempt = self.mobile_auth_attempt.wrapping_add(1);

        let Some(handle) = TokioRuntime::static_clone_handle() else {
            panic!("tokio runtime not initialized")
        };

        let instance_id = self.base().instance_id();
        let sender = DclGlobal::singleton()
            .bind()
            .get_dcl_tokio_rpc()
            .bind()
            .get_sender();

        let provider = if provider.is_empty() {
            None
        } else {
            Some(provider.to_string())
        };
        let user_id = if user_id.is_empty() {
            None
        } else {
            Some(user_id.to_string())
        };
        let session_id = if session_id.is_empty() {
            None
        } else {
            Some(session_id.to_string())
        };

        handle.spawn(async move {
            let result = start_mobile_auth(sender, provider, user_id, session_id).await;
            let Ok(mut this) = Gd::<DclPlayerIdentity>::try_from_instance_id(instance_id) else {
                return;
            };

            match result {
                Ok(pending) => {
                    tracing::info!("Mobile auth started, waiting for deep link");
                    this.bind_mut().pending_mobile_auth = Some(pending);
                }
                Err(err) => {
                    tracing::error!("Error starting mobile auth: {:?}", err);
                    this.call_deferred(
                        "_error_getting_wallet",
                        &[format!("Mobile auth error: {}", err).to_variant()],
                    );
                }
            }
        });
    }

    /// Completes mobile auth flow using the identity ID received via deep link.
    /// Should be called when app receives deep link `decentraland://open?signin=${identityId}`
    ///
    /// Deliberately NOT gated on `pending_mobile_auth` (#2644). That flag lives only in
    /// this process's memory, so an OS kill while the user is in the browser wipes it and
    /// the deep link comes back to a cold start — where refusing to complete threw away a
    /// sign-in the user had already finished. The flag also carries no data (`Option<()>`):
    /// everything needed is behind `complete_mobile_auth(identity_id)`, and an invalid or
    /// expired id already fails there with a real error.
    #[func]
    fn complete_mobile_connect_account(&mut self, identity_id: GString) {
        // Consume the flag when this process did start the flow; its absence is not an error.
        self.pending_mobile_auth.take();

        let Some(handle) = TokioRuntime::static_clone_handle() else {
            panic!("tokio runtime not initialized")
        };

        let instance_id = self.base().instance_id();
        let identity_id = identity_id.to_string();
        // Both arms below are re-checked against this on the main thread, so a cancel that
        // lost the race to `abort()` still discards the result instead of applying it.
        let attempt = self.mobile_auth_attempt;

        // Tracked, unlike the other spawns here, so `abort_try_connect_account` can actually
        // stop it. The lobby shows a Cancel button while this runs — including on the
        // cold-start resume (#2644), where the user has no other way out — and an untracked
        // handle made that button a lie: the sign-in landed anyway, moments after the cancel.
        let complete_handle = handle.spawn(async move {
            let result = complete_mobile_auth(identity_id).await;
            let Ok(mut this) = Gd::<DclPlayerIdentity>::try_from_instance_id(instance_id) else {
                return;
            };

            match result {
                Ok((ephemeral_auth_chain, chain_id)) => {
                    let address = ephemeral_auth_chain.signer();
                    let ephemeral_auth_chain_json_str =
                        serde_json::to_string(&ephemeral_auth_chain)
                            .expect("serialize ephemeral auth chain");

                    this.call_deferred(
                        "_mobile_auth_completed",
                        &[
                            attempt.to_variant(),
                            format!("{:#x}", address).to_variant(),
                            chain_id.to_variant(),
                            ephemeral_auth_chain_json_str.to_variant(),
                        ],
                    );
                }
                Err(err) => {
                    tracing::error!("Error completing mobile auth: {:?}", err);
                    this.call_deferred(
                        "_mobile_auth_failed",
                        &[
                            attempt.to_variant(),
                            format!("Mobile auth completion error: {}", err).to_variant(),
                        ],
                    );
                }
            }
        });

        // Abort rather than drop: dropping a tokio JoinHandle detaches the task, so a handle
        // already in this slot would keep running with nothing able to stop it.
        if let Some(previous) = self.try_connect_account_handle.replace(complete_handle) {
            previous.abort();
        }
    }

    /// Main-thread landing for a successful `complete_mobile_connect_account`, guarded by the
    /// attempt token so a result the user cancelled is discarded instead of applied.
    #[func]
    fn _mobile_auth_completed(
        &mut self,
        attempt: u32,
        address: GString,
        chain_id: u64,
        ephemeral_auth_chain: GString,
    ) {
        if attempt != self.mobile_auth_attempt {
            tracing::info!(
                "Discarding a completed mobile sign-in: attempt {} was retired (now {})",
                attempt,
                self.mobile_auth_attempt
            );
            return;
        }
        self.try_set_remote_wallet(address, chain_id, ephemeral_auth_chain);
    }

    /// Main-thread landing for a failed `complete_mobile_connect_account`. Guarded for the same
    /// reason as the success path: a cancelled attempt must not surface an error over whatever
    /// screen the user moved on to.
    #[func]
    fn _mobile_auth_failed(&mut self, attempt: u32, error_message: GString) {
        if attempt != self.mobile_auth_attempt {
            tracing::info!(
                "Discarding a failed mobile sign-in: attempt {} was retired (now {})",
                attempt,
                self.mobile_auth_attempt
            );
            return;
        }
        self.base_mut()
            .call_deferred("_error_getting_wallet", &[error_message.to_variant()]);
    }

    /// Returns true if there's a pending mobile auth waiting for deep link
    #[func]
    fn has_pending_mobile_auth(&self) -> bool {
        self.pending_mobile_auth.is_some()
    }

    /// Returns true if the user cancelled an in-flight sign-in in this process. The deep-link
    /// router checks this before anything else, so a `?signin=` token that arrives after a
    /// cancel is refused instead of being mistaken for the #2644 cold start.
    #[func]
    fn was_mobile_auth_cancelled(&self) -> bool {
        self.mobile_auth_cancelled
    }

    /// Generates ephemeral identity data for external signing (e.g., WalletConnect).
    /// Returns a Dictionary with:
    /// - "message": The message to be signed by the wallet
    /// - "ephemeral_private_key": PackedByteArray of the ephemeral private key
    /// - "expiration_timestamp": Unix timestamp (seconds) when the auth expires
    #[func]
    fn generate_ephemeral_for_signing(&self) -> VarDictionary {
        let (ephemeral_message, signing_key_bytes, expiration) = generate_ephemeral_for_signing();

        let expiration_timestamp = expiration
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let mut dict = VarDictionary::new();
        let _ = dict.insert("message", ephemeral_message.to_variant());
        let _ = dict.insert(
            "ephemeral_private_key",
            PackedByteArray::from(signing_key_bytes.as_slice()).to_variant(),
        );
        let _ = dict.insert("expiration_timestamp", expiration_timestamp.to_variant());
        dict
    }

    /// Completes WalletConnect authentication using an externally-signed message.
    /// This should be called after getting a signature from a native wallet app.
    ///
    /// # Arguments
    /// * `signer_address` - The wallet address that signed the message (0x...)
    /// * `signature` - The signature hex string from the wallet
    /// * `ephemeral_private_key` - The ephemeral private key from generate_ephemeral_for_signing
    /// * `expiration_timestamp` - Unix timestamp from generate_ephemeral_for_signing
    /// * `original_message` - The exact message that was signed by the wallet
    ///
    /// # Returns
    /// true if authentication was successful, false otherwise
    #[func]
    fn try_set_walletconnect_auth(
        &mut self,
        signer_address: GString,
        signature: GString,
        ephemeral_private_key: PackedByteArray,
        expiration_timestamp: i64,
        original_message: GString,
    ) -> bool {
        let expiration = std::time::SystemTime::UNIX_EPOCH
            + std::time::Duration::from_secs(expiration_timestamp as u64);

        match create_ephemeral_from_external_signature(
            &signer_address.to_string(),
            &signature.to_string(),
            ephemeral_private_key.as_slice(),
            expiration,
            &original_message.to_string(),
        ) {
            Ok(ephemeral_auth_chain) => {
                let address = ephemeral_auth_chain.signer();
                self.wallet = Some(CurrentWallet::Remote(RemoteWallet::new(address, 1)));
                self.ephemeral_auth_chain = Some(ephemeral_auth_chain);

                let address_str = format!("{:#x}", address);
                self.base_mut().call_deferred(
                    "emit_signal",
                    &[
                        "wallet_connected".to_variant(),
                        address_str.to_variant(),
                        1_u64.to_variant(),
                        false.to_variant(),
                    ],
                );
                self.is_guest = false;

                tracing::info!("WalletConnect auth successful for address: {:#x}", address);
                true
            }
            Err(e) => {
                tracing::error!("WalletConnect auth failed: {}", e);
                self.base_mut().call_deferred(
                    "_error_getting_wallet",
                    &[format!("WalletConnect auth error: {}", e).to_variant()],
                );
                false
            }
        }
    }

    #[func]
    fn try_recover_account(&mut self, dict: VarDictionary) -> bool {
        let Some(account_address) = dict.get("account_address") else {
            return false;
        };
        let Some(chain_id) = dict.get("chain_id") else {
            return false;
        };
        let Some(ephemeral_auth_chain_str) = dict.get("ephemeral_auth_chain") else {
            return false;
        };
        let local_wallet = dict
            .get("local_wallet")
            .unwrap_or(PackedByteArray::new().to_variant());

        let Some(account_address) = account_address.to_string().as_h160() else {
            return false;
        };
        let Ok(chain_id) = chain_id.try_to::<u64>() else {
            return false;
        };
        let Ok(ephemeral_auth_chain) = serde_json::from_str::<EphemeralAuthChain>(
            ephemeral_auth_chain_str.to_string().as_str(),
        ) else {
            return false;
        };
        let Ok(local_wallet_bytes) = local_wallet.try_to::<PackedByteArray>() else {
            return false;
        };

        if ephemeral_auth_chain.expired() {
            return false;
        }

        if !local_wallet_bytes.is_empty() {
            self._update_local_wallet(local_wallet_bytes.as_slice(), ephemeral_auth_chain);
            true
        } else {
            self._update_remote_wallet(account_address, chain_id, ephemeral_auth_chain);
            // Rehydrate the thirdweb-guest marker across cold starts: the
            // recovered remote wallet doesn't record how it was minted, so we
            // match the recovered address against the persisted guest session.
            // Matching addresses ⇒ this remote wallet IS the thirdweb guest.
            // A mismatch (e.g. the user upgraded to a real WalletConnect wallet
            // since) leaves the marker `false`, avoiding a false positive.
            if let Some(session) = thirdweb_guest::load_session_from_disk() {
                if session.wallet_address == account_address {
                    self.is_thirdweb_guest = true;
                }
            }
            true
        }
    }

    #[func]
    fn get_recover_account_to(&self, mut dict: VarDictionary) -> bool {
        if self.wallet.is_none() || self.ephemeral_auth_chain.is_none() {
            return false;
        }

        let chain_id = match &self.wallet {
            Some(CurrentWallet::Remote(wallet)) => wallet.chain_id(),
            _ => 1,
        };

        if let Some(CurrentWallet::Local { wallet: _, keys }) = &self.wallet {
            let _ = dict.insert(
                "local_wallet",
                PackedByteArray::from_iter(keys.iter().cloned()).to_variant(),
            );
        }

        let _ = dict.insert("account_address", self.get_address_str().to_variant());
        let _ = dict.insert("chain_id", chain_id.to_variant());
        let _ = dict.insert(
            "ephemeral_auth_chain",
            serde_json::to_string(&self.ephemeral_auth_chain.as_ref().unwrap())
                .expect("serialize ephemeral auth chain")
                .to_variant(),
        );

        true
    }

    #[func]
    pub fn get_profile_or_null(&self) -> Option<Gd<DclUserProfile>> {
        self.profile.clone()
    }

    #[func]
    pub fn set_default_profile(&mut self) {
        let mut profile = UserProfile::default();
        profile.content.user_id = Some(self.get_address_str().to_string());
        profile.content.eth_address = self.get_address_str().to_string();
        let profile = DclUserProfile::from_gd(profile);
        self.profile = Some(profile.clone());
        tracing::info!("profile > set default profile",);

        self.base_mut().call_deferred(
            "emit_signal",
            &["profile_changed".to_variant(), profile.to_variant()],
        );
    }

    #[func]
    pub fn set_random_profile(&mut self) {
        let mut profile = UserProfile::randomize();
        profile.content.user_id = Some(self.get_address_str().to_string());
        profile.content.eth_address = self.get_address_str().to_string();
        let profile = DclUserProfile::from_gd(profile);
        self.profile = Some(profile.clone());
        tracing::info!("profile > set random profile",);

        self.base_mut().call_deferred(
            "emit_signal",
            &["profile_changed".to_variant(), profile.to_variant()],
        );
    }

    #[func]
    pub fn set_profile(&mut self, profile: Gd<DclUserProfile>) {
        self.profile = Some(profile.clone());
        tracing::info!("profile > set profile func",);

        self.base_mut().call_deferred(
            "emit_signal",
            &["profile_changed".to_variant(), profile.to_variant()],
        );
    }

    /// GDScript-callable logout used by the sign-out flow to fully forget the
    /// current identity (issue #1658). A bare `logout()` can't be called from
    /// GDScript because the name is taken by the `logout` signal, and the
    /// internal `logout()` fn is comms-only — so expose an explicitly-named
    /// entry point. Clears wallet/ephemeral/profile and emits `logout`.
    #[func]
    pub fn clear_identity(&mut self) {
        self.logout();
    }

    #[func]
    pub fn get_address_str(&self) -> GString {
        match self.try_get_address() {
            Some(address) => GString::from(&format!("{:#x}", address)),
            None => "".into(),
        }
    }

    #[func]
    pub fn async_get_ephemeral_auth_chain(&self) -> Gd<Promise> {
        let promise = Promise::new_alloc();

        if let Some(ephemeral_auth_chain) = &self.ephemeral_auth_chain {
            let auth_chain_str =
                serde_json::to_string(ephemeral_auth_chain).unwrap_or_else(|_| "{}".to_string());
            let mut promise_clone = promise.clone();
            promise_clone
                .bind_mut()
                .resolve_with_data(auth_chain_str.to_variant());
        } else {
            let mut promise_clone = promise.clone();
            promise_clone
                .bind_mut()
                .reject("No ephemeral auth chain available".into());
        }

        promise
    }

    pub fn get_ephemeral_auth_chain(&self) -> Option<&EphemeralAuthChain> {
        self.ephemeral_auth_chain.as_ref()
    }

    #[func]
    pub fn async_get_identity_headers(
        &self,
        uri: GString,
        metadata: GString,
        method: GString,
    ) -> Gd<Promise> {
        let promise = Promise::new_alloc();
        let promise_instance_id = promise.instance_id();

        // Check ephemeral auth chain before spawning
        let Some(ephemeral_auth_chain) = self.ephemeral_auth_chain.clone() else {
            tracing::error!("ephemeral auth chain not initialized");
            let mut promise_clone = promise.clone();
            promise_clone
                .bind_mut()
                .reject("Ephemeral auth chain not initialized".into());
            return promise;
        };

        if let Some(handle) = TokioRuntime::static_clone_handle() {
            let uri = match http::Uri::try_from(uri.to_string().as_str()) {
                Ok(uri) => uri,
                Err(e) => {
                    tracing::error!("Invalid URI: {}", e);
                    let mut promise_clone = promise.clone();
                    promise_clone
                        .bind_mut()
                        .reject(GString::from(&format!("Invalid URI: {}", e)));
                    return promise;
                }
            };

            let method = method.to_string();
            let metadata = metadata.to_string();

            handle.spawn(async move {
                // Parse metadata from string to JSON value
                let metadata_json = if metadata.is_empty() {
                    serde_json::Value::Null
                } else {
                    match serde_json::from_str(&metadata) {
                        Ok(json) => json,
                        Err(e) => {
                            tracing::error!("Failed to parse metadata as JSON: {}", e);
                            let Ok(mut promise) =
                                Gd::<Promise>::try_from_instance_id(promise_instance_id)
                            else {
                                tracing::error!("error getting promise");
                                return;
                            };
                            promise
                                .bind_mut()
                                .reject(GString::from(&format!("Invalid metadata JSON: {}", e)));
                            return;
                        }
                    }
                };

                let headers = super::wallet::sign_request(
                    method.as_str(),
                    &uri,
                    &ephemeral_auth_chain,
                    metadata_json,
                )
                .await;

                let mut dict = VarDictionary::default();
                for (key, value) in headers {
                    dict.set(key, value);
                }

                let Ok(mut promise) = Gd::<Promise>::try_from_instance_id(promise_instance_id)
                else {
                    tracing::error!("error getting promise");
                    return;
                };

                promise.bind_mut().resolve_with_data(dict.to_variant());
            });
        } else {
            let mut promise_clone = promise.clone();
            promise_clone
                .bind_mut()
                .reject("Tokio runtime not initialized".into());
        }

        promise
    }

    #[func]
    fn _update_profile_from_lambda(&mut self, response: Gd<RequestResponse>) -> bool {
        let base_url = DclGlobal::singleton()
            .bind()
            .get_realm()
            .bind()
            .get_profile_content_url()
            .to_string();

        let request_response = response.bind();

        match UserProfile::from_lambda_response(&request_response, base_url.as_str()) {
            Ok(profile) => {
                // Never replace the local profile with an OLDER copy. Every realm change
                // re-fetches the profile (PlayerIdentity._on_realm_changed) and the lambda
                // can still serve the pre-deploy version right after the user saved an
                // avatar change — applying it would revert the just-equipped wearables
                // (#2489). Mirrors the peer-profile version guard in message_processor.
                // Returns true: a stale response is handled, not a parse failure (false
                // would make the GDScript caller reset to the default profile).
                if let Some(current) = self.profile.as_ref() {
                    let current_version = current.bind().inner.version;
                    if profile.version < current_version {
                        tracing::warn!(
                            "profile > ignoring stale lambda profile v{} (local is v{})",
                            profile.version,
                            current_version
                        );
                        return true;
                    }
                }

                let new_profile = DclUserProfile::from_gd(profile);
                self.profile = Some(new_profile.clone());
                tracing::info!("profile > set profile from lambda",);

                self.base_mut().call_deferred(
                    "emit_signal",
                    &["profile_changed".to_variant(), new_profile.to_variant()],
                );
                true
            }
            Err(e) => {
                tracing::error!("error updating profile {:?}", e);
                false
            }
        }
    }
}

impl DclPlayerIdentity {
    pub fn try_get_ephemeral_auth_chain(&self) -> Option<EphemeralAuthChain> {
        self.ephemeral_auth_chain.clone()
    }

    pub fn clone_profile(&self) -> Option<UserProfile> {
        self.profile.as_ref().map(|v| v.bind().inner.clone())
    }

    pub fn try_get_address(&self) -> Option<H160> {
        match &self.wallet {
            Some(CurrentWallet::Remote(wallet)) => Some(wallet.address()),
            Some(CurrentWallet::Local { wallet, keys: _ }) => Some(wallet.address()),
            None => None,
        }
    }

    pub fn get_address(&self) -> H160 {
        self.try_get_address().expect("wallet not initialized")
    }

    // is not exposed to godot, because it should only be called by comms
    pub fn logout(&mut self) {
        if self.try_get_address().is_none() {
            return;
        }

        self.wallet = None;
        self.ephemeral_auth_chain = None;
        self.profile = None;
        self.is_thirdweb_guest = false;
        self.is_thirdweb_guest_upgraded = false;
        // The next account starts from a clean sign-in slate: no pending browser hop, and no
        // cancel from the previous session left to refuse its deep link.
        self.pending_mobile_auth = None;
        self.mobile_auth_cancelled = false;
        self.mobile_auth_started = false;
        self.base_mut()
            .call_deferred("emit_signal", &["logout".to_variant()]);
    }

    pub fn send_async(
        &self,
        mut body: CreateRequest,
        response: RpcResultSender<Result<serde_json::Value, String>>,
    ) {
        let url_sender = DclGlobal::singleton()
            .bind()
            .get_dcl_tokio_rpc()
            .bind()
            .get_sender();
        let Some(auth_chain) = self.ephemeral_auth_chain.clone() else {
            return;
        };
        body.auth_chain = Some(auth_chain.auth_chain().clone());

        if let Some(handle) = TokioRuntime::static_clone_handle() {
            handle.spawn(async move {
                let result = do_request(body, url_sender).await.map(|(_, result)| result);
                response.send(result.map_err(|err| err.to_string()));
            });
        }
    }
}

/// Runs the silent guest-login flow end-to-end:
///   1. resolve the device anchor (native value or desktop fallback)
///   2. hash anchor → opaque thirdweb `sessionId`
///   3. POST /v1/auth/complete (guest) → wallet address + bearer token
///   4. mint a local ephemeral keypair + Decentraland delegation message
///   5. POST /v1/wallets/sign-message → external signature
///   6. assemble the EphemeralAuthChain
///
/// On success, the returned `EphemeralAuthChain` is signed by the thirdweb
/// wallet and delegates request signing to the local ephemeral key for the
/// usual ~30 day window. The thirdweb JWT itself is dropped after step 5 —
/// every cold start re-runs steps 1–6 (idempotent because the anchor is
/// stable, so thirdweb returns the same wallet address).
/// Successful outcome of the guest-login flow. Carries `is_new_user` so
/// analytics can tell a freshly minted wallet apart from a returning anchor
/// re-logging into its existing wallet ("wallets created" = new mints only).
struct GuestLoginOutcome {
    address: H160,
    chain: EphemeralAuthChain,
    is_new_user: bool,
}

async fn perform_thirdweb_guest_login(
    device_anchor_id: String,
) -> Result<GuestLoginOutcome, thirdweb_guest::GuestLoginError> {
    let anchor = device_anchor::resolve_anchor(&device_anchor_id);
    let session_id = device_anchor::compute_session_id(&anchor);

    let session = thirdweb_guest::guest_login(&session_id).await?;

    let (ephemeral_message, ephemeral_keys, expiration) = generate_ephemeral_for_signing();

    // The wallet already exists server-side at this point; a signing failure
    // still fails the login, but under its own reason so the dashboard doesn't
    // misattribute it to wallet creation.
    let signature_hex = thirdweb_guest::sign_message(
        &session.token,
        session.wallet_address,
        1,
        &ephemeral_message,
    )
    .await
    .map_err(|e| thirdweb_guest::GuestLoginError {
        reason: "sign_message",
        http_status: None,
        message: e.to_string(),
    })?;

    let signer_address_str = format!("{:#x}", session.wallet_address);
    let chain = create_ephemeral_from_external_signature(
        &signer_address_str,
        &signature_hex,
        &ephemeral_keys,
        expiration,
        &ephemeral_message,
    )
    .map_err(|e| thirdweb_guest::GuestLoginError {
        reason: "ephemeral",
        http_status: None,
        message: e.to_string(),
    })?;

    // Persist the JWT so future cold starts can renew the ephemeral
    // delegation without re-running `guest_login`. Failure is non-fatal:
    // we already have a valid in-memory ephemeral chain for this session.
    if let Err(e) = thirdweb_guest::save_session_to_disk(&session) {
        tracing::warn!(
            "thirdweb: failed to persist session to disk (non-fatal): {}",
            e
        );
    }

    Ok(GuestLoginOutcome {
        address: session.wallet_address,
        chain,
        is_new_user: session.is_new_user,
    })
}

/// Runs the "Upgrade to OTP" link end-to-end:
///   1. verify the OTP → EMAIL identity JWT (`email_complete`)
///   2. obtain a FRESH guest JWT — re-run `guest_login` from the anchor so the
///      `/link` bearer is guaranteed non-expired (the persisted one may have
///      aged out); fall back to the persisted token if the refresh fails
///   3. `link_email` merges the email into the guest user, preserving address
///   4. persist the refreshed session so the disk copy stays current
///
/// Returns the guest wallet address, which is unchanged by linking.
async fn perform_link_email(
    device_anchor_id: String,
    email: String,
    code: String,
) -> Result<H160, anyhow::Error> {
    let (email_jwt, _email_address) = thirdweb_guest::email_complete(&email, &code).await?;

    // Prefer a freshly minted guest session (idempotent: same anchor → same
    // wallet → fresh token). Fall back to the persisted token only if the
    // refresh round-trip fails (e.g. transient network), since the disk token
    // may be expired.
    let session = match thirdweb_guest::refresh_guest_session(&device_anchor_id).await {
        Ok(session) => session,
        Err(refresh_err) => {
            tracing::warn!(
                "thirdweb: guest session refresh failed ({}); falling back to persisted token",
                refresh_err
            );
            thirdweb_guest::load_session_from_disk().ok_or_else(|| {
                anyhow::anyhow!(
                    "no guest session available to authorize link (refresh failed: {})",
                    refresh_err
                )
            })?
        }
    };

    thirdweb_guest::link_email(&session.token, &email_jwt).await?;

    // Keep the disk copy current with the (possibly refreshed) token. The
    // address is unchanged by linking, so the rehydration match still holds.
    if let Err(e) = thirdweb_guest::save_session_to_disk(&session) {
        tracing::warn!(
            "thirdweb: failed to persist refreshed session after link (non-fatal): {}",
            e
        );
    }

    Ok(session.wallet_address)
}

/// Native email login — verifies the OTP and mints a DCL ephemeral auth chain
/// signed by the email identity's own wallet:
///   1. `email_complete` → email JWT + email wallet address
///   2. mint a local ephemeral keypair + Decentraland delegation message
///   3. `sign_message` with the email JWT to sign the delegation
///   4. assemble the EphemeralAuthChain
async fn perform_email_login(
    email: String,
    code: String,
) -> Result<(H160, EphemeralAuthChain), anyhow::Error> {
    let (email_jwt, email_address) = thirdweb_guest::email_complete(&email, &code).await?;

    let (ephemeral_message, ephemeral_keys, expiration) = generate_ephemeral_for_signing();

    let signature_hex =
        thirdweb_guest::sign_message(&email_jwt, email_address, 1, &ephemeral_message).await?;

    let signer_address_str = format!("{:#x}", email_address);
    let chain = create_ephemeral_from_external_signature(
        &signer_address_str,
        &signature_hex,
        &ephemeral_keys,
        expiration,
        &ephemeral_message,
    )?;

    Ok((email_address, chain))
}

/// Deletes the guest account server-side (issue #2335):
///   1. re-derive a FRESH guest JWT from the anchor (the persisted one may be
///      expired) — idempotent: same anchor → same wallet → fresh token
///   2. `unlink_guest_profile` removes the sole `guest` identity, deleting the
///      thirdweb user so the sessionId is freed for a brand-new wallet
async fn perform_delete_guest_account(device_anchor_id: String) -> Result<(), anyhow::Error> {
    let session = thirdweb_guest::refresh_guest_session(&device_anchor_id).await?;
    thirdweb_guest::unlink_guest_profile(&session.token).await
}

/// Fully deletes an UPGRADED guest account server-side (enable-upgraded-deletion):
///   1. re-derive a FRESH guest JWT from the anchor (a guest login on an
///      upgraded account returns that same user's token)
///   2. `unlink_upgraded_account` unlinks BOTH the `guest` and the `email` (plus
///      any other) profile, deleting the whole thirdweb user so the account is
///      gone and the sessionId is freed for a brand-new wallet.
async fn perform_delete_upgraded_account(device_anchor_id: String) -> Result<(), anyhow::Error> {
    let session = thirdweb_guest::refresh_guest_session(&device_anchor_id).await?;
    thirdweb_guest::unlink_upgraded_account(&session.token).await
}
