//! Centralized URL management for Decentraland services.
//!
//! URLs are automatically transformed based on the current environment (org, zone, today).
//! Per-service-group overrides allow fine-grained control via the dclenv parameter.

use crate::env::{self, DclEnvironment, ServiceGroup};

/// Domain suffix for a service group (respects per-group overrides).
fn suffix(group: ServiceGroup) -> &'static str {
    env::suffix_for(group)
}

/// Resolved environment for a service group.
fn resolved_env(group: ServiceGroup) -> DclEnvironment {
    env::env_for(group)
}

/// Domain suffix for the default environment (ungrouped URLs).
fn default_suffix() -> &'static str {
    env::get_environment().suffix()
}

// Auth
pub fn auth_frontend() -> String {
    if resolved_env(ServiceGroup::Auth) == DclEnvironment::Today {
        "http://localhost:5173/auth/requests".to_string()
    } else {
        format!(
            "https://decentraland.{}/auth/requests",
            suffix(ServiceGroup::Auth)
        )
    }
}
pub fn auth_mobile_frontend() -> String {
    if resolved_env(ServiceGroup::Auth) == DclEnvironment::Today {
        "http://localhost:5173/auth/mobile".to_string()
    } else {
        format!(
            "https://decentraland.{}/auth/mobile",
            suffix(ServiceGroup::Auth)
        )
    }
}
pub fn auth_api_base() -> String {
    if resolved_env(ServiceGroup::Auth) == DclEnvironment::Today {
        "https://auth-api.decentraland.zone".to_string()
    } else {
        format!(
            "https://auth-api.decentraland.{}",
            suffix(ServiceGroup::Auth)
        )
    }
}
pub fn auth_api_requests() -> String {
    if resolved_env(ServiceGroup::Auth) == DclEnvironment::Today {
        "https://auth-api.decentraland.zone/requests".to_string()
    } else {
        format!(
            "https://auth-api.decentraland.{}/requests",
            suffix(ServiceGroup::Auth)
        )
    }
}

// Catalyst
pub fn main_realm() -> String {
    format!(
        "https://realm-provider-ea.decentraland.{}/main",
        suffix(ServiceGroup::Catalyst)
    )
}
pub fn worlds_content_server() -> String {
    format!(
        "https://worlds-content-server.decentraland.{}/world/",
        suffix(ServiceGroup::Catalyst)
    )
}

// Peer (special: zone/today use peer-testing)
pub fn peer_base() -> String {
    if resolved_env(ServiceGroup::Catalyst) == DclEnvironment::Org {
        "https://peer.decentraland.org".to_string()
    } else {
        "https://peer-testing.decentraland.org".to_string()
    }
}
pub fn peer_content() -> String {
    format!("{}/content/", peer_base())
}
pub fn peer_lambdas() -> String {
    format!("{}/lambdas/", peer_base())
}

// Comms
pub fn comms_gatekeeper() -> String {
    format!(
        "https://comms-gatekeeper.decentraland.{}/get-scene-adapter",
        suffix(ServiceGroup::Comms)
    )
}
pub fn comms_gatekeeper_local() -> String {
    format!(
        "https://comms-gatekeeper-local.decentraland.{}/get-scene-adapter",
        suffix(ServiceGroup::Comms)
    )
}
pub fn social_service() -> String {
    format!(
        "wss://rpc-social-service-ea.decentraland.{}",
        suffix(ServiceGroup::Comms)
    )
}
pub fn archipelago_stats() -> String {
    format!(
        "https://archipelago-ea-stats.decentraland.{}",
        suffix(ServiceGroup::Comms)
    )
}

// Web3 (ungrouped — uses default)
pub fn ethereum_rpc() -> String {
    format!("wss://rpc.decentraland.{}/mainnet", default_suffix())
}
pub fn ethereum_rpc_with_project(project: &str) -> String {
    format!("{}?project={}", ethereum_rpc(), project)
}

// Places (hardcoded to org unless explicitly overridden)
pub fn places_api() -> String {
    let config = env::get_config();
    if config.has_override(ServiceGroup::Places) {
        format!(
            "https://places.decentraland.{}/api",
            config.suffix_for(ServiceGroup::Places)
        )
    } else {
        "https://places.decentraland.org/api".to_string()
    }
}

// Events
pub fn events_api() -> String {
    format!(
        "https://events.decentraland.{}/api/events",
        suffix(ServiceGroup::Events)
    )
}
pub fn jump_events() -> String {
    format!(
        "https://decentraland.{}/jump/events",
        suffix(ServiceGroup::Events)
    )
}

// Mobile BFF
pub fn mobile_bff() -> String {
    format!(
        "https://mobile-bff.decentraland.{}",
        suffix(ServiceGroup::MobileBff)
    )
}
pub fn destinations_api() -> String {
    format!(
        "https://mobile-bff.decentraland.{}/destinations",
        suffix(ServiceGroup::MobileBff)
    )
}
pub fn mobile_events_api() -> String {
    format!(
        "https://mobile-bff.decentraland.{}/events",
        suffix(ServiceGroup::MobileBff)
    )
}
pub fn account_deletion() -> String {
    format!(
        "https://mobile-bff.decentraland.{}/deletion",
        suffix(ServiceGroup::MobileBff)
    )
}

// Notifications
pub fn notifications_api() -> String {
    format!(
        "https://notifications.decentraland.{}",
        suffix(ServiceGroup::Notifications)
    )
}

// Frontend (ungrouped — uses default)
pub fn host() -> String {
    format!("https://decentraland.{}", default_suffix())
}
pub fn marketplace() -> String {
    format!("https://decentraland.{}/marketplace", default_suffix())
}
pub fn marketplace_claim_name() -> String {
    format!(
        "https://decentraland.{}/marketplace/names/claim",
        default_suffix()
    )
}
pub fn privacy_policy() -> String {
    format!("https://decentraland.{}/privacy", default_suffix())
}
pub fn terms_of_service() -> String {
    format!("https://decentraland.{}/terms", default_suffix())
}
pub fn content_policy() -> String {
    format!("https://decentraland.{}/content", default_suffix())
}

// Proxy (ungrouped — uses default)
pub fn open_sea_proxy() -> String {
    format!("https://opensea.decentraland.{}", default_suffix())
}

// Fixed (no transformation - used for signed fetch headers)
pub fn origin() -> String {
    "https://decentraland.org".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_env_is_org() {
        assert_eq!(default_suffix(), "org");
    }

    #[test]
    fn test_peer_base_org() {
        assert_eq!(peer_base(), "https://peer.decentraland.org");
    }

    #[test]
    fn test_peer_content_uses_peer_base() {
        assert_eq!(peer_content(), format!("{}/content/", peer_base()));
    }

    #[test]
    fn test_peer_lambdas_uses_peer_base() {
        assert_eq!(peer_lambdas(), format!("{}/lambdas/", peer_base()));
    }

    #[test]
    fn test_origin_is_fixed() {
        assert_eq!(origin(), "https://decentraland.org");
    }

    #[test]
    fn test_all_urls_valid() {
        let urls = [
            auth_frontend(),
            auth_mobile_frontend(),
            auth_api_base(),
            auth_api_requests(),
            main_realm(),
            worlds_content_server(),
            peer_base(),
            peer_content(),
            peer_lambdas(),
            comms_gatekeeper(),
            comms_gatekeeper_local(),
            social_service(),
            archipelago_stats(),
            ethereum_rpc(),
            places_api(),
            events_api(),
            mobile_events_api(),
            notifications_api(),
            mobile_bff(),
            host(),
            marketplace(),
            marketplace_claim_name(),
            privacy_policy(),
            terms_of_service(),
            content_policy(),
            jump_events(),
            account_deletion(),
            open_sea_proxy(),
            origin(),
        ];

        for url in urls {
            assert!(
                url.starts_with("https://") || url.starts_with("wss://"),
                "Invalid URL: {}",
                url
            );
        }
    }
}
