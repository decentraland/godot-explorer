//! Reading what a place's servers say about it: a world's scene listing, the comms handshake a
//! password-protected world needs, and the summary the loading screen shows.
//!
//! All pure: the host fetches, these read. JSON shapes are tolerated the way the content servers
//! need them to be, so an odd payload yields "nothing known" instead of a failure.

use std::collections::BTreeMap;

use serde_json::{json, Value};

use crate::realm;

/// Godot's `str()` of a JSON value: absent is empty, null prints as `<null>`.
fn gd_str(value: Option<&Value>) -> String {
    match value {
        None => String::new(),
        Some(Value::String(text)) => text.clone(),
        Some(Value::Null) => "<null>".to_string(),
        Some(other) => other.to_string(),
    }
}

fn scenes(listing: &Value) -> &[Value] {
    listing
        .get("scenes")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

/// A world's `/about` names only its last deployed scene, so its `/scenes` listing is the only
/// place the full parcel layout shows up. Maps `"x,y"` to the scene's entity.
pub fn world_parcel_index(listing: &Value) -> BTreeMap<String, Value> {
    let mut index = BTreeMap::new();
    for scene in scenes(listing).iter().filter(|scene| scene.is_object()) {
        let entity = scene
            .get("entity")
            .filter(|entity| entity.is_object())
            .cloned()
            .unwrap_or_else(|| json!({}));
        let listed = scene
            .get("parcels")
            .and_then(Value::as_array)
            .filter(|parcels| !parcels.is_empty());
        let Some(parcels) = listed.or_else(|| entity.get("pointers").and_then(Value::as_array))
        else {
            continue;
        };
        for parcel in parcels {
            index.insert(gd_str(Some(parcel)), entity.clone());
        }
    }
    index
}

/// The listing reshaped into the rows the realm stores, so applying a destination re-fetches
/// nothing. A scene with no entity id is not a row.
pub fn world_scene_rows(listing: &Value, base_content_url: &str) -> Vec<realm::EntityUrn> {
    scenes(listing)
        .iter()
        .filter(|scene| scene.is_object())
        .filter_map(|scene| {
            let entity_id = gd_str(scene.get("entityId"));
            (!entity_id.is_empty()).then(|| realm::EntityUrn {
                urn: format!("urn:decentraland:entity:{entity_id}"),
                entity_id,
                base_url: base_content_url.to_string(),
            })
        })
        .collect()
}

/// Worlds advertise `adapter: "fixed-adapter:signed-login:<url>"`; `fixedAdapter` is the older
/// key and wins when both are present. Any other shape means there is nowhere to prove a secret
/// against, which is the empty string.
pub fn comms_handshake_url(about: &Value) -> String {
    let Some(comms) = about.get("comms").filter(|comms| comms.is_object()) else {
        return String::new();
    };
    let mut adapter = gd_str(comms.get("fixedAdapter"));
    if adapter.is_empty() {
        adapter = gd_str(comms.get("adapter"));
    }
    let adapter = adapter.strip_prefix("fixed-adapter:").unwrap_or(&adapter);
    adapter
        .strip_prefix("signed-login:")
        .unwrap_or_default()
        .to_string()
}

/// The `origin` the comms handler expects: scheme and authority.
pub fn origin_of(url: &str) -> String {
    let scheme = if url.starts_with("http://") {
        "http://"
    } else {
        "https://"
    };
    let rest = realm::remove_scheme(url);
    let authority = rest.split('/').next().unwrap_or_default();
    format!("{scheme}{authority}")
}

/// Mirrors the signed-login metadata of the real connect, `isGuest` included: proving the secret
/// against a different shape than the connect sends would prove nothing.
pub fn handshake_metadata(realm_url: &str, secret: &str) -> String {
    let text = |value: &str| Value::String(value.to_string()).to_string();
    format!(
        "{{\"intent\":\"dcl:explorer:comms-handshake\",\"signer\":\"dcl:explorer\",\"isGuest\":true,\"origin\":{},\"secret\":{}}}",
        text(&origin_of(realm_url)),
        text(secret)
    )
}

/// A rate limiter or a wrong secret? The status code is lost in the rejection, so the message is
/// all there is to tell them apart.
pub fn is_rate_limited(error_message: &str) -> bool {
    let lower = error_message.to_lowercase();
    lower.contains("too many") || lower.contains("rate limit")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneInfo {
    pub id: String,
    pub title: String,
    pub creator: String,
    pub image_url: String,
    pub asset_count: usize,
}

/// What the loading screen shows, from a scene entity the existence check already read.
pub fn scene_info(entity: &Value, content_base_url: &str) -> SceneInfo {
    scene_info_with_files(entity, &format!("{content_base_url}contents/"))
}

/// Same, for a base that already addresses content files (a scene urn's `baseUrl`).
pub fn scene_info_with_files(entity: &Value, contents_base_url: &str) -> SceneInfo {
    let object = |value: Option<&Value>| value.filter(|v| v.is_object()).cloned();
    let metadata = object(entity.get("metadata")).unwrap_or_else(|| json!({}));
    let display = object(metadata.get("display")).unwrap_or_else(|| json!({}));
    let content = entity
        .get("content")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();

    let thumbnail = gd_str(display.get("navmapThumbnail"));
    let image_url = if thumbnail.is_empty() {
        String::new()
    } else {
        content
            .iter()
            .find(|file| file.is_object() && gd_str(file.get("file")) == thumbnail)
            .map(|file| format!("{contents_base_url}{}", gd_str(file.get("hash"))))
            .unwrap_or_default()
    };

    SceneInfo {
        id: gd_str(entity.get("id")),
        title: gd_str(display.get("title")),
        creator: creator_of(&metadata),
        image_url,
        asset_count: content.len(),
    }
}

/// `contact.name` is what Places reports as the contact; `owner` is the fallback.
fn creator_of(metadata: &Value) -> String {
    if let Some(contact) = metadata.get("contact").filter(|c| c.is_object()) {
        let name = gd_str(contact.get("name"));
        if !name.is_empty() {
            return name;
        }
    }
    gd_str(metadata.get("owner"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parcel_index_keeps_the_entity_and_reads_pointers_as_a_fallback() {
        let listing = json!({"scenes": [
            {"parcels": ["0,2", "0,3"], "entity": {"id": "bafk1", "content": []}},
            {"entity": {"id": "bafk2", "pointers": ["-11,3"], "content": []}},
            "not a scene",
        ]});
        let index = world_parcel_index(&listing);
        assert!(index.contains_key("0,2") && index.contains_key("0,3"));
        assert!(index.contains_key("-11,3"));
        assert!(!index.contains_key("123,123"));
        assert_eq!(index["0,2"]["id"], "bafk1");
    }

    #[test]
    fn an_unreadable_listing_indexes_nothing() {
        assert!(world_parcel_index(&json!({})).is_empty());
        assert!(world_parcel_index(&json!({"scenes": 3})).is_empty());
    }

    #[test]
    fn scene_rows_skip_scenes_without_an_entity_id() {
        let rows = world_scene_rows(
            &json!({"scenes": [{"entityId": "bafk1"}, {"entityId": ""}, {}]}),
            "https://worlds.example.com/contents/",
        );
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].urn, "urn:decentraland:entity:bafk1");
        assert_eq!(rows[0].base_url, "https://worlds.example.com/contents/");
    }

    #[test]
    fn handshake_url_prefers_the_older_key() {
        let url = |comms: Value| comms_handshake_url(&json!({ "comms": comms }));
        assert_eq!(
            url(json!({"adapter": "fixed-adapter:signed-login:https://a/"})),
            "https://a/"
        );
        assert_eq!(
            url(json!({
                "fixedAdapter": "signed-login:https://old/",
                "adapter": "fixed-adapter:signed-login:https://new/"
            })),
            "https://old/"
        );
        assert_eq!(url(json!({"adapter": "ws-room:wss://x"})), "");
        assert_eq!(comms_handshake_url(&json!({})), "");
    }

    #[test]
    fn origin_is_scheme_plus_authority() {
        assert_eq!(
            origin_of("https://worlds.example.com/world/x.dcl.eth/"),
            "https://worlds.example.com"
        );
        assert_eq!(origin_of("http://10.0.0.1:8000/x"), "http://10.0.0.1:8000");
        assert_eq!(
            origin_of("worlds.example.com"),
            "https://worlds.example.com"
        );
    }

    #[test]
    fn handshake_metadata_mirrors_the_signed_login_meta() {
        let meta: Value = serde_json::from_str(&handshake_metadata(
            "https://w.example.com/world/x/",
            "hu\"nter2",
        ))
        .unwrap();
        assert_eq!(meta["secret"], "hu\"nter2");
        assert_eq!(meta["intent"], "dcl:explorer:comms-handshake");
        assert_eq!(meta["signer"], "dcl:explorer");
        assert_eq!(meta["isGuest"], true);
        assert_eq!(meta["origin"], "https://w.example.com");
    }

    #[test]
    fn rate_limits_are_told_apart_from_wrong_secrets() {
        assert!(is_rate_limited("Too many shared-secret attempts"));
        assert!(is_rate_limited("Rate limit exceeded"));
        assert!(!is_rate_limited("InvalidAccessError"));
    }

    #[test]
    fn scene_info_resolves_the_thumbnail_through_its_hash() {
        let entity = json!({
            "id": "bafk",
            "content": [{"file": "scene.json", "hash": "h1"}, {"file": "thumb.png", "hash": "h2"}],
            "metadata": {"display": {"title": "Candy Sheep Meadow", "navmapThumbnail": "thumb.png"},
                         "contact": {"name": ""}, "owner": "0xabc"}
        });
        let info = scene_info(&entity, "https://peer.example.com/content/");
        assert_eq!(info.title, "Candy Sheep Meadow");
        assert_eq!(
            info.image_url,
            "https://peer.example.com/content/contents/h2"
        );
        assert_eq!(info.asset_count, 2);
        assert_eq!(info.id, "bafk");
        assert_eq!(info.creator, "0xabc");
    }

    #[test]
    fn a_bare_payload_yields_nothing_not_an_error() {
        let info = scene_info(&json!({"content": []}), "https://peer.example.com/content/");
        assert_eq!(info.image_url, "");
        assert_eq!(info.asset_count, 0);
        assert_eq!(info.title, "");
    }
}
