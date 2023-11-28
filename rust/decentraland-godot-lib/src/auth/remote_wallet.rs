use std::fmt;

use async_trait::async_trait;
use ethers::{signers::WalletError, types::H160};

use super::{
    auth_identity::try_create_ephemeral,
    ephemeral_auth_chain::EphemeralAuthChain,
    wallet::ObjSafeWalletSigner,
    with_browser_and_server::{get_account, remote_sign_message, RemoteReportState},
};

#[derive(Clone)]
pub struct RemoteWallet {
    address: H160,
    report_url_sender: tokio::sync::mpsc::Sender<RemoteReportState>,
    chain_id: u64,
}

impl fmt::Debug for RemoteWallet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RemoteWallet")
            .field("address", &self.address)
            .field("chain_id", &self.chain_id)
            .finish()
    }
}

impl RemoteWallet {
    pub async fn with_auth_identity(
        report_url_sender: tokio::sync::mpsc::Sender<RemoteReportState>,
    ) -> Result<(Self, EphemeralAuthChain), ()> {
        let (ephemeral_wallet, chain_id) = try_create_ephemeral(report_url_sender.clone()).await?;

        Ok((
            Self {
                address: ephemeral_wallet.signer(),
                report_url_sender,
                chain_id,
            },
            ephemeral_wallet,
        ))
    }

    pub async fn with(
        report_url_sender: tokio::sync::mpsc::Sender<RemoteReportState>,
    ) -> Result<Self, ()> {
        let (address, chain_id) = get_account(report_url_sender.clone()).await?;

        Ok(Self {
            address,
            report_url_sender,
            chain_id,
        })
    }

    pub fn new(
        address: H160,
        chain_id: u64,
        report_url_sender: tokio::sync::mpsc::Sender<RemoteReportState>,
    ) -> Self {
        Self {
            address,
            report_url_sender,
            chain_id,
        }
    }

    pub fn address(&self) -> H160 {
        self.address
    }

    pub fn chain_id(&self) -> u64 {
        self.chain_id
    }
}

#[async_trait]
impl ObjSafeWalletSigner for RemoteWallet {
    async fn sign_message(&self, message: &[u8]) -> Result<ethers::types::Signature, WalletError> {
        let (_, signature, _chain_id) =
            remote_sign_message(message, Some(self.address), self.report_url_sender.clone())
                .await
                .map_err(|_| WalletError::Eip712Error("Unknown error".to_owned()))?;

        Ok(signature)
    }

    async fn sign_transaction(
        &self,
        _message: &ethers::types::transaction::eip2718::TypedTransaction,
    ) -> Result<ethers::types::Signature, WalletError> {
        Err(WalletError::Eip712Error("Not implemented".to_owned()))
    }

    fn address(&self) -> ethers::types::Address {
        self.address
    }

    fn chain_id(&self) -> u64 {
        self.chain_id
    }
}
#[cfg(test)]
mod test {
    use super::*;
    use tracing_test::traced_test;

    #[traced_test]
    #[tokio::test]
    async fn test_get_remote_wallet() {
        let (sx, _rx) = tokio::sync::mpsc::channel(100);
        let Ok(remote_wallet) = RemoteWallet::with_auth_identity(sx).await else {
            return;
        };
        tracing::info!("remote_wallet {:?} ", remote_wallet.0);
    }
}
