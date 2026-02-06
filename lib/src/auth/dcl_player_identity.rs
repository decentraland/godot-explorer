use ethers_core::types::H160;
use ethers_signers::LocalWallet;
use godot::prelude::*;
use rand::thread_rng;
use std::time::UNIX_EPOCH;
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
use super::ephemeral_auth_chain::EphemeralAuthChain;
use super::remote_wallet::RemoteWallet;
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
    /// and complete_mobile_connect_account (when deep link arrives)
    pending_mobile_auth: Option<()>,

    #[var]
    is_guest: bool,

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
            try_connect_account_handle: None,
            pending_mobile_auth: None,
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
        self.profile = None;
    }

    #[func]
    fn _error_getting_wallet(&mut self, error_str: GString) {
        tracing::error!("error getting wallet {:?}", error_str);
        self.base_mut()
            .emit_signal("auth_error", &[error_str.to_variant()]);
    }

    #[func]
    fn create_guest_account(&mut self) {
        let local_wallet = LocalWallet::new(&mut thread_rng());
        let local_wallet_bytes = local_wallet.signer().to_bytes().to_vec();
        let ephemeral_auth_chain = create_local_ephemeral(&local_wallet);
        self._update_local_wallet(local_wallet_bytes.as_slice(), ephemeral_auth_chain);
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
    #[func]
    fn complete_mobile_connect_account(&mut self, identity_id: GString) {
        if self.pending_mobile_auth.take().is_none() {
            tracing::error!("No pending mobile auth to complete");
            self.base_mut().call_deferred(
                "_error_getting_wallet",
                &["No pending mobile auth".to_variant()],
            );
            return;
        };

        let Some(handle) = TokioRuntime::static_clone_handle() else {
            panic!("tokio runtime not initialized")
        };

        let instance_id = self.base().instance_id();
        let identity_id = identity_id.to_string();

        handle.spawn(async move {
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
                        "try_set_remote_wallet",
                        &[
                            format!("{:#x}", address).to_variant(),
                            chain_id.to_variant(),
                            ephemeral_auth_chain_json_str.to_variant(),
                        ],
                    );
                }
                Err(err) => {
                    tracing::error!("Error completing mobile auth: {:?}", err);
                    this.call_deferred(
                        "_error_getting_wallet",
                        &[format!("Mobile auth completion error: {}", err).to_variant()],
                    );
                }
            }
        });
    }

    /// Returns true if there's a pending mobile auth waiting for deep link
    #[func]
    fn has_pending_mobile_auth(&self) -> bool {
        self.pending_mobile_auth.is_some()
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
    ) -> bool {
        let expiration = std::time::SystemTime::UNIX_EPOCH
            + std::time::Duration::from_secs(expiration_timestamp as u64);

        match create_ephemeral_from_external_signature(
            &signer_address.to_string(),
            &signature.to_string(),
            ephemeral_private_key.as_slice(),
            expiration,
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
