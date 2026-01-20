/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Centralized URL management for Decentraland services.
//!
//! URLs are automatically transformed based on the current environment (org, zone, today).

use crate::env::{get_environment, DclEnvironment};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UrlTransformType {
    /// decentraland.org -> decentraland.{env}
    Standard,
    /// peer.decentraland.org -> peer-testing.decentraland.org for zone/today
    Peer,
    /// No transformation
    Fixed,
}

/// All Decentraland URL types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecentralandUrl {
    // Auth
    AuthFrontend,
    AuthApiBase,
    AuthApiRequests,
    // Content
    Genesis,
    WorldsContentServer,
    PeerBase,
    PeerContent,
    PeerLambdas,
    // Comms
    CommsGatekeeper,
    SocialService,
    ArchipelagoStats,
    // Web3
    EthereumRpc,
    // API
    PlacesApi,
    EventsApi,
    NotificationsApi,
    MobileBff,
    // Frontend
    Host,
    Marketplace,
    MarketplaceClaimName,
    PrivacyPolicy,
    TermsOfService,
    ContentPolicy,
    JumpEvents,
    AccountDeletion,
    // Proxy
    OpenSeaProxy,
    // Fixed
    Origin,
}

impl DecentralandUrl {
    fn template(&self) -> (&'static str, UrlTransformType) {
        use DecentralandUrl::*;
        use UrlTransformType::*;

        match self {
            // Auth
            AuthFrontend => ("https://decentraland.org/auth/requests", Standard),
            AuthApiBase => ("https://auth-api.decentraland.org", Standard),
            AuthApiRequests => ("https://auth-api.decentraland.org/requests", Standard),
            // Content
            Genesis => ("https://realm-provider-ea.decentraland.org/main", Standard),
            WorldsContentServer => (
                "https://worlds-content-server.decentraland.org/world/",
                Standard,
            ),
            PeerBase => ("https://peer.decentraland.org", Peer),
            PeerContent => ("https://peer.decentraland.org/content/", Peer),
            PeerLambdas => ("https://peer.decentraland.org/lambdas/", Peer),
            // Comms
            CommsGatekeeper => (
                "https://comms-gatekeeper.decentraland.org/get-scene-adapter",
                Standard,
            ),
            SocialService => ("wss://rpc-social-service-ea.decentraland.org", Standard),
            ArchipelagoStats => ("https://archipelago-ea-stats.decentraland.org", Standard),
            // Web3
            EthereumRpc => ("wss://rpc.decentraland.org/mainnet", Standard),
            // API
            PlacesApi => ("https://places.decentraland.org/api", Standard),
            EventsApi => ("https://events.decentraland.org/api", Standard),
            NotificationsApi => ("https://notifications.decentraland.org", Standard),
            MobileBff => ("https://mobile-bff.decentraland.org", Standard),
            // Frontend
            Host => ("https://decentraland.org", Standard),
            Marketplace => ("https://decentraland.org/marketplace", Standard),
            MarketplaceClaimName => ("https://decentraland.org/marketplace/names/claim", Standard),
            PrivacyPolicy => ("https://decentraland.org/privacy", Standard),
            TermsOfService => ("https://decentraland.org/terms", Standard),
            ContentPolicy => ("https://decentraland.org/content", Standard),
            JumpEvents => ("https://decentraland.org/jump/events", Standard),
            AccountDeletion => ("https://decentraland.org/account-deletion", Standard),
            // Proxy
            OpenSeaProxy => ("https://opensea.decentraland.org", Standard),
            // Fixed
            Origin => ("https://decentraland.org", Fixed),
        }
    }
}

/// Get the URL for a specific URL type, transformed for the current environment.
pub fn get_url(url_type: DecentralandUrl) -> String {
    let env = get_environment();
    let (template, transform_type) = url_type.template();

    match transform_type {
        UrlTransformType::Fixed => template.to_string(),
        UrlTransformType::Standard => {
            if env == DclEnvironment::Org {
                template.to_string()
            } else {
                template.replace(
                    "decentraland.org",
                    &format!("decentraland.{}", env.suffix()),
                )
            }
        }
        UrlTransformType::Peer => {
            if env == DclEnvironment::Org {
                template.to_string()
            } else {
                template.replace("peer.decentraland.org", "peer-testing.decentraland.org")
            }
        }
    }
}

// Convenience functions
pub fn auth_frontend() -> String {
    get_url(DecentralandUrl::AuthFrontend)
}
pub fn auth_api_base() -> String {
    get_url(DecentralandUrl::AuthApiBase)
}
pub fn auth_api_requests() -> String {
    get_url(DecentralandUrl::AuthApiRequests)
}
pub fn genesis() -> String {
    get_url(DecentralandUrl::Genesis)
}
pub fn worlds_content_server() -> String {
    get_url(DecentralandUrl::WorldsContentServer)
}
pub fn peer_base() -> String {
    get_url(DecentralandUrl::PeerBase)
}
pub fn peer_content() -> String {
    get_url(DecentralandUrl::PeerContent)
}
pub fn peer_lambdas() -> String {
    get_url(DecentralandUrl::PeerLambdas)
}
pub fn comms_gatekeeper() -> String {
    get_url(DecentralandUrl::CommsGatekeeper)
}
pub fn social_service() -> String {
    get_url(DecentralandUrl::SocialService)
}
pub fn archipelago_stats() -> String {
    get_url(DecentralandUrl::ArchipelagoStats)
}
pub fn ethereum_rpc() -> String {
    get_url(DecentralandUrl::EthereumRpc)
}
pub fn places_api() -> String {
    get_url(DecentralandUrl::PlacesApi)
}
pub fn events_api() -> String {
    get_url(DecentralandUrl::EventsApi)
}
pub fn notifications_api() -> String {
    get_url(DecentralandUrl::NotificationsApi)
}
pub fn mobile_bff() -> String {
    get_url(DecentralandUrl::MobileBff)
}
pub fn host() -> String {
    get_url(DecentralandUrl::Host)
}
pub fn marketplace() -> String {
    get_url(DecentralandUrl::Marketplace)
}
pub fn marketplace_claim_name() -> String {
    get_url(DecentralandUrl::MarketplaceClaimName)
}
pub fn privacy_policy() -> String {
    get_url(DecentralandUrl::PrivacyPolicy)
}
pub fn terms_of_service() -> String {
    get_url(DecentralandUrl::TermsOfService)
}
pub fn content_policy() -> String {
    get_url(DecentralandUrl::ContentPolicy)
}
pub fn jump_events() -> String {
    get_url(DecentralandUrl::JumpEvents)
}
pub fn account_deletion() -> String {
    get_url(DecentralandUrl::AccountDeletion)
}
pub fn open_sea_proxy() -> String {
    get_url(DecentralandUrl::OpenSeaProxy)
}
pub fn origin() -> String {
    get_url(DecentralandUrl::Origin)
}

pub fn ethereum_rpc_with_project(project: &str) -> String {
    format!("{}?project={}", ethereum_rpc(), project)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_url_org() {
        let env = DclEnvironment::Org;
        let template = "https://decentraland.org/test";
        let result = if env == DclEnvironment::Org {
            template.to_string()
        } else {
            template.replace(
                "decentraland.org",
                &format!("decentraland.{}", env.suffix()),
            )
        };
        assert_eq!(result, "https://decentraland.org/test");
    }

    #[test]
    fn test_standard_url_zone() {
        let env = DclEnvironment::Zone;
        let template = "https://decentraland.org/test";
        let result = template.replace(
            "decentraland.org",
            &format!("decentraland.{}", env.suffix()),
        );
        assert_eq!(result, "https://decentraland.zone/test");
    }

    #[test]
    fn test_standard_url_today() {
        let env = DclEnvironment::Today;
        let template = "https://decentraland.org/test";
        let result = template.replace(
            "decentraland.org",
            &format!("decentraland.{}", env.suffix()),
        );
        assert_eq!(result, "https://decentraland.today/test");
    }

    #[test]
    fn test_subdomain_transformation() {
        let env = DclEnvironment::Zone;
        let template = "https://places.decentraland.org/api";
        let result = template.replace(
            "decentraland.org",
            &format!("decentraland.{}", env.suffix()),
        );
        assert_eq!(result, "https://places.decentraland.zone/api");
    }

    #[test]
    fn test_peer_url_org() {
        let env = DclEnvironment::Org;
        let template = "https://peer.decentraland.org/content/";
        let result = if env == DclEnvironment::Org {
            template.to_string()
        } else {
            template.replace("peer.decentraland.org", "peer-testing.decentraland.org")
        };
        assert_eq!(result, "https://peer.decentraland.org/content/");
    }

    #[test]
    fn test_peer_url_zone() {
        let template = "https://peer.decentraland.org/content/";
        let result = template.replace("peer.decentraland.org", "peer-testing.decentraland.org");
        assert_eq!(result, "https://peer-testing.decentraland.org/content/");
    }

    #[test]
    fn test_fixed_urls_not_transformed() {
        let (_, transform) = DecentralandUrl::Origin.template();
        assert_eq!(transform, UrlTransformType::Fixed);
    }

    #[test]
    fn test_peer_urls_have_peer_transform_type() {
        assert_eq!(
            DecentralandUrl::PeerBase.template().1,
            UrlTransformType::Peer
        );
        assert_eq!(
            DecentralandUrl::PeerContent.template().1,
            UrlTransformType::Peer
        );
        assert_eq!(
            DecentralandUrl::PeerLambdas.template().1,
            UrlTransformType::Peer
        );
    }

    #[test]
    fn test_all_url_types_have_templates() {
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
            DecentralandUrl::Origin,
        ];

        for url_type in all_types {
            let (template, _) = url_type.template();
            assert!(
                !template.is_empty(),
                "URL type {:?} has empty template",
                url_type
            );
            assert!(
                template.starts_with("https://") || template.starts_with("wss://"),
                "URL type {:?} has invalid protocol",
                url_type
            );
        }
    }

    #[test]
    fn test_convenience_functions_exist() {
        let _ = auth_frontend();
        let _ = auth_api_base();
        let _ = auth_api_requests();
        let _ = genesis();
        let _ = worlds_content_server();
        let _ = peer_base();
        let _ = peer_content();
        let _ = peer_lambdas();
        let _ = comms_gatekeeper();
        let _ = social_service();
        let _ = archipelago_stats();
        let _ = ethereum_rpc();
        let _ = places_api();
        let _ = events_api();
        let _ = notifications_api();
        let _ = mobile_bff();
        let _ = host();
        let _ = marketplace();
        let _ = marketplace_claim_name();
        let _ = privacy_policy();
        let _ = terms_of_service();
        let _ = content_policy();
        let _ = jump_events();
        let _ = account_deletion();
        let _ = open_sea_proxy();
        let _ = origin();
    }
}
