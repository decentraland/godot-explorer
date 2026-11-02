use std::{future::Future, sync::Arc, time::Duration};

use ethers_core::types::H160;

use crate::{
    comms::profile::{ProfileNotFound, UserProfile},
    content::profile::{
        request_lambda_profile, request_registry_profile, request_registry_profile_by_url,
    },
    http_request::http_queue_requester::HttpQueueRequester,
};

#[derive(Debug)]
pub(super) struct FetchedProfile {
    pub profile: UserProfile,
    pub cross_environment: bool,
}

fn in_environment(endpoint: &str, environment: &str) -> bool {
    url::Url::parse(endpoint)
        .ok()
        .and_then(|url| url.host_str().map(str::to_owned))
        .is_some_and(|host| {
            host == format!("decentraland.{environment}")
                || host.ends_with(&format!(".decentraland.{environment}"))
        })
}

/// Compares two lambdas endpoints ignoring trailing-slash style — Godot
/// publishes `…/lambdas/` in its LiveKit metadata while Unity publishes
/// `…/lambdas`, and the slash-only mismatch must not make the realm's own
/// endpoint look like a different catalyst.
fn is_same_lambda_endpoint(a: &str, b: &str) -> bool {
    a.trim_end_matches('/') == b.trim_end_matches('/')
}

// Missing profiles are definitive; transport errors and stale versions may be retried.
async fn fetch_with<F, Fut>(
    realm: &str,
    content: &str,
    peer_endpoint: Option<&str>,
    version: u32,
    mut fetch: F,
) -> anyhow::Result<FetchedProfile>
where
    F: FnMut(String, String, bool) -> Fut,
    Fut: Future<Output = anyhow::Result<UserProfile>>,
{
    let zone = in_environment(realm, "zone");
    let mut endpoint = if zone {
        realm
    } else {
        peer_endpoint.unwrap_or(realm)
    };
    let mut result = fetch(endpoint.into(), content.into(), false).await;
    if result.is_err() && !is_same_lambda_endpoint(endpoint, realm) {
        endpoint = realm;
        result = fetch(endpoint.into(), content.into(), false).await;
    }
    for delay in [2, 4] {
        if result
            .as_ref()
            .is_ok_and(|profile| profile.version >= version)
            || result
                .as_ref()
                .is_err_and(|error| error.is::<ProfileNotFound>())
        {
            break;
        }
        tokio::time::sleep(Duration::from_secs(delay)).await;
        result = fetch(endpoint.into(), content.into(), false).await;
    }
    if !result
        .as_ref()
        .is_ok_and(|profile| profile.version >= version)
    {
        let registry = fetch(realm.into(), content.into(), true).await;
        // A failed registry lookup must not discard an available (older) profile,
        // or turn an uncertain network failure into a definitive "No profile".
        if registry.is_ok()
            || result
                .as_ref()
                .is_err_and(|error| error.is::<ProfileNotFound>())
        {
            result = registry;
        }
    }
    if !is_same_lambda_endpoint(endpoint, realm)
        && !result
            .as_ref()
            .is_ok_and(|profile| profile.version >= version)
    {
        let realm_result = fetch(realm.into(), content.into(), false).await;
        if realm_result.is_ok() || result.is_err() {
            result = realm_result;
        }
    }
    if zone
        && result
            .as_ref()
            .is_err_and(|error| error.is::<ProfileNotFound>())
    {
        // Production versions are independent of the version announced in zone.
        let profile = fetch(
            "https://peer.decentraland.org/lambdas/".into(),
            "https://peer.decentraland.org/content/".into(),
            false,
        )
        .await?;
        return Ok(FetchedProfile {
            profile,
            cross_environment: true,
        });
    }
    result.map(|profile| FetchedProfile {
        profile,
        cross_environment: false,
    })
}

pub(super) async fn fetch_profile(
    address: H160,
    realm: &str,
    content: &str,
    peer_endpoint: Option<&str>,
    version: u32,
    requester: Arc<HttpQueueRequester>,
) -> anyhow::Result<FetchedProfile> {
    tokio::time::timeout(
        Duration::from_secs(20),
        fetch_with(
            realm,
            content,
            peer_endpoint,
            version,
            |endpoint, content, registry| {
                let requester = requester.clone();
                async move {
                    let profile = if registry && in_environment(&endpoint, "zone") {
                        request_registry_profile_by_url(
                            address,
                            &content,
                            requester,
                            "https://asset-bundle-registry.decentraland.zone/profiles".into(),
                        )
                        .await?
                    } else if registry {
                        request_registry_profile(address, &content, requester).await?
                    } else {
                        request_lambda_profile(address, &endpoint, &content, requester).await?
                    };
                    anyhow::ensure!(
                        profile.content.eth_address.parse::<H160>().ok() == Some(address),
                        "profile address mismatch"
                    );
                    Ok(profile)
                }
            },
        ),
    )
    .await
    .map_err(|_| anyhow::anyhow!("profile lookup timed out"))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::ready;

    #[test]
    fn lambda_endpoint_comparison_ignores_trailing_slash_style() {
        // Godot metadata carries `…/lambdas/`, Unity metadata carries `…/lambdas` —
        // same catalyst, must compare equal so the realm endpoint isn't treated
        // as a different fetch target.
        assert!(is_same_lambda_endpoint(
            "https://peer.decentraland.org/lambdas",
            "https://peer.decentraland.org/lambdas/"
        ));
        assert!(is_same_lambda_endpoint(
            "https://peer.decentraland.org/lambdas/",
            "https://peer.decentraland.org/lambdas/"
        ));
        assert!(!is_same_lambda_endpoint(
            "https://peer.decentraland.org/lambdas",
            "https://peer-ec2.decentraland.org/lambdas/"
        ));
    }

    fn profile(version: u32) -> UserProfile {
        UserProfile {
            version,
            ..UserProfile::default()
        }
    }

    #[tokio::test]
    async fn stale_peer_profile_still_tries_realm_after_registry_failure() {
        let result = fetch_with(
            "https://peer.decentraland.org/lambdas/",
            "",
            Some("https://peer-other.decentraland.org/lambdas/"),
            2,
            |url, _, registry| {
                ready(if registry {
                    Err(anyhow::anyhow!("registry unavailable"))
                } else if url.contains("peer-other") {
                    Ok(profile(1))
                } else {
                    Ok(profile(2))
                })
            },
        )
        .await
        .unwrap();
        assert_eq!(result.profile.version, 2);
        assert!(!result.cross_environment);
    }

    #[tokio::test]
    async fn zone_wins_even_when_peer_advertises_org() {
        let mut calls = 0;
        let result = fetch_with(
            "https://peer.decentraland.zone/lambdas/",
            "zone/",
            Some("https://peer.decentraland.org/lambdas/"),
            3,
            |url, _, registry| {
                calls += 1;
                assert!(url.contains(".zone/"));
                assert!(!registry);
                ready(Ok(profile(3)))
            },
        )
        .await
        .unwrap();
        assert_eq!(calls, 1);
        assert!(!result.cross_environment);
    }

    #[tokio::test]
    async fn missing_zone_uses_org_with_independent_version_and_content() {
        let mut calls = 0;
        let result = fetch_with(
            "https://peer.decentraland.zone/lambdas/",
            "zone/",
            None,
            900,
            |url, content, _| {
                calls += 1;
                ready(if url.contains(".org/") {
                    assert_eq!(content, "https://peer.decentraland.org/content/");
                    Ok(profile(1))
                } else {
                    Err(ProfileNotFound.into())
                })
            },
        )
        .await
        .unwrap();
        assert_eq!(calls, 3);
        assert!(result.cross_environment);
        assert_eq!(result.profile.version, 1);
    }

    #[tokio::test]
    async fn missing_everywhere_stays_missing_and_does_not_retry() {
        let mut calls = 0;
        let result = fetch_with(
            "https://peer.decentraland.zone/lambdas/",
            "",
            None,
            0,
            |_, _, _| {
                calls += 1;
                ready(Err(ProfileNotFound.into()))
            },
        )
        .await;
        assert!(result.unwrap_err().is::<ProfileNotFound>());
        assert_eq!(calls, 3);
    }

    #[tokio::test]
    async fn production_and_custom_realms_never_cross_environments() {
        for realm in [
            "https://peer.decentraland.org/lambdas/",
            "https://decentraland.zone.evil.test/",
        ] {
            let mut calls = 0;
            let result = fetch_with(realm, "", None, 0, |url, _, _| {
                calls += 1;
                assert_eq!(url, realm);
                ready(Err(ProfileNotFound.into()))
            })
            .await;
            assert!(result.unwrap_err().is::<ProfileNotFound>());
            assert_eq!(calls, 2);
        }
    }

    #[tokio::test]
    async fn uncertain_registry_failure_is_not_missing_or_cross_environment() {
        let result = fetch_with(
            "https://peer.decentraland.zone/lambdas/",
            "",
            None,
            0,
            |url, _, registry| {
                assert!(!url.contains(".org/"));
                ready(if registry {
                    Err(anyhow::anyhow!("network unavailable"))
                } else {
                    Err(ProfileNotFound.into())
                })
            },
        )
        .await;
        assert!(!result.unwrap_err().is::<ProfileNotFound>());
    }
}
