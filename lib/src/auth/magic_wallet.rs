use async_trait::async_trait;
use ethers_core::types::Signature;
use ethers_core::types::H160;
use ethers_signers::WalletError;
use std::fmt;
use std::str::FromStr;

use crate::godot_classes::dcl_tokio_rpc::GodotTokioCall;

use super::magic_auth_identity::try_create_magic_link_ephemeral;
use super::{ephemeral_auth_chain::EphemeralAuthChain, wallet::ObjSafeWalletSigner};

#[derive(Clone)]
pub struct MagicWallet {
    address: H160,
    chain_id: u64,
    sender: tokio::sync::mpsc::Sender<GodotTokioCall>,
}

impl fmt::Debug for MagicWallet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MagicWallet")
            .field("address", &self.address)
            .field("chain_id", &self.chain_id)
            .finish()
    }
}

impl MagicWallet {
    pub async fn with_auth_identity(
        sender: tokio::sync::mpsc::Sender<GodotTokioCall>,
    ) -> Result<(Self, EphemeralAuthChain), anyhow::Error> {
        let (ephemeral_wallet, chain_id) = try_create_magic_link_ephemeral(sender.clone()).await?;

        Ok((
            Self {
                address: ephemeral_wallet.signer(),
                chain_id,
                sender,
            },
            ephemeral_wallet,
        ))
    }

    pub fn address(&self) -> H160 {
        self.address
    }

    pub fn chain_id(&self) -> u64 {
        self.chain_id
    }
}

#[async_trait]
impl ObjSafeWalletSigner for MagicWallet {
    async fn sign_message(
        &self,
        message: &[u8],
    ) -> Result<ethers_core::types::Signature, WalletError> {
        let (sx, rx) = tokio::sync::oneshot::channel::<(String, String)>();

        self.sender
            .send(GodotTokioCall::MagicSignMessage {
                message: String::from_utf8_lossy(message).to_string(),
                response: sx.into(),
            })
            .await
            .map_err(|e| WalletError::Eip712Error(e.to_string()))?;

        let (_, signature) = rx
            .await
            .map_err(|e| WalletError::Eip712Error(e.to_string()))?;

        let signature = Signature::from_str(signature.as_str())
            .map_err(|e| WalletError::Eip712Error(e.to_string()))?;

        Ok(signature)
    }

    async fn sign_transaction(
        &self,
        _message: &ethers_core::types::transaction::eip2718::TypedTransaction,
    ) -> Result<ethers_core::types::Signature, WalletError> {
        Err(WalletError::Eip712Error("Not implemented".to_owned()))
    }

    fn address(&self) -> ethers_core::types::Address {
        self.address
    }

    fn chain_id(&self) -> u64 {
        self.chain_id
    }
}
