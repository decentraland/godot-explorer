use std::sync::Arc;

use async_trait::async_trait;

use ethers_core::types::{transaction::eip2718::TypedTransaction, Address, Signature, H160};
use ethers_core::utils::hex;
use ethers_signers::{LocalWallet, Signer, WalletError};

use http::Uri;
use rand::thread_rng;
use serde::{Deserialize, Serialize};

use super::ephemeral_auth_chain::EphemeralAuthChain;
#[derive(Clone)]
pub struct Wallet {
    inner: Arc<Box<dyn ObjSafeWalletSigner + 'static + Send + Sync>>,
}

impl Wallet {
    pub async fn sign_message<S: Send + Sync + AsRef<[u8]>>(
        &self,
        message: S,
    ) -> Result<Signature, WalletError> {
        self.inner.sign_message(message.as_ref()).await
    }

    pub fn address(&self) -> Address {
        self.inner.address()
    }

    pub fn new_local_wallet() -> Self {
        Self {
            inner: Arc::new(Box::new(LocalWallet::new(&mut thread_rng()))),
        }
    }

    pub fn new_from_inner(inner: Box<dyn ObjSafeWalletSigner + 'static + Send + Sync>) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[async_trait]
pub trait ObjSafeWalletSigner {
    async fn sign_message(&self, message: &[u8]) -> Result<Signature, WalletError>;

    /// Signs the transaction
    async fn sign_transaction(&self, message: &TypedTransaction) -> Result<Signature, WalletError>;

    /// Returns the signer's Ethereum Address
    fn address(&self) -> Address;

    /// Returns the signer's chain id
    fn chain_id(&self) -> u64;
}

#[async_trait]
impl ObjSafeWalletSigner for LocalWallet {
    async fn sign_message(&self, message: &[u8]) -> Result<Signature, WalletError> {
        Signer::sign_message(self, message).await
    }

    async fn sign_transaction(&self, message: &TypedTransaction) -> Result<Signature, WalletError> {
        Signer::sign_transaction(self, message).await
    }

    fn address(&self) -> Address {
        Signer::address(self)
    }

    fn chain_id(&self) -> u64 {
        Signer::chain_id(self)
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

    /// Creates an ephemeral identity auth chain using a pre-formatted signature hex string.
    /// Used for WalletConnect where we need to preserve the original v=27/28 format
    /// instead of ethers' normalized v=0/1 format.
    pub fn new_ephemeral_identity_auth_chain_from_hex(
        signer_address: Address,
        ephemeral_message: String,
        signature_hex: String,
    ) -> Self {
        const PERSONAL_SIGNATURE_LENGTH: usize = 132;
        let auth_chain_type = if signature_hex.len() == PERSONAL_SIGNATURE_LENGTH {
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
                signature: signature_hex,
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

    /// Creates a SimpleAuthChain from a vector of ChainLinks.
    /// Used when receiving auth chain from external sources (e.g., mobile auth flow).
    pub fn from_chain_links(links: Vec<ChainLink>) -> Self {
        Self(links)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChainLink {
    #[serde(rename = "type")]
    pub ty: String,
    pub payload: String,
    pub signature: String,
}

impl ChainLink {
    pub fn new(ty: String, payload: String, signature: String) -> Self {
        Self {
            ty,
            payload,
            signature,
        }
    }
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

/// Sign the Pulse ENet handshake payload and pack the auth chain as the JSON `x-identity-*`
/// header dictionary the server expects inside `HandshakeRequest.auth_chain` — identical in shape
/// to the HTTP signed-fetch headers (`sign_request`), just carried as protobuf bytes.
///
/// The payload is the signed-fetch string for `connect` on path `/`:
/// `connect:/:{timestamp_ms}:{}` — deliberately NOT lowercased: the server
/// (`HandshakeHandler`/`SignedFetch.BuildSignedFetchPayload`) verifies the signature over this
/// exact string, and Unity/bevy sign it verbatim. (It happens to be all-lowercase today; don't
/// couple to that.) Re-sign per attempt: the server enforces a ±60s replay window on the
/// timestamp.
pub async fn sign_pulse_connect(wallet: &EphemeralAuthChain) -> Result<Vec<u8>, String> {
    let unix_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_millis();

    let payload = format!("connect:/:{unix_time}:{{}}");
    let signature = wallet
        .ephemeral_wallet()
        .sign_message(&payload)
        .await
        .map_err(|e| format!("ephemeral sign failed: {e}"))?;
    let mut auth_chain = wallet.auth_chain().clone();
    auth_chain.add_signed_entity(payload, signature);

    let mut dict = serde_json::Map::new();
    for (key, value) in auth_chain.headers() {
        dict.insert(key, serde_json::Value::String(value));
    }
    dict.insert(
        "x-identity-timestamp".to_owned(),
        serde_json::Value::String(unix_time.to_string()),
    );
    dict.insert(
        "x-identity-metadata".to_owned(),
        serde_json::Value::String("{}".to_owned()),
    );
    serde_json::to_vec(&dict).map_err(|e| e.to_string())
}

/// Which bytes `sign_request` signs, and therefore which `@dcl/crypto-middleware`
/// generation accepts the signature. The two disagree on how the payload is built:
///
/// ```text
/// 5.x  [method, path, timestamp, metadata].join(":").toLowerCase()
/// 6.x  [method.toLowerCase(), path.toLowerCase(), timestamp, metadata].join(":")
/// ```
///
/// 6.0.0 stopped folding the metadata, so no single signature satisfies both — unless
/// the metadata is already all-lowercase, in which case the two payloads are identical
/// byte for byte. That is the only lever a client has, and it costs the metadata.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SignedMetadata {
    /// ADR-44: fold the payload, transmit the metadata as serialized. This is what every
    /// other explorer sends and what services read `authMetadata.sceneId` / `.realmName`
    /// back out of, so it is the default. A 6.x service rejects it with 401 whenever the
    /// metadata carries an uppercase character.
    Verbatim,
    /// Fold the metadata too, and transmit it folded so the header still matches what was
    /// signed. The same bytes then verify under both generations. Only use it where the
    /// service does not read the metadata back: folding destroys camelCase keys and any
    /// case-sensitive value.
    Lowercase,
}

pub async fn sign_request<META: Serialize>(
    method: &str,
    uri: &Uri,
    wallet: &EphemeralAuthChain,
    meta: META,
    metadata_format: SignedMetadata,
) -> Vec<(String, String)> {
    let unix_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();

    // Whatever goes into the signature must also go into `x-identity-metadata`: the server
    // rebuilds the expected payload from that header and compares. Signing `{"productid":...}`
    // while transmitting `{"productId":...}` is what made every 6.x-verified request fail with
    // "Invalid final authority".
    //
    // The joined payload is folded in both modes, which is ADR-44 and costs nothing against a
    // 6.x server either: it lowercases method and path itself, and the timestamp is digits.
    // The metadata is the only component the two generations treat differently, so it is the
    // only thing this mode decides.
    //
    // The HTTP body is never touched here. credits-server answers `productId is required` to a
    // lowercased body, which is how we know metadata and body are independent to the server.
    let meta = serde_json::to_string(&meta).unwrap();
    let meta = match metadata_format {
        SignedMetadata::Verbatim => meta,
        SignedMetadata::Lowercase => meta.to_lowercase(),
    };
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
