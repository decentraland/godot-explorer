//! Godot bindings for the centralized URL management system.

use godot::prelude::*;

use crate::urls;

/// Godot class providing access to Decentraland URLs.
///
/// All methods are static. URLs are automatically transformed based on the
/// current environment (org, zone, today) set via `DclGlobal.set_dcl_environment()`.
#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub struct DclUrls {
    _base: Base<RefCounted>,
}

#[godot_api]
impl DclUrls {
    // Auth
    #[func]
    pub fn auth_frontend() -> GString {
        urls::auth_frontend().to_godot()
    }
    #[func]
    pub fn auth_mobile_frontend() -> GString {
        urls::auth_mobile_frontend().to_godot()
    }
    #[func]
    pub fn auth_api_base() -> GString {
        urls::auth_api_base().to_godot()
    }
    #[func]
    pub fn auth_api_requests() -> GString {
        urls::auth_api_requests().to_godot()
    }

    // Content
    #[func]
    pub fn main_realm() -> GString {
        urls::main_realm().to_godot()
    }
    #[func]
    pub fn worlds_content_server() -> GString {
        urls::worlds_content_server().to_godot()
    }
    #[func]
    pub fn peer_base() -> GString {
        urls::peer_base().to_godot()
    }
    #[func]
    pub fn peer_content() -> GString {
        urls::peer_content().to_godot()
    }
    #[func]
    pub fn peer_lambdas() -> GString {
        urls::peer_lambdas().to_godot()
    }

    // Comms
    #[func]
    pub fn comms_gatekeeper() -> GString {
        urls::comms_gatekeeper().to_godot()
    }
    #[func]
    pub fn comms_gatekeeper_local() -> GString {
        urls::comms_gatekeeper_local().to_godot()
    }
    #[func]
    pub fn social_service() -> GString {
        urls::social_service().to_godot()
    }
    #[func]
    pub fn archipelago_stats() -> GString {
        urls::archipelago_stats().to_godot()
    }

    // Web3
    #[func]
    pub fn ethereum_rpc() -> GString {
        urls::ethereum_rpc().to_godot()
    }
    #[func]
    pub fn ethereum_rpc_with_project(project: GString) -> GString {
        urls::ethereum_rpc_with_project(&project.to_string()).to_godot()
    }

    // API
    #[func]
    pub fn places_api() -> GString {
        urls::places_api().to_godot()
    }
    #[func]
    pub fn events_api() -> GString {
        urls::events_api().to_godot()
    }
    #[func]
    pub fn notifications_api() -> GString {
        urls::notifications_api().to_godot()
    }
    #[func]
    pub fn mobile_bff() -> GString {
        urls::mobile_bff().to_godot()
    }

    // Frontend
    #[func]
    pub fn host() -> GString {
        urls::host().to_godot()
    }
    #[func]
    pub fn marketplace() -> GString {
        urls::marketplace().to_godot()
    }
    #[func]
    pub fn marketplace_claim_name() -> GString {
        urls::marketplace_claim_name().to_godot()
    }
    #[func]
    pub fn privacy_policy() -> GString {
        urls::privacy_policy().to_godot()
    }
    #[func]
    pub fn terms_of_service() -> GString {
        urls::terms_of_service().to_godot()
    }
    #[func]
    pub fn content_policy() -> GString {
        urls::content_policy().to_godot()
    }
    #[func]
    pub fn jump_events() -> GString {
        urls::jump_events().to_godot()
    }
    #[func]
    pub fn account_deletion() -> GString {
        urls::account_deletion().to_godot()
    }

    // Proxy
    #[func]
    pub fn open_sea_proxy() -> GString {
        urls::open_sea_proxy().to_godot()
    }

    // Fixed
    #[func]
    pub fn origin() -> GString {
        urls::origin().to_godot()
    }
}
