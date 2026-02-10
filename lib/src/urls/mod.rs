//! Centralized URL management for Decentraland services.
//!
//! URLs are automatically transformed based on the current environment (org, zone, today).

use crate::env::get_environment;

fn env() -> &'static str {
    get_environment().suffix()
}

// Auth
pub fn auth_frontend() -> String {
    format!("https://decentraland.{}/auth/requests", env())
}
pub fn auth_mobile_frontend() -> String {
    format!("https://decentraland.{}/auth/mobile", env())
}
pub fn auth_api_base() -> String {
    format!("https://auth-api.decentraland.{}", env())
}
pub fn auth_api_requests() -> String {
    format!("https://auth-api.decentraland.{}/requests", env())
}

// Content
pub fn main_realm() -> String {
    format!("https://realm-provider-ea.decentraland.{}/main", env())
}
pub fn worlds_content_server() -> String {
    format!(
        "https://worlds-content-server.decentraland.{}/world/",
        env()
    )
}

// Peer (special: zone/today use peer-testing)
pub fn peer_base() -> String {
    if env() == "org" {
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
        env()
    )
}
pub fn social_service() -> String {
    format!("wss://rpc-social-service-ea.decentraland.{}", env())
}
pub fn archipelago_stats() -> String {
    format!("https://archipelago-ea-stats.decentraland.{}", env())
}

// Web3
pub fn ethereum_rpc() -> String {
    format!("wss://rpc.decentraland.{}/mainnet", env())
}
pub fn ethereum_rpc_with_project(project: &str) -> String {
    format!("{}?project={}", ethereum_rpc(), project)
}

// API
pub fn places_api() -> String {
    format!("https://places.decentraland.{}/api", env())
}
pub fn destinations_api() -> String {
    format!("https://mobile-bff.decentraland.{}/destinations", env())
}
pub fn events_api() -> String {
    format!("https://events.decentraland.{}/api/events", env())
}
pub fn mobile_events_api() -> String {
    format!("https://mobile-bff.decentraland.{}/events", env())
}
pub fn notifications_api() -> String {
    format!("https://notifications.decentraland.{}", env())
}
pub fn mobile_bff() -> String {
    format!("https://mobile-bff.decentraland.{}", env())
}

// Frontend
pub fn host() -> String {
    format!("https://decentraland.{}", env())
}
pub fn marketplace() -> String {
    format!("https://decentraland.{}/marketplace", env())
}
pub fn marketplace_claim_name() -> String {
    format!("https://decentraland.{}/marketplace/names/claim", env())
}
pub fn privacy_policy() -> String {
    format!("https://decentraland.{}/privacy", env())
}
pub fn terms_of_service() -> String {
    format!("https://decentraland.{}/terms", env())
}
pub fn content_policy() -> String {
    format!("https://decentraland.{}/content", env())
}
pub fn jump_events() -> String {
    format!("https://decentraland.{}/jump/events", env())
}
pub fn account_deletion() -> String {
    format!("https://mobile-bff.decentraland.{}/deletion", env())
}

// Proxy
pub fn open_sea_proxy() -> String {
    format!("https://opensea.decentraland.{}", env())
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
        assert_eq!(env(), "org");
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
