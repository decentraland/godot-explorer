/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Godot bindings for the centralized URL management system.
//!
//! This class provides static methods for accessing Decentraland URLs from GDScript.
//! All URLs are automatically transformed based on the current environment.
//!
//! # Usage in GDScript
//! ```gdscript
//! # Get the genesis realm URL
//! var genesis_url = DclUrls.genesis()
//!
//! # Get the places API URL
//! var places_url = DclUrls.places_api()
//!
//! # Get the events API URL
//! var events_url = DclUrls.events_api()
//! ```

use godot::prelude::*;

use crate::urls;

/// Godot class providing access to Decentraland URLs.
///
/// All methods are static - you don't need to create an instance.
/// URLs are automatically transformed based on the current environment
/// (org, zone, today) set via `DclGlobal.set_dcl_environment()`.
#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub struct DclUrls {
    _base: Base<RefCounted>,
}

#[godot_api]
impl DclUrls {
    // ========================================================================
    // Auth URLs
    // ========================================================================

    /// Get the auth frontend URL.
    /// Returns: https://decentraland.{ENV}/auth/requests
    #[func]
    pub fn auth_frontend() -> GString {
        urls::auth_frontend().to_godot()
    }

    /// Get the auth API base URL.
    /// Returns: https://auth-api.decentraland.{ENV}
    #[func]
    pub fn auth_api_base() -> GString {
        urls::auth_api_base().to_godot()
    }

    /// Get the auth API requests endpoint URL.
    /// Returns: https://auth-api.decentraland.{ENV}/requests
    #[func]
    pub fn auth_api_requests() -> GString {
        urls::auth_api_requests().to_godot()
    }

    // ========================================================================
    // Content URLs
    // ========================================================================

    /// Get the genesis realm provider URL.
    /// Returns: https://realm-provider-ea.decentraland.{ENV}/main
    #[func]
    pub fn genesis() -> GString {
        urls::genesis().to_godot()
    }

    /// Get the worlds content server base URL.
    /// Returns: https://worlds-content-server.decentraland.{ENV}/world/
    #[func]
    pub fn worlds_content_server() -> GString {
        urls::worlds_content_server().to_godot()
    }

    /// Get the peer base URL.
    /// Production: https://peer.decentraland.org
    /// Zone/Today: https://peer-testing.decentraland.org
    #[func]
    pub fn peer_base() -> GString {
        urls::peer_base().to_godot()
    }

    /// Get the peer content server URL.
    /// Production: https://peer.decentraland.org/content/
    /// Zone/Today: https://peer-testing.decentraland.org/content/
    #[func]
    pub fn peer_content() -> GString {
        urls::peer_content().to_godot()
    }

    /// Get the peer lambdas server URL.
    /// Production: https://peer.decentraland.org/lambdas/
    /// Zone/Today: https://peer-testing.decentraland.org/lambdas/
    #[func]
    pub fn peer_lambdas() -> GString {
        urls::peer_lambdas().to_godot()
    }

    // ========================================================================
    // Comms URLs
    // ========================================================================

    /// Get the comms gatekeeper URL.
    /// Returns: https://comms-gatekeeper.decentraland.{ENV}/get-scene-adapter
    #[func]
    pub fn comms_gatekeeper() -> GString {
        urls::comms_gatekeeper().to_godot()
    }

    /// Get the social service WebSocket URL.
    /// Returns: wss://rpc-social-service-ea.decentraland.{ENV}
    #[func]
    pub fn social_service() -> GString {
        urls::social_service().to_godot()
    }

    /// Get the archipelago stats URL.
    /// Returns: https://archipelago-ea-stats.decentraland.{ENV}
    #[func]
    pub fn archipelago_stats() -> GString {
        urls::archipelago_stats().to_godot()
    }

    // ========================================================================
    // Web3 URLs
    // ========================================================================

    /// Get the Ethereum RPC WebSocket URL.
    /// Returns: wss://rpc.decentraland.{ENV}/mainnet
    #[func]
    pub fn ethereum_rpc() -> GString {
        urls::ethereum_rpc().to_godot()
    }

    /// Get the Ethereum RPC WebSocket URL with project parameter.
    /// Returns: wss://rpc.decentraland.{ENV}/mainnet?project={project}
    #[func]
    pub fn ethereum_rpc_with_project(project: GString) -> GString {
        urls::ethereum_rpc_with_project(&project.to_string()).to_godot()
    }

    // ========================================================================
    // API URLs
    // ========================================================================

    /// Get the places API base URL.
    /// Returns: https://places.decentraland.{ENV}/api
    #[func]
    pub fn places_api() -> GString {
        urls::places_api().to_godot()
    }

    /// Get the events API base URL.
    /// Returns: https://events.decentraland.{ENV}/api
    #[func]
    pub fn events_api() -> GString {
        urls::events_api().to_godot()
    }

    /// Get the notifications API base URL.
    /// Returns: https://notifications.decentraland.{ENV}
    #[func]
    pub fn notifications_api() -> GString {
        urls::notifications_api().to_godot()
    }

    /// Get the mobile BFF URL.
    /// Returns: https://mobile-bff.decentraland.{ENV}
    #[func]
    pub fn mobile_bff() -> GString {
        urls::mobile_bff().to_godot()
    }

    // ========================================================================
    // Frontend URLs
    // ========================================================================

    /// Get the Decentraland host URL.
    /// Returns: https://decentraland.{ENV}
    #[func]
    pub fn host() -> GString {
        urls::host().to_godot()
    }

    /// Get the marketplace URL.
    /// Returns: https://decentraland.{ENV}/marketplace
    #[func]
    pub fn marketplace() -> GString {
        urls::marketplace().to_godot()
    }

    /// Get the marketplace claim name URL.
    /// Returns: https://decentraland.{ENV}/marketplace/names/claim
    #[func]
    pub fn marketplace_claim_name() -> GString {
        urls::marketplace_claim_name().to_godot()
    }

    /// Get the jump events URL.
    /// Returns: https://decentraland.{ENV}/jump/events
    #[func]
    pub fn jump_events() -> GString {
        urls::jump_events().to_godot()
    }

    /// Get the account deletion URL.
    /// Returns: https://decentraland.{ENV}/account-deletion
    #[func]
    pub fn account_deletion() -> GString {
        urls::account_deletion().to_godot()
    }

    // ========================================================================
    // Proxy URLs
    // ========================================================================

    /// Get the OpenSea proxy URL.
    /// Returns: https://opensea.decentraland.{ENV}
    #[func]
    pub fn opensea_proxy() -> GString {
        urls::opensea_proxy().to_godot()
    }

    // ========================================================================
    // Origin URL (for signed fetch)
    // ========================================================================

    /// Get the origin URL for signed fetch headers.
    /// Returns: https://decentraland.org (always .org)
    #[func]
    pub fn origin() -> GString {
        urls::origin().to_godot()
    }

    // ========================================================================
    // Utility Methods
    // ========================================================================

    /// Clear the URL cache. Call this if you change the environment at runtime.
    #[func]
    pub fn clear_cache() {
        urls::clear_cache();
    }
}
