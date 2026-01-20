/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Centralized URL management for Decentraland services.
//!
//! This module provides a single source of truth for all Decentraland URLs,
//! following the Unity `DecentralandUrlsSource.cs` pattern. URLs are automatically
//! transformed based on the current environment (org, zone, today).

use std::{
    collections::HashMap,
    sync::{OnceLock, RwLock},
};

use crate::env::{get_environment, DclEnvironment};

/// All Decentraland URL types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DecentralandUrl {
    // Auth
    /// Frontend auth URL: https://decentraland.{ENV}/auth/requests
    AuthFrontend,
    /// Auth API base URL: https://auth-api.decentraland.{ENV}
    AuthApiBase,
    /// Auth API requests endpoint: https://auth-api.decentraland.{ENV}/requests
    AuthApiRequests,

    // Content
    /// Genesis realm provider: https://realm-provider-ea.decentraland.{ENV}/main
    Genesis,
    /// Worlds content server: https://worlds-content-server.decentraland.{ENV}/world/
    WorldsContentServer,
    /// Peer base URL (special: uses peer-testing for zone/today)
    PeerBase,
    /// Peer content server (special: uses peer-testing for zone/today)
    PeerContent,
    /// Peer lambdas server (special: uses peer-testing for zone/today)
    PeerLambdas,

    // Comms
    /// Comms gatekeeper: https://comms-gatekeeper.decentraland.{ENV}/get-scene-adapter
    CommsGatekeeper,
    /// Social service RPC: wss://rpc-social-service-ea.decentraland.{ENV}
    SocialService,
    /// Archipelago stats: https://archipelago-ea-stats.decentraland.{ENV}
    ArchipelagoStats,

    // Web3
    /// Ethereum RPC: wss://rpc.decentraland.{ENV}/mainnet
    EthereumRpc,

    // APIs
    /// Places API: https://places.decentraland.{ENV}/api
    PlacesApi,
    /// Events API: https://events.decentraland.{ENV}/api
    EventsApi,
    /// Notifications API: https://notifications.decentraland.{ENV}
    NotificationsApi,
    /// Mobile BFF: https://mobile-bff.decentraland.{ENV}
    MobileBff,

    // Frontend
    /// Host: https://decentraland.{ENV}
    Host,
    /// Marketplace: https://decentraland.{ENV}/marketplace
    Marketplace,
    /// Marketplace claim name: https://decentraland.{ENV}/marketplace/names/claim
    MarketplaceClaimName,
    /// Privacy policy: https://decentraland.{ENV}/privacy
    PrivacyPolicy,
    /// Terms of service: https://decentraland.{ENV}/terms
    TermsOfService,
    /// Content policy: https://decentraland.{ENV}/content
    ContentPolicy,
    /// Jump events: https://decentraland.{ENV}/jump/events
    JumpEvents,
    /// Account deletion: https://decentraland.{ENV}/account-deletion
    AccountDeletion,

    // Proxies
    /// OpenSea proxy: https://opensea.decentraland.{ENV}
    OpenSeaProxy,

    // CDN (fixed, not transformed)
    /// Adaptation layer (fixed): https://sdk-team-cdn.decentraland.org/ipfs
    AdaptationLayer,

    // Origin (for signed fetch headers - always .org)
    /// Origin for signed fetch: https://decentraland.org
    Origin,
}

/// URL transformation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UrlTransformType {
    /// Standard transformation: decentraland.org -> decentraland.{env}
    Standard,
    /// Peer transformation: peer.decentraland.org -> peer-testing.decentraland.org for zone/today
    Peer,
    /// Fixed: no transformation
    Fixed,
}

/// Base URLs with transformation type
struct UrlTemplate {
    template: &'static str,
    transform_type: UrlTransformType,
}

impl DecentralandUrl {
    fn template(&self) -> UrlTemplate {
        match self {
            // Auth
            Self::AuthFrontend => UrlTemplate {
                template: "https://decentraland.org/auth/requests",
                transform_type: UrlTransformType::Standard,
            },
            Self::AuthApiBase => UrlTemplate {
                template: "https://auth-api.decentraland.org",
                transform_type: UrlTransformType::Standard,
            },
            Self::AuthApiRequests => UrlTemplate {
                template: "https://auth-api.decentraland.org/requests",
                transform_type: UrlTransformType::Standard,
            },

            // Content
            Self::Genesis => UrlTemplate {
                template: "https://realm-provider-ea.decentraland.org/main",
                transform_type: UrlTransformType::Standard,
            },
            Self::WorldsContentServer => UrlTemplate {
                template: "https://worlds-content-server.decentraland.org/world/",
                transform_type: UrlTransformType::Standard,
            },
            Self::PeerBase => UrlTemplate {
                template: "https://peer.decentraland.org",
                transform_type: UrlTransformType::Peer,
            },
            Self::PeerContent => UrlTemplate {
                template: "https://peer.decentraland.org/content/",
                transform_type: UrlTransformType::Peer,
            },
            Self::PeerLambdas => UrlTemplate {
                template: "https://peer.decentraland.org/lambdas/",
                transform_type: UrlTransformType::Peer,
            },

            // Comms
            Self::CommsGatekeeper => UrlTemplate {
                template: "https://comms-gatekeeper.decentraland.org/get-scene-adapter",
                transform_type: UrlTransformType::Standard,
            },
            Self::SocialService => UrlTemplate {
                template: "wss://rpc-social-service-ea.decentraland.org",
                transform_type: UrlTransformType::Standard,
            },
            Self::ArchipelagoStats => UrlTemplate {
                template: "https://archipelago-ea-stats.decentraland.org",
                transform_type: UrlTransformType::Standard,
            },

            // Web3
            Self::EthereumRpc => UrlTemplate {
                template: "wss://rpc.decentraland.org/mainnet",
                transform_type: UrlTransformType::Standard,
            },

            // APIs
            Self::PlacesApi => UrlTemplate {
                template: "https://places.decentraland.org/api",
                transform_type: UrlTransformType::Standard,
            },
            Self::EventsApi => UrlTemplate {
                template: "https://events.decentraland.org/api",
                transform_type: UrlTransformType::Standard,
            },
            Self::NotificationsApi => UrlTemplate {
                template: "https://notifications.decentraland.org",
                transform_type: UrlTransformType::Standard,
            },
            Self::MobileBff => UrlTemplate {
                template: "https://mobile-bff.decentraland.org",
                transform_type: UrlTransformType::Standard,
            },

            // Frontend
            Self::Host => UrlTemplate {
                template: "https://decentraland.org",
                transform_type: UrlTransformType::Standard,
            },
            Self::Marketplace => UrlTemplate {
                template: "https://decentraland.org/marketplace",
                transform_type: UrlTransformType::Standard,
            },
            Self::MarketplaceClaimName => UrlTemplate {
                template: "https://decentraland.org/marketplace/names/claim",
                transform_type: UrlTransformType::Standard,
            },
            Self::PrivacyPolicy => UrlTemplate {
                template: "https://decentraland.org/privacy",
                transform_type: UrlTransformType::Standard,
            },
            Self::TermsOfService => UrlTemplate {
                template: "https://decentraland.org/terms",
                transform_type: UrlTransformType::Standard,
            },
            Self::ContentPolicy => UrlTemplate {
                template: "https://decentraland.org/content",
                transform_type: UrlTransformType::Standard,
            },
            Self::JumpEvents => UrlTemplate {
                template: "https://decentraland.org/jump/events",
                transform_type: UrlTransformType::Standard,
            },
            Self::AccountDeletion => UrlTemplate {
                template: "https://decentraland.org/account-deletion",
                transform_type: UrlTransformType::Standard,
            },

            // Proxies
            Self::OpenSeaProxy => UrlTemplate {
                template: "https://opensea.decentraland.org",
                transform_type: UrlTransformType::Standard,
            },

            // CDN (fixed, not transformed)
            Self::AdaptationLayer => UrlTemplate {
                template: "https://sdk-team-cdn.decentraland.org/ipfs",
                transform_type: UrlTransformType::Fixed,
            },

            // Origin (always .org for signed fetch)
            Self::Origin => UrlTemplate {
                template: "https://decentraland.org",
                transform_type: UrlTransformType::Fixed,
            },
        }
    }
}

/// URL cache for transformed URLs
struct UrlCache {
    environment: DclEnvironment,
    cache: HashMap<DecentralandUrl, String>,
}

impl UrlCache {
    fn new(environment: DclEnvironment) -> Self {
        Self {
            environment,
            cache: HashMap::new(),
        }
    }
}

static URL_CACHE: OnceLock<RwLock<UrlCache>> = OnceLock::new();

/// Initialize or get the URL cache
fn get_cache() -> &'static RwLock<UrlCache> {
    URL_CACHE.get_or_init(|| RwLock::new(UrlCache::new(get_environment())))
}

/// Transform a standard URL template to use the current environment's domain
fn transform_standard_url(url: &str, env: DclEnvironment) -> String {
    if env == DclEnvironment::Org {
        return url.to_string();
    }

    let suffix = env.suffix();
    url.replace("decentraland.org", &format!("decentraland.{}", suffix))
}

/// Transform a peer URL template for the current environment
/// Production (org): https://peer.decentraland.org
/// Zone/Today: https://peer-testing.decentraland.org
fn transform_peer_url(url: &str, env: DclEnvironment) -> String {
    if env == DclEnvironment::Org {
        return url.to_string();
    }

    // For zone and today environments, use peer-testing
    url.replace("peer.decentraland.org", "peer-testing.decentraland.org")
}

/// Get the URL for a specific URL type
///
/// This function returns the URL transformed for the current environment.
/// Results are cached for performance.
///
/// # Example
/// ```
/// use dclgodot::urls::{get_url, DecentralandUrl};
///
/// let genesis_url = get_url(DecentralandUrl::Genesis);
/// // Returns "https://realm-provider-ea.decentraland.org/main" for production
/// // Returns "https://realm-provider-ea.decentraland.zone/main" for staging
/// ```
pub fn get_url(url_type: DecentralandUrl) -> String {
    let current_env = get_environment();
    let cache = get_cache();

    // Check cache first
    {
        let cache_read = cache.read().unwrap();
        // If environment changed, we need to rebuild cache
        if cache_read.environment == current_env {
            if let Some(cached) = cache_read.cache.get(&url_type) {
                return cached.clone();
            }
        }
    }

    // Generate URL based on transformation type
    let template = url_type.template();
    let url = match template.transform_type {
        UrlTransformType::Fixed => template.template.to_string(),
        UrlTransformType::Standard => transform_standard_url(template.template, current_env),
        UrlTransformType::Peer => transform_peer_url(template.template, current_env),
    };

    // Store in cache
    {
        let mut cache_write = cache.write().unwrap();
        // Check if environment changed and clear cache if so
        if cache_write.environment != current_env {
            cache_write.cache.clear();
            cache_write.environment = current_env;
        }
        cache_write.cache.insert(url_type, url.clone());
    }

    url
}

/// Clear the URL cache (useful when environment changes)
pub fn clear_cache() {
    if let Some(cache) = URL_CACHE.get() {
        let mut cache_write = cache.write().unwrap();
        cache_write.cache.clear();
        cache_write.environment = get_environment();
    }
}

// Convenience functions for common URL types
// These provide a cleaner API for frequently used URLs

/// Get the genesis realm URL
pub fn genesis() -> String {
    get_url(DecentralandUrl::Genesis)
}

/// Get the worlds content server base URL
pub fn worlds_content_server() -> String {
    get_url(DecentralandUrl::WorldsContentServer)
}

/// Get the peer base URL
pub fn peer_base() -> String {
    get_url(DecentralandUrl::PeerBase)
}

/// Get the peer content URL
pub fn peer_content() -> String {
    get_url(DecentralandUrl::PeerContent)
}

/// Get the peer lambdas URL
pub fn peer_lambdas() -> String {
    get_url(DecentralandUrl::PeerLambdas)
}

/// Get the auth frontend URL
pub fn auth_frontend() -> String {
    get_url(DecentralandUrl::AuthFrontend)
}

/// Get the auth API base URL
pub fn auth_api_base() -> String {
    get_url(DecentralandUrl::AuthApiBase)
}

/// Get the auth API requests URL
pub fn auth_api_requests() -> String {
    get_url(DecentralandUrl::AuthApiRequests)
}

/// Get the comms gatekeeper URL
pub fn comms_gatekeeper() -> String {
    get_url(DecentralandUrl::CommsGatekeeper)
}

/// Get the social service URL
pub fn social_service() -> String {
    get_url(DecentralandUrl::SocialService)
}

/// Get the Ethereum RPC URL with optional query parameters
pub fn ethereum_rpc() -> String {
    get_url(DecentralandUrl::EthereumRpc)
}

/// Get the Ethereum RPC URL with project parameter
pub fn ethereum_rpc_with_project(project: &str) -> String {
    format!(
        "{}?project={}",
        get_url(DecentralandUrl::EthereumRpc),
        project
    )
}

/// Get the places API URL
pub fn places_api() -> String {
    get_url(DecentralandUrl::PlacesApi)
}

/// Get the events API URL
pub fn events_api() -> String {
    get_url(DecentralandUrl::EventsApi)
}

/// Get the notifications API URL
pub fn notifications_api() -> String {
    get_url(DecentralandUrl::NotificationsApi)
}

/// Get the host URL
pub fn host() -> String {
    get_url(DecentralandUrl::Host)
}

/// Get the marketplace URL
pub fn marketplace() -> String {
    get_url(DecentralandUrl::Marketplace)
}

/// Get the marketplace claim name URL
pub fn marketplace_claim_name() -> String {
    get_url(DecentralandUrl::MarketplaceClaimName)
}

/// Get the archipelago stats URL
pub fn archipelago_stats() -> String {
    get_url(DecentralandUrl::ArchipelagoStats)
}

/// Get the jump events URL
pub fn jump_events() -> String {
    get_url(DecentralandUrl::JumpEvents)
}

/// Get the OpenSea proxy URL
pub fn opensea_proxy() -> String {
    get_url(DecentralandUrl::OpenSeaProxy)
}

/// Get the adaptation layer URL (SDK Team CDN)
pub fn adaptation_layer() -> String {
    get_url(DecentralandUrl::AdaptationLayer)
}

/// Get the origin URL for signed fetch (always decentraland.org)
pub fn origin() -> String {
    get_url(DecentralandUrl::Origin)
}

/// Get the account deletion URL
pub fn account_deletion() -> String {
    get_url(DecentralandUrl::AccountDeletion)
}

/// Get the mobile BFF URL
pub fn mobile_bff() -> String {
    get_url(DecentralandUrl::MobileBff)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_url_org() {
        let url = transform_standard_url("https://decentraland.org/test", DclEnvironment::Org);
        assert_eq!(url, "https://decentraland.org/test");
    }

    #[test]
    fn test_standard_url_zone() {
        let url = transform_standard_url("https://decentraland.org/test", DclEnvironment::Zone);
        assert_eq!(url, "https://decentraland.zone/test");
    }

    #[test]
    fn test_standard_url_today() {
        let url = transform_standard_url("https://decentraland.org/test", DclEnvironment::Today);
        assert_eq!(url, "https://decentraland.today/test");
    }

    #[test]
    fn test_subdomain_transformation() {
        let url =
            transform_standard_url("https://places.decentraland.org/api", DclEnvironment::Zone);
        assert_eq!(url, "https://places.decentraland.zone/api");
    }

    #[test]
    fn test_wss_transformation() {
        let url = transform_standard_url(
            "wss://rpc-social-service-ea.decentraland.org",
            DclEnvironment::Zone,
        );
        assert_eq!(url, "wss://rpc-social-service-ea.decentraland.zone");
    }

    #[test]
    fn test_peer_url_org() {
        let url = transform_peer_url(
            "https://peer.decentraland.org/content/",
            DclEnvironment::Org,
        );
        assert_eq!(url, "https://peer.decentraland.org/content/");
    }

    #[test]
    fn test_peer_url_zone() {
        let url = transform_peer_url(
            "https://peer.decentraland.org/content/",
            DclEnvironment::Zone,
        );
        assert_eq!(url, "https://peer-testing.decentraland.org/content/");
    }

    #[test]
    fn test_peer_url_today() {
        let url = transform_peer_url(
            "https://peer.decentraland.org/lambdas/",
            DclEnvironment::Today,
        );
        assert_eq!(url, "https://peer-testing.decentraland.org/lambdas/");
    }

    #[test]
    fn test_fixed_urls_not_transformed() {
        // Origin should always be .org
        let template = DecentralandUrl::Origin.template();
        assert_eq!(template.transform_type, UrlTransformType::Fixed);

        // Adaptation layer should always be .org
        let template = DecentralandUrl::AdaptationLayer.template();
        assert_eq!(template.transform_type, UrlTransformType::Fixed);
    }

    #[test]
    fn test_peer_urls_have_peer_transform_type() {
        let peer_base = DecentralandUrl::PeerBase.template();
        assert_eq!(peer_base.transform_type, UrlTransformType::Peer);

        let peer_content = DecentralandUrl::PeerContent.template();
        assert_eq!(peer_content.transform_type, UrlTransformType::Peer);

        let peer_lambdas = DecentralandUrl::PeerLambdas.template();
        assert_eq!(peer_lambdas.transform_type, UrlTransformType::Peer);
    }

    #[test]
    fn test_all_url_types_have_templates() {
        // Ensure all URL types have valid templates
        let all_types = [
            DecentralandUrl::AuthFrontend,
            DecentralandUrl::AuthApiBase,
            DecentralandUrl::AuthApiRequests,
            DecentralandUrl::Genesis,
            DecentralandUrl::WorldsContentServer,
            DecentralandUrl::PeerBase,
            DecentralandUrl::PeerContent,
            DecentralandUrl::PeerLambdas,
            DecentralandUrl::CommsGatekeeper,
            DecentralandUrl::SocialService,
            DecentralandUrl::ArchipelagoStats,
            DecentralandUrl::EthereumRpc,
            DecentralandUrl::PlacesApi,
            DecentralandUrl::EventsApi,
            DecentralandUrl::NotificationsApi,
            DecentralandUrl::MobileBff,
            DecentralandUrl::Host,
            DecentralandUrl::Marketplace,
            DecentralandUrl::MarketplaceClaimName,
            DecentralandUrl::PrivacyPolicy,
            DecentralandUrl::TermsOfService,
            DecentralandUrl::ContentPolicy,
            DecentralandUrl::JumpEvents,
            DecentralandUrl::AccountDeletion,
            DecentralandUrl::OpenSeaProxy,
            DecentralandUrl::AdaptationLayer,
            DecentralandUrl::Origin,
        ];

        for url_type in all_types {
            let template = url_type.template();
            assert!(
                !template.template.is_empty(),
                "URL type {:?} has empty template",
                url_type
            );
            assert!(
                template.template.starts_with("https://")
                    || template.template.starts_with("wss://"),
                "URL type {:?} has invalid protocol",
                url_type
            );
        }
    }
}
