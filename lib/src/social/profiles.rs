//! Resolves the profiles of the addresses the social service returns, in batches, from the
//! asset-bundle-registry (`POST /profiles`), the same source the service used before it stopped
//! returning profiles.

use std::collections::HashMap;
use std::time::Duration;

use anyhow::{anyhow, Result};
use serde::Deserialize;
use serde_json::json;

use crate::urls;

use super::types::FriendProfile;

const REQUEST_TIMEOUT_SECS: u64 = 15;
const MAX_ADDRESSES_PER_REQUEST: usize = 100;

#[derive(Deserialize)]
struct RegistryProfile {
    #[serde(default)]
    avatars: Vec<RegistryAvatar>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegistryAvatar {
    #[serde(default)]
    eth_address: Option<String>,
    #[serde(default)]
    user_id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    unclaimed_name: Option<String>,
    #[serde(default)]
    has_claimed_name: bool,
    #[serde(default)]
    avatar: Option<RegistryAvatarData>,
}

#[derive(Deserialize)]
struct RegistryAvatarData {
    #[serde(default)]
    snapshots: Option<RegistrySnapshots>,
}

#[derive(Deserialize)]
struct RegistrySnapshots {
    #[serde(default)]
    face256: Option<String>,
}

impl RegistryAvatar {
    fn into_friend_profile(self) -> Option<FriendProfile> {
        let address = self.eth_address.or(self.user_id)?.to_lowercase();
        let name = self
            .name
            .filter(|name| !name.is_empty())
            .or(self.unclaimed_name.filter(|name| !name.is_empty()))
            .unwrap_or_else(|| address.clone());
        let profile_picture_url = self
            .avatar
            .and_then(|avatar| avatar.snapshots)
            .and_then(|snapshots| snapshots.face256)
            .unwrap_or_default();

        Some(FriendProfile {
            address,
            name,
            has_claimed_name: self.has_claimed_name,
            profile_picture_url,
        })
    }
}

pub struct ProfileResolver {
    url: String,
    http: reqwest::Client,
}

impl ProfileResolver {
    pub fn new() -> Self {
        Self::with_url(urls::asset_bundle_registry_profiles())
    }

    pub fn with_url(url: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .build()
            .expect("reqwest client with a timeout always builds");

        Self { url, http }
    }

    /// One profile per address, in the same order. An address whose profile can't be resolved
    /// (missing, or the registry failing) gets [`FriendProfile::without_profile`], so a registry
    /// outage degrades names and pictures instead of hiding friends.
    pub async fn resolve(&self, addresses: &[String]) -> Vec<FriendProfile> {
        let mut found = HashMap::new();
        for chunk in addresses.chunks(MAX_ADDRESSES_PER_REQUEST) {
            match self.fetch(chunk).await {
                Ok(profiles) => found.extend(
                    profiles
                        .into_iter()
                        .map(|profile| (profile.address.clone(), profile)),
                ),
                Err(e) => tracing::warn!("Failed to resolve {} profiles: {e}", chunk.len()),
            }
        }

        addresses
            .iter()
            .map(|address| {
                found
                    .get(&address.to_lowercase())
                    .cloned()
                    .unwrap_or_else(|| FriendProfile::without_profile(address.clone()))
            })
            .collect()
    }

    async fn fetch(&self, addresses: &[String]) -> Result<Vec<FriendProfile>> {
        let response = self
            .http
            .post(&self.url)
            .json(&json!({ "ids": addresses }))
            .send()
            .await
            .map_err(|e| anyhow!("POST {} failed: {e}", self.url))?;
        let status = response.status();
        if !status.is_success() {
            return Err(anyhow!("POST {} answered {status}", self.url));
        }

        let profiles: Vec<RegistryProfile> = response
            .json()
            .await
            .map_err(|e| anyhow!("POST {} returned an unexpected body: {e}", self.url))?;
        Ok(profiles
            .into_iter()
            .filter_map(|profile| profile.avatars.into_iter().next())
            .filter_map(RegistryAvatar::into_friend_profile)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_a_friend_profile_from_a_registry_avatar() {
        let profiles: Vec<RegistryProfile> = serde_json::from_str(
            r#"[{"avatars":[{"userId":"0xABC","ethAddress":"0xABC","name":"Abc","hasClaimedName":true,
                "avatar":{"snapshots":{"face256":"https://images/face.png"}}}]}]"#,
        )
        .unwrap();

        let profile = profiles
            .into_iter()
            .next()
            .and_then(|profile| profile.avatars.into_iter().next())
            .and_then(RegistryAvatar::into_friend_profile)
            .unwrap();

        assert_eq!(profile.address, "0xabc");
        assert_eq!(profile.name, "Abc");
        assert!(profile.has_claimed_name);
        assert_eq!(profile.profile_picture_url, "https://images/face.png");
    }

    #[test]
    fn falls_back_to_the_unclaimed_name_and_then_the_address() {
        let unclaimed: RegistryAvatar =
            serde_json::from_str(r#"{"ethAddress":"0xabc","name":"","unclaimedName":"Guest"}"#)
                .unwrap();
        assert_eq!(unclaimed.into_friend_profile().unwrap().name, "Guest");

        let nameless: RegistryAvatar = serde_json::from_str(r#"{"ethAddress":"0xabc"}"#).unwrap();
        let profile = nameless.into_friend_profile().unwrap();
        assert_eq!(profile.name, "0xabc");
        assert_eq!(profile.profile_picture_url, "");
    }
}
