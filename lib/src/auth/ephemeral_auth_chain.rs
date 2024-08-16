use std::fmt;

use ethers_core::types::H160;
use ethers_signers::LocalWallet;
use serde::{
    de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor},
    ser::SerializeStruct,
    Serialize,
};

use super::wallet::{SimpleAuthChain, Wallet};

pub struct EphemeralAuthChain {
    signer: H160,

    ephemeral_wallet: Wallet,
    ephemeral_keys: Vec<u8>,
    expiration: std::time::SystemTime,

    auth_chain: SimpleAuthChain,
}

impl EphemeralAuthChain {
    pub fn new(
        signer: H160,
        ephemeral_keys: Vec<u8>,
        auth_chain: SimpleAuthChain,
        expiration: std::time::SystemTime,
    ) -> Self {
        Self {
            signer,
            ephemeral_wallet: Wallet::new_from_inner(Box::new(
                LocalWallet::from_bytes(&ephemeral_keys).unwrap(),
            )),
            ephemeral_keys,
            auth_chain,
            expiration,
        }
    }

    pub fn ephemeral_wallet(&self) -> &Wallet {
        &self.ephemeral_wallet
    }

    pub fn signer(&self) -> H160 {
        self.signer
    }

    pub fn expiration(&self) -> std::time::SystemTime {
        self.expiration
    }

    pub fn auth_chain(&self) -> &SimpleAuthChain {
        &self.auth_chain
    }
}

impl Clone for EphemeralAuthChain {
    fn clone(&self) -> Self {
        Self::new(
            self.signer,
            self.ephemeral_keys.clone(),
            self.auth_chain.clone(),
            self.expiration,
        )
    }
}

impl Serialize for EphemeralAuthChain {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("ephemeral_auth_chain", 4)?;
        state.serialize_field("signer", &self.signer)?;
        state.serialize_field("ephemeral_keys", &self.ephemeral_keys)?;
        state.serialize_field("auth_chain", &self.auth_chain)?;
        state.serialize_field("expiration", &self.expiration)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for EphemeralAuthChain {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field {
            Signer,
            EphemeralKeys,
            AuthChain,
            Expiration,
        }
        const FIELDS: &[&str] = &["signer", "ephemeral_keys", "auth_chain", "expiration"];

        struct EphemeralAuthChainVisitor;

        impl<'de> Visitor<'de> for EphemeralAuthChainVisitor {
            type Value = EphemeralAuthChain;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct EphemeralAuthChain")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<EphemeralAuthChain, V::Error>
            where
                V: SeqAccess<'de>,
            {
                let signer = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let ephemeral_keys = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                let auth_chain = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(2, &self))?;
                let expiration = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(3, &self))?;

                Ok(EphemeralAuthChain::new(
                    signer,
                    ephemeral_keys,
                    auth_chain,
                    expiration,
                ))
            }

            fn visit_map<V>(self, mut map: V) -> Result<EphemeralAuthChain, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut signer = None;
                let mut ephemeral_keys = None;
                let mut auth_chain = None;
                let mut expiration = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Signer => {
                            if signer.is_some() {
                                return Err(de::Error::duplicate_field("signer"));
                            }
                            signer = Some(map.next_value()?);
                        }
                        Field::EphemeralKeys => {
                            if ephemeral_keys.is_some() {
                                return Err(de::Error::duplicate_field("ephemeral_keys"));
                            }
                            ephemeral_keys = Some(map.next_value()?);
                        }
                        Field::AuthChain => {
                            if auth_chain.is_some() {
                                return Err(de::Error::duplicate_field("auth_chain"));
                            }
                            auth_chain = Some(map.next_value()?);
                        }
                        Field::Expiration => {
                            if expiration.is_some() {
                                return Err(de::Error::duplicate_field("expiration"));
                            }
                            expiration = Some(map.next_value()?);
                        }
                    }
                }
                let signer = signer.ok_or_else(|| de::Error::missing_field("signer"))?;
                let ephemeral_keys =
                    ephemeral_keys.ok_or_else(|| de::Error::missing_field("ephemeral_keys"))?;
                let auth_chain =
                    auth_chain.ok_or_else(|| de::Error::missing_field("auth_chain"))?;
                let expiration =
                    expiration.ok_or_else(|| de::Error::missing_field("expiration"))?;

                Ok(EphemeralAuthChain::new(
                    signer,
                    ephemeral_keys,
                    auth_chain,
                    expiration,
                ))
            }
        }

        deserializer.deserialize_struct("ephemeral_auth_chain", FIELDS, EphemeralAuthChainVisitor)
    }
}
