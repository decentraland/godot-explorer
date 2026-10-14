//! Address to public profile, from the catalyst lambdas.

use std::time::Duration;

use async_trait::async_trait;
use ethers_core::types::H160;
use serde::Deserialize;

use super::types::PublicProfile;
use crate::urls;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

#[async_trait]
pub trait ProfileSource: Send + Sync {
    /// `None` when the address is not a valid one or the profile cannot be fetched.
    async fn public_profile(&self, address: &str) -> Option<PublicProfile>;
}

#[derive(Deserialize)]
struct LambdaProfiles {
    #[serde(default)]
    avatars: Vec<LambdaProfile>,
}

#[derive(Deserialize)]
struct LambdaProfile {
    #[serde(default)]
    name: String,
    #[serde(default, rename = "hasClaimedName")]
    has_claimed_name: Option<bool>,
    #[serde(default)]
    avatar: LambdaAvatar,
}

#[derive(Default, Deserialize)]
struct LambdaAvatar {
    snapshots: Option<LambdaSnapshots>,
}

#[derive(Deserialize)]
struct LambdaSnapshots {
    #[serde(default)]
    face256: String,
}

fn parse(json: &str) -> Option<PublicProfile> {
    let profile = serde_json::from_str::<LambdaProfiles>(json)
        .ok()?
        .avatars
        .into_iter()
        .next()?;
    Some(PublicProfile {
        name: profile.name,
        has_claimed_name: profile.has_claimed_name.unwrap_or(false),
        face_url: profile
            .avatar
            .snapshots
            .map(|s| s.face256)
            .unwrap_or_default(),
    })
}

fn profile_url(lambdas: &str, address: H160) -> String {
    format!("{}/profiles/{:#x}", lambdas.trim_end_matches('/'), address)
}

pub struct LambdaProfileSource {
    client: reqwest::Client,
}

impl LambdaProfileSource {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent(crate::USER_AGENT)
                .timeout(REQUEST_TIMEOUT)
                .build()
                .expect("failed to build reqwest client"),
        }
    }
}

impl Default for LambdaProfileSource {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProfileSource for LambdaProfileSource {
    async fn public_profile(&self, address: &str) -> Option<PublicProfile> {
        let address = address.parse::<H160>().ok()?;
        let url = profile_url(&urls::peer_lambdas(), address);
        let response = match self.client.get(&url).send().await {
            Ok(r) if r.status().is_success() => r,
            Ok(r) => {
                tracing::debug!("profile {url}: {}", r.status());
                return None;
            }
            Err(e) => {
                tracing::debug!("profile {url}: {e}");
                return None;
            }
        };
        parse(&response.text().await.ok()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_name_claim_and_face() {
        let json = r#"{"avatars":[{"name":"Ana","hasClaimedName":true,
            "avatar":{"snapshots":{"face256":"https://x/face.png","body":"b"}}}]}"#;
        assert_eq!(
            parse(json),
            Some(PublicProfile {
                name: "Ana".into(),
                has_claimed_name: true,
                face_url: "https://x/face.png".into(),
            })
        );
    }

    #[test]
    fn missing_fields_default() {
        let json = r#"{"avatars":[{"name":"Bo"}]}"#;
        assert_eq!(
            parse(json),
            Some(PublicProfile {
                name: "Bo".into(),
                has_claimed_name: false,
                face_url: String::new(),
            })
        );
    }

    #[test]
    fn no_avatar_or_bad_json_is_none() {
        assert_eq!(parse(r#"{"avatars":[]}"#), None);
        assert_eq!(parse("nope"), None);
    }

    #[test]
    fn url_tolerates_both_slash_styles() {
        let a = H160::from_low_u64_be(0x1234);
        assert_eq!(
            profile_url("https://p/lambdas/", a),
            profile_url("https://p/lambdas", a)
        );
        assert!(profile_url("https://p/lambdas/", a)
            .ends_with("/profiles/0x0000000000000000000000000000000000001234"));
    }
}
