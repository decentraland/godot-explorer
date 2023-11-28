use ethers::types::H160;
use godot::prelude::*;

use crate::comms::profile::UserProfile;
use crate::scene_runner::tokio_runtime::TokioRuntime;

use super::ephemeral_auth_chain::EphemeralAuthChain;
use super::remote_wallet::RemoteWallet;
use super::wallet::AsH160;
use super::with_browser_and_server::RemoteReportState;

#[derive(GodotClass)]
#[class(base=Node)]
pub struct DclPlayerIdentity {
    remote_wallet: Option<RemoteWallet>,
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
            remote_wallet: None,
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
                    self.base.emit_signal(
                        "need_open_url".into(),
                        &[url.to_variant(), description.to_variant()],
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
    fn wallet_connected(&self, address: GodotString, chain_id: u64);

    #[signal]
    fn profile_changed(&self, address: GodotString, chain_id: u64);

    #[func]
    fn try_set_wallet(
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
            Err(_e) => {
                tracing::error!(
                    "invalid data ephemeral_auth_chain {:?}",
                    ephemeral_auth_chain
                );
                self.base.call_deferred(
                    "_error_getting_wallet".into(),
                    &["Error parsing ephemeral_auth_chain".to_variant()],
                );
                return false;
            }
        };

        self._update_wallet(address, chain_id, ephemeral_auth_chain);
        true
    }

    fn _update_wallet(
        &mut self,
        account_address: H160,
        chain_id: u64,
        ephemeral_auth_chain: EphemeralAuthChain,
    ) {
        self.remote_wallet = Some(RemoteWallet::new(
            account_address,
            chain_id,
            self.remote_report_sender.clone(),
        ));
        self.ephemeral_auth_chain = Some(ephemeral_auth_chain);
        self.profile.content.user_id = Some(format!("{:#x}", account_address));
        self.base.emit_signal(
            "wallet_connected".into(),
            &[
                format!("{:#x}", self.remote_wallet.as_ref().unwrap().address()).to_variant(),
                chain_id.to_variant(),
            ],
        );
    }

    #[func]
    fn _error_getting_wallet(&mut self, error_str: GodotString) {}

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
                        "try_set_wallet".into(),
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

        self._update_wallet(account_address, chain_id, ephemeral_auth_chain);
        true
    }

    #[func]
    fn get_recover_account_to(&self, mut dict: Dictionary) -> bool {
        if self.remote_wallet.is_none() || self.ephemeral_auth_chain.is_none() {
            return false;
        }
        let remote_wallet = self.remote_wallet.as_ref().unwrap();
        dict.insert(
            "account_address",
            format!("{:#x}", self.remote_wallet.as_ref().unwrap().address()).to_variant(),
        );
        dict.insert("chain_id", remote_wallet.chain_id().to_variant());
        dict.insert(
            "ephemeral_auth_chain",
            serde_json::to_string(&self.ephemeral_auth_chain.as_ref().unwrap())
                .expect("serialize ephemeral auth chain")
                .to_variant(),
        );
        true
    }
}

impl DclPlayerIdentity {
    pub fn try_get_wallet(&self) -> Option<RemoteWallet> {
        self.remote_wallet.clone()
    }

    pub fn try_get_ephemeral_auth_chain(&self) -> Option<EphemeralAuthChain> {
        self.ephemeral_auth_chain.clone()
    }

    pub fn profile(&self) -> &UserProfile {
        &self.profile
    }

    pub fn update_profile_from_dictionary(&mut self, dict: &Dictionary) {
        self.profile.content.copy_from_godot_dictionary(dict);
        self.profile.version += 1;
    }
}
