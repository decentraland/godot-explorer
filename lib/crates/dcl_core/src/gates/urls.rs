//! Every URL the shell needs, as one typed record built from `crate::urls`, so the shell never
//! spells a host. The record is rebuilt on every call: the environment can change at runtime
//! (deep link `dclenv`), and the shell must follow it.

use crate::{env::DclEnvironment, realm, urls};

/// Fixed universal-link host for shared scenes (`/open?position=x,y` or `?realm=<world>`).
const MOBILE_OPEN: &str = "https://mobile.dclexplorer.com/open";

/// The dev-only realm picker's non-default entry: the SDK team's Goerli Plaza copy.
const SDK_TEAM_REALM: &str = "https://sdk-team-cdn.decentraland.org/ipfs/goerli-plaza-main-latest";

/// A world the dev-only realm picker offers.
const DEV_WORLD: &str = "olavra.dcl.eth";

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum Environment {
    Org,
    Zone,
    Today,
}

impl From<DclEnvironment> for Environment {
    fn from(env: DclEnvironment) -> Self {
        match env {
            DclEnvironment::Org => Self::Org,
            DclEnvironment::Zone => Self::Zone,
            DclEnvironment::Today => Self::Today,
        }
    }
}

/// The default environment of the process (per-service overrides show in the URLs themselves).
pub fn environment() -> Environment {
    crate::env::get_environment().into()
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct Urls {
    /// `https://decentraland.<env>`
    pub host: String,
    pub help: String,
    pub privacy_policy: String,
    pub terms_of_service: String,
    pub content_policy: String,

    /// Places API base (`.../api`); the places list is `places`.
    pub places_api: String,
    pub places: String,
    /// `.../api/events`
    pub events_api: String,
    pub jump_events: String,

    /// The default (Genesis City) realm.
    pub main_realm: String,
    /// `.../world/`; a world's realm is this plus its lowercase name.
    pub worlds_content_server: String,
    pub peer_base: String,
    pub peer_content: String,
    pub peer_lambdas: String,

    pub mobile_bff: String,
    pub destinations_api: String,
    pub mobile_events_api: String,
    pub account_deletion: String,
    pub notifications_api: String,
    pub credits_server: String,
    pub social_service: String,

    pub marketplace: String,
    pub marketplace_api: String,
    pub marketplace_catalog_api: String,
    pub marketplace_claim_name: String,

    pub auth_frontend: String,
    pub auth_mobile_frontend: String,
    pub auth_api_base: String,
    pub auth_api_requests: String,

    /// Base of shared scene links; see `scene_link`.
    pub mobile_open: String,
    /// Realms the dev-only picker offers.
    pub dev_realms: Vec<String>,
}

pub fn build() -> Urls {
    Urls {
        host: urls::host(),
        help: format!("{}/help", urls::host()),
        privacy_policy: urls::privacy_policy(),
        terms_of_service: urls::terms_of_service(),
        content_policy: urls::content_policy(),

        places_api: urls::places_api(),
        places: format!("{}/places", urls::places_api()),
        events_api: urls::events_api(),
        jump_events: urls::jump_events(),

        main_realm: urls::main_realm(),
        worlds_content_server: urls::worlds_content_server(),
        peer_base: urls::peer_base(),
        peer_content: urls::peer_content(),
        peer_lambdas: urls::peer_lambdas(),

        mobile_bff: urls::mobile_bff(),
        destinations_api: urls::destinations_api(),
        mobile_events_api: urls::mobile_events_api(),
        account_deletion: urls::account_deletion(),
        notifications_api: urls::notifications_api(),
        credits_server: urls::credits_server(),
        social_service: urls::social_service(),

        marketplace: urls::marketplace(),
        marketplace_api: urls::marketplace_api(),
        marketplace_catalog_api: urls::marketplace_catalog_api(),
        marketplace_claim_name: urls::marketplace_claim_name(),

        auth_frontend: urls::auth_frontend(),
        auth_mobile_frontend: urls::auth_mobile_frontend(),
        auth_api_base: urls::auth_api_base(),
        auth_api_requests: urls::auth_api_requests(),

        mobile_open: MOBILE_OPEN.to_string(),
        dev_realms: vec![
            urls::main_realm(),
            SDK_TEAM_REALM.to_string(),
            realm::dcl_world_url(DEV_WORLD),
        ],
    }
}

fn encode(text: &str) -> String {
    realm::uri_encode(text)
}

/// The catalog page of a backpack section (`"wearables"` or `"emotes"`).
pub fn marketplace_browse(section: &str) -> String {
    urls::marketplace_browse(section)
}

/// The detail page of a catalog item.
pub fn marketplace_item(contract_address: &str, item_id: &str) -> String {
    urls::marketplace_item(contract_address, item_id)
}

/// Web link that opens the explorer in a world.
pub fn jump_realm_link(realm_name: &str) -> String {
    format!("{}/jump/?realm={}", urls::host(), encode(realm_name))
}

/// Web link that opens the explorer at a Genesis City position (`"x,y"`).
pub fn jump_position_link(position: &str) -> String {
    format!("{}/jump/?position={}", urls::host(), encode(position))
}

/// Web page of an event.
pub fn event_link(event_id: &str) -> String {
    format!("{}/events/event/?id={}", urls::host(), encode(event_id))
}

/// Shareable link to a scene: `position=x,y` in Genesis City, `realm=<world>` otherwise.
pub fn scene_link(realm_url: &str, x: i32, y: i32) -> String {
    let query = if realm::normalize_realm_url(realm_url)
        == realm::normalize_realm_url(&urls::main_realm())
    {
        format!("position={x},{y}")
    } else {
        let name = realm_url
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or_default();
        format!("realm={name}")
    };
    format!("{MOBILE_OPEN}?{query}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_field_is_filled() {
        let all = build();
        for (name, value) in [
            ("host", &all.host),
            ("help", &all.help),
            ("privacy_policy", &all.privacy_policy),
            ("terms_of_service", &all.terms_of_service),
            ("content_policy", &all.content_policy),
            ("places_api", &all.places_api),
            ("places", &all.places),
            ("events_api", &all.events_api),
            ("jump_events", &all.jump_events),
            ("main_realm", &all.main_realm),
            ("worlds_content_server", &all.worlds_content_server),
            ("peer_base", &all.peer_base),
            ("peer_content", &all.peer_content),
            ("peer_lambdas", &all.peer_lambdas),
            ("mobile_bff", &all.mobile_bff),
            ("destinations_api", &all.destinations_api),
            ("mobile_events_api", &all.mobile_events_api),
            ("account_deletion", &all.account_deletion),
            ("notifications_api", &all.notifications_api),
            ("credits_server", &all.credits_server),
            ("social_service", &all.social_service),
            ("marketplace", &all.marketplace),
            ("marketplace_api", &all.marketplace_api),
            ("marketplace_catalog_api", &all.marketplace_catalog_api),
            ("marketplace_claim_name", &all.marketplace_claim_name),
            ("auth_frontend", &all.auth_frontend),
            ("auth_mobile_frontend", &all.auth_mobile_frontend),
            ("auth_api_base", &all.auth_api_base),
            ("auth_api_requests", &all.auth_api_requests),
            ("mobile_open", &all.mobile_open),
        ] {
            assert!(
                value.starts_with("http://")
                    || value.starts_with("https://")
                    || value.starts_with("wss://"),
                "{name} is not a URL: {value:?}"
            );
        }
        assert_eq!(all.dev_realms.len(), 3);
        assert!(all.dev_realms.iter().all(|realm| !realm.is_empty()));
    }

    #[test]
    fn links_encode_their_parameter() {
        assert!(jump_position_link("-9,-9").ends_with("/jump/?position=-9%2C-9"));
        assert!(jump_realm_link("a b.dcl.eth").ends_with("/jump/?realm=a%20b.dcl.eth"));
        assert!(event_link("x&y").ends_with("/events/event/?id=x%26y"));
    }

    #[test]
    fn scene_links_name_a_position_in_genesis_and_a_world_elsewhere() {
        assert_eq!(
            scene_link(&urls::main_realm(), 3, -4),
            "https://mobile.dclexplorer.com/open?position=3,-4"
        );
        assert_eq!(
            scene_link(
                &format!("{}foo.dcl.eth", urls::worlds_content_server()),
                0,
                0
            ),
            "https://mobile.dclexplorer.com/open?realm=foo.dcl.eth"
        );
    }
}
