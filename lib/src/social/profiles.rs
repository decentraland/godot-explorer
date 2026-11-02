//! Resolves the profiles of the addresses the social service returns, in batches, from the
//! asset-bundle-registry (`POST /profiles`), the same source the service used before it stopped
//! returning profiles.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use futures_util::future::join_all;
use serde::Deserialize;
use serde_json::json;

use crate::urls;

use super::types::FriendProfile;

const REQUEST_TIMEOUT_SECS: u64 = 15;
const MAX_ADDRESSES_PER_REQUEST: usize = 100;
const CACHE_TTL: Duration = Duration::from_secs(5 * 60);

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
    cache: Mutex<HashMap<String, (FriendProfile, Instant)>>,
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

        Self {
            url,
            http,
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// One profile per address, in the same order. An address whose profile can't be resolved
    /// (missing, or the registry failing) gets [`FriendProfile::without_profile`], so a registry
    /// outage degrades names and pictures instead of hiding friends.
    ///
    /// Resolved profiles are cached for [`CACHE_TTL`], so the periodic list refreshes don't hit
    /// the registry again; the addresses still missing are requested in concurrent chunks.
    pub async fn resolve(&self, addresses: &[String]) -> Vec<FriendProfile> {
        let keys: Vec<String> = addresses.iter().map(|a| a.to_lowercase()).collect();
        let mut found = self.cached(&keys);

        let mut seen = HashSet::new();
        let missing: Vec<String> = keys
            .iter()
            .filter(|key| !found.contains_key(*key) && seen.insert(*key))
            .cloned()
            .collect();

        if !missing.is_empty() {
            let chunks: Vec<&[String]> = missing.chunks(MAX_ADDRESSES_PER_REQUEST).collect();
            let results = join_all(chunks.iter().map(|chunk| self.fetch(chunk))).await;

            let mut fetched = Vec::new();
            for (chunk, result) in chunks.into_iter().zip(results) {
                match result {
                    Ok(profiles) => fetched.extend(profiles),
                    Err(e) => tracing::warn!("Failed to resolve {} profiles: {e}", chunk.len()),
                }
            }
            self.store(&fetched);
            found.extend(
                fetched
                    .into_iter()
                    .map(|profile| (profile.address.clone(), profile)),
            );
        }

        addresses
            .iter()
            .zip(&keys)
            .map(|(address, key)| {
                found
                    .get(key)
                    .cloned()
                    .unwrap_or_else(|| FriendProfile::without_profile(address.clone()))
            })
            .collect()
    }

    fn cached(&self, keys: &[String]) -> HashMap<String, FriendProfile> {
        let cache = self.cache.lock().unwrap();
        keys.iter()
            .filter_map(|key| {
                let (profile, stored_at) = cache.get(key)?;
                (stored_at.elapsed() < CACHE_TTL).then(|| (key.clone(), profile.clone()))
            })
            .collect()
    }

    /// Only resolved profiles are cached, so a miss or a registry failure is retried next time.
    fn store(&self, profiles: &[FriendProfile]) {
        let mut cache = self.cache.lock().unwrap();
        cache.retain(|_, (_, stored_at)| stored_at.elapsed() < CACHE_TTL);
        let now = Instant::now();
        for profile in profiles {
            cache.insert(profile.address.clone(), (profile.clone(), now));
        }
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

    #[tokio::test]
    async fn serves_cached_profiles_and_falls_back_for_the_rest() {
        // Nothing listens there, so whatever is not cached can only fall back.
        let resolver = ProfileResolver::with_url("http://127.0.0.1:1/profiles".to_string());
        resolver.store(&[FriendProfile {
            address: "0xabc".to_string(),
            name: "Abc".to_string(),
            has_claimed_name: true,
            profile_picture_url: "https://images/face.png".to_string(),
        }]);

        let profiles = resolver
            .resolve(&["0xDEF".to_string(), "0xABC".to_string()])
            .await;

        assert_eq!(profiles[0].address, "0xDEF");
        assert_eq!(profiles[0].name, "0xDEF");
        assert_eq!(profiles[1].name, "Abc");
    }
}
