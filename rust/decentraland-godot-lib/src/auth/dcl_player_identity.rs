use ethers::signers::LocalWallet;
use ethers::types::H160;
use godot::prelude::*;
use rand::thread_rng;
use tokio::task::JoinHandle;

use crate::avatars::dcl_user_profile::DclUserProfile;
use crate::comms::profile::{LambdaProfiles, UserProfile};
use crate::content::bytes::fast_create_packed_byte_array_from_vec;
use crate::dcl::scene_apis::RpcResultSender;
use crate::godot_classes::promise::Promise;
use crate::http_request::request_response::{RequestResponse, ResponseEnum};
use crate::scene_runner::tokio_runtime::TokioRuntime;

use super::auth_identity::create_local_ephemeral;
use super::ephemeral_auth_chain::EphemeralAuthChain;
use super::remote_wallet::RemoteWallet;
use super::wallet::{AsH160, Wallet};
use super::with_browser_and_server::{remote_send_async, RPCSendableMessage, RemoteReportState};

enum CurrentWallet {
    Remote(RemoteWallet),
    Local { wallet: Wallet, keys: Vec<u8> },
}

#[derive(GodotClass)]
#[class(base=Node)]
pub struct DclPlayerIdentity {
    wallet: Option<CurrentWallet>,
    ephemeral_auth_chain: Option<EphemeralAuthChain>,

    remote_report_sender: tokio::sync::mpsc::Sender<RemoteReportState>,
    remote_report_receiver: tokio::sync::mpsc::Receiver<RemoteReportState>,

    profile: Option<Gd<DclUserProfile>>,

    try_connect_account_handle: Option<JoinHandle<()>>,

    #[var]
    is_guest: bool,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for DclPlayerIdentity {
    fn init(base: Base<Node>) -> Self {
        let (remote_report_sender, remote_report_receiver) = tokio::sync::mpsc::channel(100);

        Self {
            wallet: None,
            ephemeral_auth_chain: None,
            remote_report_receiver,
            remote_report_sender,
            profile: None,
            base,
            is_guest: false,
            try_connect_account_handle: None,
        }
    }

    fn process(&mut self, _dt: f64) {
        while let Ok(state) = self.remote_report_receiver.try_recv() {
            match state {
                RemoteReportState::OpenUrl { url, description } => {
                    self.base.call_deferred(
                        "emit_signal".into(),
                        &[
                            "need_open_url".to_variant(),
                            url.to_variant(),
                            description.to_variant(),
                        ],
                    );
                }
            }
        }
    }
}

#[godot_api]
impl DclPlayerIdentity {
    #[signal]
    fn need_open_url(&self, url: GString, description: GString);

    #[signal]
    fn logout(&self);

    #[signal]
    fn wallet_connected(&self, address: GString, chain_id: u64, is_guest: bool);

    #[signal]
    fn profile_changed(&self, new_profile: Gd<DclUserProfile>);

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
                self.base.call_deferred(
                    "_error_getting_wallet".into(),
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
            self.remote_report_sender.clone(),
        )));
        self.ephemeral_auth_chain = Some(ephemeral_auth_chain);

        let address = self.get_address();
        self.base.call_deferred(
            "emit_signal".into(),
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

        self.base.call_deferred(
            "emit_signal".into(),
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

        let instance_id = self.base.instance_id();
        let sender = self.remote_report_sender.clone();
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
                        "try_set_remote_wallet".into(),
                        &[
                            format!("{:#x}", wallet.address()).to_variant(),
                            wallet.chain_id().to_variant(),
                            ephemeral_auth_chain_json_str.to_variant(),
                        ],
                    );
                }
                Err(err) => {
                    tracing::error!("error getting wallet {:?}", err);
                    this.call_deferred(
                        "_error_getting_wallet".into(),
                        &["Unknown error".to_variant()],
                    );
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
    }

    #[func]
    fn try_recover_account(&mut self, dict: Dictionary) -> bool {
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

        if !local_wallet_bytes.is_empty() {
            self._update_local_wallet(local_wallet_bytes.as_slice(), ephemeral_auth_chain);
            true
        } else {
            self._update_remote_wallet(account_address, chain_id, ephemeral_auth_chain);
            true
        }
    }

    #[func]
    fn get_recover_account_to(&self, mut dict: Dictionary) -> bool {
        if self.wallet.is_none() || self.ephemeral_auth_chain.is_none() {
            return false;
        }

        let chain_id = match &self.wallet {
            Some(CurrentWallet::Remote(wallet)) => wallet.chain_id(),
            _ => 1,
        };

        if let Some(CurrentWallet::Local { wallet: _, keys }) = &self.wallet {
            dict.insert(
                "local_wallet",
                PackedByteArray::from_iter(keys.iter().cloned()).to_variant(),
            );
        }

        dict.insert("account_address", self.get_address_str().to_variant());
        dict.insert("chain_id", chain_id.to_variant());
        dict.insert(
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

        self.base.call_deferred(
            "emit_signal".into(),
            &["profile_changed".to_variant(), profile.to_variant()],
        );
    }

    #[func]
    pub fn set_profile(&mut self, profile: Gd<DclUserProfile>) {
        self.profile = Some(profile.clone());

        self.base.call_deferred(
            "emit_signal".into(),
            &["profile_changed".to_variant(), profile.to_variant()],
        );
    }

    #[func]
    pub fn get_address_str(&self) -> GString {
        match self.try_get_address() {
            Some(address) => format!("{:#x}", address).into(),
            None => "".into(),
        }
    }

    #[func]
    pub fn async_get_identity_headers(
        &self,
        uri: GString,
        metadata: GString,
        method: GString,
    ) -> Gd<Promise> {
        let promise = Promise::new_gd();
        let promise_instance_id = promise.instance_id();

        if let Some(handle) = TokioRuntime::static_clone_handle() {
            let ephemeral_auth_chain = self
                .ephemeral_auth_chain
                .as_ref()
                .expect("ephemeral auth chain not initialized")
                .clone();

            let uri = http::Uri::try_from(uri.to_string().as_str()).expect("Invalid url");
            let method = method.to_string();
            let metadata = metadata.to_string();

            handle.spawn(async move {
                let headers = super::wallet::sign_request(
                    method.as_str(),
                    &uri,
                    &ephemeral_auth_chain,
                    metadata,
                )
                .await;

                let mut dict = Dictionary::default();
                for (key, value) in headers {
                    dict.set(key.to_godot(), value.to_godot());
                }

                let Ok(mut promise) = Gd::<Promise>::try_from_instance_id(promise_instance_id)
                else {
                    tracing::error!("error getting promise");
                    return;
                };

                promise.bind_mut().resolve_with_data(dict.to_variant());
            });
        }

        promise
    }

    #[func]
    pub fn async_prepare_deploy_profile(&self, new_profile: Gd<DclUserProfile>) -> Gd<Promise> {
        let promise = Promise::new_gd();
        let promise_instance_id = promise.instance_id();

        let current_profile = if let Some(profile) = self.profile.clone() {
            profile
        } else {
            DclUserProfile::from_gd(UserProfile {
                version: 0,
                ..Default::default()
            })
        };

        let mut new_user_profile = new_profile.bind().inner.clone();
        let eth_address = self.get_address_str().to_string();
        new_user_profile.version = current_profile.bind().inner.version + 1;
        new_user_profile.content.version = new_user_profile.version as i64;
        new_user_profile.content.user_id = Some(eth_address.clone());
        new_user_profile.content.eth_address = eth_address;

        if let Some(handle) = TokioRuntime::static_clone_handle() {
            let ephemeral_auth_chain = self
                .ephemeral_auth_chain
                .as_ref()
                .expect("ephemeral auth chain not initialized")
                .clone();
            handle.spawn(async move {
                let deploy_data = super::deploy_profile::prepare_deploy_profile(
                    ephemeral_auth_chain.clone(),
                    new_user_profile,
                )
                .await;

                let Ok(mut promise) = Gd::<Promise>::try_from_instance_id(promise_instance_id)
                else {
                    tracing::error!("error getting promise");
                    return;
                };

                let Ok((content_type, body_payload)) = deploy_data else {
                    promise
                        .bind_mut()
                        .reject("error preparing deploy profile".into());
                    return;
                };

                let body_payload = fast_create_packed_byte_array_from_vec(&body_payload);
                let mut dict = Dictionary::default();
                dict.set("content_type", content_type.to_variant());
                dict.set("body_payload", body_payload.to_variant());

                promise.bind_mut().resolve_with_data(dict.to_variant());
            });
        }

        promise
    }

    #[func]
    fn _update_profile_from_lambda(&mut self, response: Gd<RequestResponse>) -> bool {
        match &response.bind().response_data {
            Ok(ResponseEnum::String(json)) => {
                if let Ok(response) = serde_json::from_str::<LambdaProfiles>(json.as_str()) {
                    let Some(mut content) = response.avatars.into_iter().next() else {
                        tracing::error!("error parsing lambda response");
                        return false;
                    };

                    // clean up the lambda result
                    if let Some(snapshots) = content.avatar.snapshots.as_mut() {
                        if let Some(hash) = snapshots
                            .body
                            .rsplit_once('/')
                            .map(|(_, hash)| hash.to_owned())
                        {
                            snapshots.body = hash;
                        }
                        if let Some(hash) = snapshots
                            .face256
                            .rsplit_once('/')
                            .map(|(_, hash)| hash.to_owned())
                        {
                            snapshots.face256 = hash;
                        }
                    }

                    let new_profile = DclUserProfile::from_gd(UserProfile {
                        version: content.version as u32,
                        content,
                        base_url: "https://peer.decentraland.org/content/contents/".to_owned(),
                    });
                    self.profile = Some(new_profile.clone());

                    self.base.call_deferred(
                        "emit_signal".into(),
                        &["profile_changed".to_variant(), new_profile.to_variant()],
                    );

                    return true;
                } else {
                    tracing::error!("error parsing lambda response");
                }
            }
            Err(e) => {
                tracing::error!("error updating profile {:?}", e);
            }
            _ => {
                tracing::error!("error updating profile");
            }
        }
        false
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
        self.base
            .call_deferred("emit_signal".into(), &["logout".to_variant()]);
    }

    pub fn send_async(
        &self,
        body: RPCSendableMessage,
        response: RpcResultSender<Result<serde_json::Value, String>>,
    ) {
        let url_sender = self.remote_report_sender.clone();
        if let Some(handle) = TokioRuntime::static_clone_handle() {
            handle.spawn(async move {
                let result = remote_send_async(body, None, url_sender).await;
                response.send(result.map_err(|err| err.to_string()));
            });
        }
    }
}
