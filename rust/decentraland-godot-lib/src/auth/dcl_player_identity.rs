use ethers::signers::LocalWallet;
use ethers::types::H160;
use godot::prelude::*;
use rand::thread_rng;

use crate::comms::profile::UserProfile;
use crate::scene_runner::tokio_runtime::TokioRuntime;

use super::auth_identity::create_local_ephemeral;
use super::ephemeral_auth_chain::EphemeralAuthChain;
use super::remote_wallet::RemoteWallet;
use super::wallet::{AsH160, Wallet};
use super::with_browser_and_server::RemoteReportState;

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

    profile: UserProfile,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl NodeVirtual for DclPlayerIdentity {
    fn init(base: Base<Node>) -> Self {
        let (remote_report_sender, remote_report_receiver) = tokio::sync::mpsc::channel(100);

        Self {
            wallet: None,
            ephemeral_auth_chain: None,
            remote_report_receiver,
            remote_report_sender,
            profile: UserProfile::default(),
            base,
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
    fn need_open_url(&self, url: GodotString, description: GodotString);

    #[signal]
    fn logout(&self);

    #[signal]
    fn wallet_connected(&self, address: GodotString, chain_id: u64);

    #[signal]
    fn profile_changed(&self, new_profile: Dictionary);

    #[func]
    fn try_set_remote_wallet(
        &mut self,
        address_string: GodotString,
        chain_id: u64,
        ephemeral_auth_chain: GodotString,
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
        self.profile.content.user_id = Some(format!("{:#x}", account_address));

        let address = self.address();
        self.base.call_deferred(
            "emit_signal".into(),
            &[
                "wallet_connected".to_variant(),
                format!("{:#x}", address).to_variant(),
                chain_id.to_variant(),
            ],
        );
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

        let address = format!("{:#x}", self.address());
        self.profile.content.user_id = Some(address.clone());

        self.base.call_deferred(
            "emit_signal".into(),
            &[
                "wallet_connected".to_variant(),
                address.to_variant(),
                1_u64.to_variant(),
            ],
        );
    }

    #[func]
    fn _error_getting_wallet(&mut self, error_str: GodotString) {
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
        handle.spawn(async move {
            let wallet = RemoteWallet::with_auth_identity(sender).await;
            let Some(mut this) = Gd::<DclPlayerIdentity>::try_from_instance_id(instance_id) else {
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
                Err(_) => {
                    this.call_deferred(
                        "_error_getting_wallet".into(),
                        &["Unknown error".to_variant()],
                    );
                }
            }
        });
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

        dict.insert(
            "account_address",
            format!("{:#x}", self.address()).to_variant(),
        );
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
    pub fn get_profile(&self) -> Dictionary {
        self.profile
            .content
            .to_godot_dictionary(&self.profile.base_url)
    }

    #[func]
    pub fn update_profile(&mut self, dict: Dictionary) {
        self.update_profile_from_dictionary(&dict);
    }
}

impl DclPlayerIdentity {
    pub fn try_get_ephemeral_auth_chain(&self) -> Option<EphemeralAuthChain> {
        self.ephemeral_auth_chain.clone()
    }

    pub fn profile(&self) -> &UserProfile {
        &self.profile
    }

    pub fn is_connected(&self) -> bool {
        self.wallet.is_some() && self.ephemeral_auth_chain.is_some()
    }

    pub fn address(&self) -> H160 {
        match &self.wallet {
            Some(CurrentWallet::Remote(wallet)) => wallet.address(),
            Some(CurrentWallet::Local { wallet, keys: _ }) => wallet.address(),
            None => panic!("wallet not initialized"),
        }
    }

    pub fn update_profile_from_dictionary(&mut self, dict: &Dictionary) {
        self.profile.content.copy_from_godot_dictionary(dict);
        self.profile.version += 1;
        self.base.call_deferred(
            "emit_signal".into(),
            &["profile_changed".to_variant(), dict.to_variant()],
        );
    }

    // is not exposed to godot, because it should only be called by comms
    pub fn logout(&mut self) {
        if !self.is_connected() {
            return;
        }

        self.wallet = None;
        self.ephemeral_auth_chain = None;
        self.profile = UserProfile::default();
        self.base
            .call_deferred("emit_signal".into(), &["logout".to_variant()]);
    }
}
