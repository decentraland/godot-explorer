//! Realm rules: how a realm string becomes a URL, which servers are the Genesis City catalysts,
//! and what a realm's `/about` document says. Pure functions; fetching and state live in the host.

use serde_json::Value;

use crate::urls;

/// The Genesis City catalysts. A realm on one of these is the main world.
pub const DAO_SERVERS: [&str; 13] = [
    "https://peer-ec1.decentraland.org/",
    "https://peer-ec2.decentraland.org/",
    "https://peer-wc1.decentraland.org/",
    "https://peer-eu1.decentraland.org/",
    "https://peer-ap1.decentraland.org/",
    "https://interconnected.online/",
    "https://peer.decentral.io/",
    "https://peer.melonwave.com/",
    "https://peer.kyllian.me/",
    "https://peer.uadevops.com/",
    "https://peer.dclnodes.io/",
    "https://realm-provider.decentraland.org/main/",
    "https://realm-provider-ea.decentraland.org/main/",
];

const DEFAULT_MIN_BOUNDS: (i32, i32) = (-150, -150);
const DEFAULT_MAX_BOUNDS: (i32, i32) = (163, 158);
const ENTITY_URN_PREFIX: &str = "urn:decentraland:entity:";

pub fn is_dcl_ens(name: &str) -> bool {
    name.strip_suffix(".dcl.eth")
        .is_some_and(|label| !label.is_empty() && label.chars().all(|c| c.is_ascii_alphanumeric()))
}

pub fn is_genesis_city(realm: &str) -> bool {
    let url = ensure_starts_with_https(&ensure_ends_with_slash(&resolve_realm_url(realm)));
    DAO_SERVERS.contains(&url.as_str())
}

/// A realm served from this machine or its network, i.e. a local preview: an IP address
/// (`http://192.168.1.20:8000`) or `localhost`.
pub fn is_local_preview(realm: &str) -> bool {
    realm.lines().any(|line| {
        let lower = line.to_ascii_lowercase();
        let Some(rest) = lower
            .strip_prefix("https://")
            .or_else(|| lower.strip_prefix("http://"))
        else {
            return false;
        };
        is_localhost(rest) || is_ip_address(rest)
    })
}

fn is_localhost(authority_and_path: &str) -> bool {
    authority_and_path
        .strip_prefix("localhost")
        .is_some_and(|rest| matches!(rest.chars().next(), None | Some(':' | '/' | '?' | '#')))
}

/// Four dot-separated numbers; the last may be followed by a port or path.
fn is_ip_address(authority_and_path: &str) -> bool {
    let mut groups = authority_and_path.splitn(4, '.');
    let number = |group: Option<&str>, last: bool| {
        let Some(group) = group else { return false };
        let digits = group.chars().take_while(char::is_ascii_digit).count();
        digits > 0 && (last || digits == group.len())
    };
    number(groups.next(), false)
        && number(groups.next(), false)
        && number(groups.next(), false)
        && number(groups.next(), true)
}

/// Percent-encodes everything but the RFC 3986 unreserved characters, like Godot's `uri_encode`.
fn uri_encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

pub fn dcl_world_url(dcl_name: &str) -> String {
    urls::worlds_content_server() + &uri_encode(&dcl_name.to_lowercase())
}

pub fn ensure_dcl_ens(url: &str) -> String {
    url.replace(&urls::worlds_content_server(), "")
}

pub fn ensure_remove_slash(text: &str) -> String {
    text.strip_suffix('/').unwrap_or(text).to_string()
}

pub fn ensure_ends_with_slash(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    format!("{}/", text.strip_suffix('/').unwrap_or(text))
}

pub fn ensure_starts_with_https(text: &str) -> String {
    if text.starts_with("https://") || text.starts_with("http://") {
        text.to_string()
    } else {
        format!("https://{text}")
    }
}

pub fn remove_scheme(text: &str) -> String {
    let text = text.strip_prefix("https://").unwrap_or(text);
    text.strip_prefix("http://").unwrap_or(text).to_string()
}

pub fn ensure_reduce_url(url: &str) -> String {
    remove_scheme(&ensure_remove_slash(&ensure_dcl_ens(url)))
}

pub fn resolve_realm_url(value: &str) -> String {
    if is_dcl_ens(value) {
        dcl_world_url(value)
    } else {
        value.to_string()
    }
}

/// The canonical URL a realm string resolves to. Two spellings of the same destination
/// ("SpaceRunner.dcl.eth" and "spacerunner.dcl.eth", with or without a trailing slash or scheme)
/// normalise to one string.
pub fn normalize_realm_url(value: &str) -> String {
    if value.is_empty() {
        return String::new();
    }
    ensure_starts_with_https(&ensure_ends_with_slash(&resolve_realm_url(value)))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamValue {
    Text(String),
    /// A key with no `=value`.
    Flag,
}

pub fn get_params(url: &str) -> Vec<(String, Vec<ParamValue>)> {
    let mut params: Vec<(String, Vec<ParamValue>)> = Vec::new();
    let Some(query) = url.split('?').nth(1) else {
        return params;
    };
    for param in query.split('&') {
        let mut key_value = param.split('=');
        let key = key_value.next().unwrap_or_default().to_string();
        let value = key_value
            .next()
            .map_or(ParamValue::Flag, |v| ParamValue::Text(v.to_string()));
        match params.iter_mut().find(|(k, _)| *k == key) {
            Some((_, values)) => values.push(value),
            None => params.push((key, vec![value])),
        }
    }
    params
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityUrn {
    pub urn: String,
    pub entity_id: String,
    pub base_url: String,
}

/// `urn:decentraland:entity:ba…` optionally followed by `?=&baseUrl=…`.
pub fn parse_urn(urn: &str) -> Option<EntityUrn> {
    let id = urn.strip_prefix(ENTITY_URN_PREFIX)?;
    let id = id.get(..59)?;
    if !id.starts_with("ba") || !id.chars().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    let base_url = get_params(urn)
        .into_iter()
        .find(|(key, _)| key == "baseUrl")
        .and_then(|(_, values)| values.into_iter().next())
        .map(|value| match value {
            ParamValue::Text(text) => text,
            ParamValue::Flag => String::new(),
        })
        .unwrap_or_default();
    Some(EntityUrn {
        urn: format!("{ENTITY_URN_PREFIX}{id}"),
        entity_id: id.to_string(),
        base_url,
    })
}

/// Why an `/about` document cannot be committed, `None` when it can. Checked before any realm
/// state changes: a malformed `content` would otherwise leave a half-committed realm.
pub fn about_problem(about: &Value) -> Option<&'static str> {
    if !about.is_object() {
        return Some("invalid /about response");
    }
    match about.pointer("/content/publicUrl") {
        Some(Value::String(_)) => None,
        _ => Some("/about missing or malformed content.publicUrl"),
    }
}

/// What a realm's `/about` says, in the shape the host stores.
#[derive(Debug, Clone, PartialEq)]
pub struct RealmCommit {
    pub scene_urns: Vec<EntityUrn>,
    /// Set when the scene list came from the world's own listing; the host writes it back into
    /// `configurations.scenesUrn` so the scene fetcher reads the full list.
    pub scenes_urn_override: Option<Vec<String>>,
    pub global_scene_urns: Vec<EntityUrn>,
    pub city_loader_content_base_url: String,
    pub lambda_server_base_url: String,
    pub realm_name: String,
    pub network_id: i64,
    pub serves_local_scenes: bool,
    pub min_bounds: (i32, i32),
    pub max_bounds: (i32, i32),
    pub content_base_url: String,
}

fn parsed_urns(configuration: &Value, key: &str) -> Vec<EntityUrn> {
    configuration
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|urn| parse_urn(urn.as_str()?))
        .collect()
}

fn as_i32(value: Option<&Value>) -> i32 {
    value
        .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)))
        .unwrap_or(0) as i32
}

/// `world_scenes` is the complete scene list of a `.dcl.eth` world when the host already has it;
/// `/about` alone only reports the last deployed scene of those.
pub fn commit_from_about(about: &Value, world_scenes: &[EntityUrn]) -> RealmCommit {
    let configuration = about.get("configurations").unwrap_or(&Value::Null);
    let text = |key: &str| {
        configuration
            .get(key)
            .and_then(Value::as_str)
            .map(str::to_string)
    };

    let realm_name = text("realmName").unwrap_or_else(|| "no_realm_name".to_string());

    let mut scene_urns = parsed_urns(configuration, "scenesUrn");
    let mut scenes_urn_override = None;
    if is_dcl_ens(&text("realmName").unwrap_or_default()) && !world_scenes.is_empty() {
        scene_urns = world_scenes.to_vec();
        scenes_urn_override = Some(
            world_scenes
                .iter()
                .map(|scene| format!("{}?=&baseUrl={}", scene.urn, scene.base_url))
                .collect(),
        );
    }

    let city_loader_content_base_url = text("cityLoaderContentServer")
        .filter(|url| !url.is_empty())
        .map(|url| ensure_ends_with_slash(&url))
        .unwrap_or_default();

    let lambda_server_base_url = match about.pointer("/lambdas/publicUrl").and_then(Value::as_str) {
        Some("") => String::new(),
        Some(url) => ensure_ends_with_slash(url),
        None => ensure_ends_with_slash(&urls::peer_lambdas()),
    };

    let sizes = configuration
        .pointer("/map/sizes")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let (min_bounds, max_bounds) = if sizes.is_empty() {
        (DEFAULT_MIN_BOUNDS, DEFAULT_MAX_BOUNDS)
    } else {
        sizes.iter().fold(
            ((i32::MAX, i32::MAX), (i32::MIN, i32::MIN)),
            |(min, max), size| {
                let (left, top) = (as_i32(size.get("left")), as_i32(size.get("top")));
                let (right, bottom) = (as_i32(size.get("right")), as_i32(size.get("bottom")));
                (
                    (min.0.min(left), min.1.min(bottom)),
                    (max.0.max(right), max.1.max(top)),
                )
            },
        )
    };

    RealmCommit {
        scene_urns,
        scenes_urn_override,
        global_scene_urns: parsed_urns(configuration, "globalScenesUrn"),
        city_loader_content_base_url,
        lambda_server_base_url,
        realm_name,
        network_id: configuration
            .get("networkId")
            .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)))
            .unwrap_or(1),
        serves_local_scenes: configuration
            .get("localSceneParcels")
            .and_then(Value::as_array)
            .is_some_and(|parcels| !parcels.is_empty()),
        min_bounds,
        max_bounds,
        content_base_url: ensure_ends_with_slash(
            about
                .pointer("/content/publicUrl")
                .and_then(Value::as_str)
                .unwrap_or_default(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const ENTITY: &str = "bafkreigdk4n6z3pqz4yu5t6nrcxstrv3jx3itxqjxy5zoaxhvhoqzpbfsy";

    #[test]
    fn recognises_world_names() {
        assert!(is_dcl_ens("spacerunner.dcl.eth"));
        assert!(is_dcl_ens("SpaceRunner.dcl.eth"));
        assert!(!is_dcl_ens(".dcl.eth"));
        assert!(!is_dcl_ens("space-runner.dcl.eth"));
        assert!(!is_dcl_ens("spacerunner.dcl.eth/"));
    }

    #[test]
    fn genesis_city_is_any_dao_server_in_any_spelling() {
        assert!(is_genesis_city("https://peer-ec1.decentraland.org"));
        assert!(is_genesis_city("peer-ec1.decentraland.org/"));
        assert!(is_genesis_city(
            "https://realm-provider.decentraland.org/main"
        ));
        assert!(!is_genesis_city("https://example.com"));
        assert!(!is_genesis_city("spacerunner.dcl.eth"));
        assert!(!is_genesis_city(""));
    }

    #[test]
    fn local_preview_is_an_ip_address() {
        assert!(is_local_preview("http://192.168.1.20:8000"));
        assert!(is_local_preview("HTTPS://10.0.0.1"));
        assert!(is_local_preview("http://localhost:8000"));
        assert!(is_local_preview("https://LOCALHOST/path"));
        assert!(is_local_preview("http://localhost"));
        assert!(!is_local_preview("http://localhost.example.com"));
        assert!(!is_local_preview("https://peer.decentraland.org"));
        assert!(!is_local_preview("http://1.2.3"));
    }

    #[test]
    fn normalisation_makes_two_spellings_one() {
        let a = normalize_realm_url("SpaceRunner.dcl.eth");
        assert_eq!(a, normalize_realm_url("spacerunner.dcl.eth"));
        assert!(a.starts_with("https://") && a.ends_with("/spacerunner.dcl.eth/"));
        assert_eq!(
            normalize_realm_url("peer.example.com"),
            "https://peer.example.com/"
        );
        assert_eq!(normalize_realm_url(""), "");
    }

    #[test]
    fn slash_and_scheme_helpers() {
        assert_eq!(ensure_ends_with_slash(""), "");
        assert_eq!(ensure_ends_with_slash("a/"), "a/");
        assert_eq!(ensure_ends_with_slash("a"), "a/");
        assert_eq!(ensure_remove_slash("a/"), "a");
        assert_eq!(remove_scheme("http://a"), "a");
        assert_eq!(ensure_starts_with_https("http://a"), "http://a");
        assert_eq!(uri_encode("a b/é"), "a%20b%2F%C3%A9");
    }

    #[test]
    fn urn_carries_its_base_url() {
        let urn = format!("{ENTITY_URN_PREFIX}{ENTITY}?=&baseUrl=https://x.org/contents/");
        let parsed = parse_urn(&urn).unwrap();
        assert_eq!(parsed.entity_id, ENTITY);
        assert_eq!(parsed.urn, format!("{ENTITY_URN_PREFIX}{ENTITY}"));
        assert_eq!(parsed.base_url, "https://x.org/contents/");
    }

    #[test]
    fn bad_urns_are_rejected() {
        assert!(parse_urn("urn:decentraland:entity:bafshort").is_none());
        assert!(parse_urn("urn:other:thing").is_none());
    }

    #[test]
    fn about_must_have_a_public_content_url() {
        assert_eq!(about_problem(&json!([])), Some("invalid /about response"));
        assert!(about_problem(&json!({"content": null})).is_some());
        assert!(about_problem(&json!({"content": {"publicUrl": 3}})).is_some());
        assert!(about_problem(&json!({"content": {"publicUrl": "https://c/"}})).is_none());
    }

    #[test]
    fn commit_reads_the_about_document() {
        let about = json!({
            "content": {"publicUrl": "https://peer.org/content"},
            "lambdas": {"publicUrl": "https://peer.org/lambdas"},
            "configurations": {
                "realmName": "main",
                "networkId": 1,
                "cityLoaderContentServer": "https://city.org",
                "scenesUrn": [format!("{ENTITY_URN_PREFIX}{ENTITY}?=&baseUrl=https://b/")],
                "map": {"sizes": [
                    {"left": -10, "top": 20, "right": 30, "bottom": -40},
                    {"left": -5, "top": 50, "right": 10, "bottom": -1}
                ]}
            }
        });
        let commit = commit_from_about(&about, &[]);
        assert_eq!(commit.realm_name, "main");
        assert_eq!(commit.content_base_url, "https://peer.org/content/");
        assert_eq!(commit.lambda_server_base_url, "https://peer.org/lambdas/");
        assert_eq!(commit.city_loader_content_base_url, "https://city.org/");
        assert_eq!(commit.scene_urns.len(), 1);
        assert_eq!(commit.min_bounds, (-10, -40));
        assert_eq!(commit.max_bounds, (30, 50));
        assert!(!commit.serves_local_scenes);
    }

    #[test]
    fn commit_falls_back_to_defaults() {
        let about = json!({"content": {"publicUrl": "https://c"}});
        let commit = commit_from_about(&about, &[]);
        assert_eq!(commit.realm_name, "no_realm_name");
        assert_eq!(commit.network_id, 1);
        assert_eq!(commit.min_bounds, DEFAULT_MIN_BOUNDS);
        assert_eq!(commit.max_bounds, DEFAULT_MAX_BOUNDS);
        assert_eq!(
            commit.lambda_server_base_url,
            ensure_ends_with_slash(&urls::peer_lambdas())
        );
    }

    #[test]
    fn a_world_listing_replaces_the_last_scene_only_for_dcl_names() {
        let scene = EntityUrn {
            urn: format!("{ENTITY_URN_PREFIX}{ENTITY}"),
            entity_id: ENTITY.into(),
            base_url: "https://w/contents/".into(),
        };
        let world = json!({"content": {"publicUrl": "https://c"}, "configurations": {"realmName": "foo.dcl.eth"}});
        let commit = commit_from_about(&world, std::slice::from_ref(&scene));
        assert_eq!(commit.scene_urns, vec![scene.clone()]);
        assert_eq!(
            commit.scenes_urn_override,
            Some(vec![format!("{}?=&baseUrl=https://w/contents/", scene.urn)])
        );

        let other =
            json!({"content": {"publicUrl": "https://c"}, "configurations": {"realmName": "main"}});
        let commit = commit_from_about(&other, std::slice::from_ref(&scene));
        assert!(commit.scene_urns.is_empty() && commit.scenes_urn_override.is_none());
    }
}
