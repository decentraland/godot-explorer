use std::sync::Arc;

use async_trait::async_trait;

use ethers_core::types::{transaction::eip2718::TypedTransaction, Address, Signature, H160};
use ethers_core::utils::hex;
use ethers_signers::{LocalWallet, Signer, WalletError};

use http::Uri;
use rand::thread_rng;
use serde::{Deserialize, Serialize};

use super::ephemeral_auth_chain::EphemeralAuthChain;
use super::remote_wallet::RemoteWallet;

pub enum WalletType {
    Local(LocalWallet),
    Remote(RemoteWallet),
}

#[derive(Clone)]
pub struct Wallet {
    inner: Arc<WalletType>,
}

impl Wallet {
    pub async fn sign_message<S: Send + Sync + AsRef<[u8]>>(
        &self,
        message: S,
    ) -> Result<Signature, WalletError> {        
        match &*self.inner {
            WalletType::Local(wallet) => Signer::sign_message(wallet, message).await,
            WalletType::Remote(_) => Err(WalletError::Eip712Error("Not implemented".to_owned())),
        }
    }

    pub fn address(&self) -> Address {
        match &*self.inner {
            WalletType::Local(wallet) => wallet.address(),
            WalletType::Remote(wallet) => wallet.address(),
        }
    }

    pub fn new_local_wallet() -> Self {
        Self {
            inner: Arc::new(WalletType::Local(LocalWallet::new(&mut thread_rng()))),
        }
    }

    pub fn new_from_inner(inner: WalletType) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}


#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SimpleAuthChain(Vec<ChainLink>);

impl SimpleAuthChain {
    pub fn new(signer_address: Address, payload: String, signature: Signature) -> Self {
        Self(vec![
            ChainLink {
                ty: "SIGNER".to_owned(),
                payload: format!("{signer_address:#x}"),
                signature: String::default(),
            },
            ChainLink {
                ty: "ECDSA_SIGNED_ENTITY".to_owned(),
                payload,
                signature: format!("0x{signature}"),
            },
        ])
    }

    pub fn new_ephemeral_identity_auth_chain(
        signer_address: Address,
        ephemeral_message: String,
        signature: Signature,
    ) -> Self {
        const PERSONAL_SIGNATURE_LENGTH: usize = 132;
        let first_signature = format!("0x{signature}");
        let auth_chain_type = if first_signature.len() == PERSONAL_SIGNATURE_LENGTH {
            "ECDSA_EPHEMERAL"
        } else {
            "ECDSA_EIP_1654_EPHEMERAL"
        };
        Self(vec![
            ChainLink {
                ty: "SIGNER".to_owned(),
                payload: format!("{signer_address:#x}"),
                signature: String::default(),
            },
            ChainLink {
                ty: auth_chain_type.to_owned(),
                payload: ephemeral_message,
                signature: first_signature,
            },
        ])
    }

    pub fn add_signed_entity(&mut self, payload: String, signature: Signature) {
        self.0.push(ChainLink {
            ty: "ECDSA_SIGNED_ENTITY".to_owned(),
            payload,
            signature: format!("0x{signature}"),
        });
    }

    pub fn headers(&self) -> impl Iterator<Item = (String, String)> + '_ {
        self.0.iter().enumerate().map(|(ix, link)| {
            (
                format!("x-identity-auth-chain-{}", ix),
                serde_json::to_string(&link).unwrap(),
            )
        })
    }

    pub fn formdata(&self) -> impl Iterator<Item = (String, String)> + '_ {
        self.0.iter().enumerate().flat_map(|(ix, link)| {
            [
                (format!("authChain[{ix}][type]"), link.ty.clone()),
                (format!("authChain[{ix}][payload]"), link.payload.clone()),
                (
                    format!("authChain[{ix}][signature]"),
                    link.signature.clone(),
                ),
            ]
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChainLink {
    #[serde(rename = "type")]
    ty: String,
    payload: String,
    signature: String,
}

// convert string -> Address
pub trait AsH160 {
    fn as_h160(&self) -> Option<H160>;
}

impl AsH160 for &str {
    fn as_h160(&self) -> Option<H160> {
        if self.starts_with("0x") {
            return (&self[2..]).as_h160();
        }

        let Ok(hex_bytes) = hex::decode(self.as_bytes()) else {
            return None;
        };
        if hex_bytes.len() != H160::len_bytes() {
            return None;
        }

        Some(H160::from_slice(hex_bytes.as_slice()))
    }
}

impl AsH160 for String {
    fn as_h160(&self) -> Option<H160> {
        self.as_str().as_h160()
    }
}

pub async fn sign_request<META: Serialize>(
    method: &str,
    uri: &Uri,
    wallet: &EphemeralAuthChain,
    meta: META,
) -> Vec<(String, String)> {
    let unix_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let meta = serde_json::to_string(&meta).unwrap();
    let payload = format!("{}:{}:{}:{}", method, uri.path(), unix_time, meta).to_lowercase();

    let signature = wallet
        .ephemeral_wallet()
        .sign_message(&payload)
        .await
        .expect("signature by ephemeral should always work");
    let mut auth_chain = wallet.auth_chain().clone();
    auth_chain.add_signed_entity(payload, signature);

    let mut headers: Vec<_> = auth_chain.headers().collect();
    headers.push(("x-identity-timestamp".to_owned(), format!("{}", unix_time)));
    headers.push(("x-identity-metadata".to_owned(), meta));
    headers
}
