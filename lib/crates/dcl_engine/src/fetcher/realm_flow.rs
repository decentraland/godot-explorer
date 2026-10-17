//! Committing a resolved realm: the `/about` the navigation already read becomes the realm's
//! state, and the player is placed in it.

use super::globals;
use dcl_core::{
    destination::world_scene_rows,
    realm::{self, EntityUrn},
    urls,
};
use godot::classes::Json;
use godot::prelude::*;
use serde_json::Value;

use super::http;
use crate::godot_classes::dcl_realm::{DclRealm, RealmData};

/// What a navigation hands the realm.
pub struct Landing {
    pub realm_string: String,
    pub realm_url: String,
    pub about: Value,
    /// A world's `/scenes` listing, when the resolve already read it.
    pub listing: Option<Value>,
    /// Empty unless the world is password-protected, which keeps `secret` off the wire for
    /// every other realm (#2651).
    pub credential: String,
    /// Only a join asks for the spawn point: a named parcel lands there, and a reload leaves
    /// the player standing where they are.
    pub wants_spawn: bool,
}

fn to_variant(value: &Value) -> Variant {
    Json::parse_string(&value.to_string())
}

fn urn_row(urn: &EntityUrn) -> VarDictionary {
    let mut row = VarDictionary::new();
    row.set("urn", urn.urn.as_str());
    row.set("entityId", urn.entity_id.as_str());
    row.set("baseUrl", urn.base_url.as_str());
    row
}

fn rows(urns: &[EntityUrn]) -> Array<VarDictionary> {
    urns.iter().map(urn_row).collect()
}

/// Applies a landing to `realm`; false when the realm could not be committed.
pub async fn apply(mut realm: Gd<DclRealm>, landing: Landing) -> bool {
    realm.signals().realm_changing().emit();
    realm.bind_mut().set_credential(&landing.credential);
    commit(realm, landing).await
}

async fn commit(mut realm_node: Gd<DclRealm>, landing: Landing) -> bool {
    let Landing {
        realm_string,
        realm_url,
        mut about,
        listing,
        wants_spawn,
        ..
    } = landing;

    let realm_name = about
        .pointer("/configurations/realmName")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    // A world's /about names only its last deployed scene; the full list is on its /scenes.
    let mut world_scenes = Vec::new();
    if realm::is_dcl_ens(&realm_name) {
        // Content comes from the worlds server, not content.publicUrl (the Genesis peer).
        let content_url = urls::worlds_content_server().replace("/world/", "/") + "contents/";
        if let Some(listing) = &listing {
            world_scenes = world_scene_rows(listing, &content_url);
        }
        if world_scenes.is_empty() {
            world_scenes = fetch_world_scenes(&realm_name, &content_url).await;
        }
    }

    let commit = realm::commit_from_about(&about, &world_scenes);
    if let Some(urns) = &commit.scenes_urn_override {
        // So the scene fetcher reads the full list from realm_about.
        about["configurations"]["scenesUrn"] = urns.clone().into();
    }
    let realm_about = to_variant(&about)
        .try_to::<VarDictionary>()
        .unwrap_or_default();

    // A local preview advertises the parcels it serves: the only signal on the QR/deeplink
    // preview path, which never sets `--preview`. Pulse uses it to apply the same explicit
    // opt-in rule there as on desktop.
    globals::comms()
        .bind_mut()
        .set_local_scene_development(commit.serves_local_scenes);

    let last_urn = commit.scene_urns.last().cloned();
    let resolve_position = wants_spawn || realm::is_dcl_ens(&realm_name);
    realm_node.bind_mut().commit(RealmData {
        realm_string,
        realm_url: realm_url.clone(),
        about: realm_about,
        scene_urns: rows(&commit.scene_urns),
        global_scene_urns: rows(&commit.global_scene_urns),
        city_loader_content_base_url: commit.city_loader_content_base_url,
        lambda_server_base_url: commit.lambda_server_base_url,
        realm_name,
        network_id: commit.network_id,
        min_bounds: Vector2i::new(commit.min_bounds.0, commit.min_bounds.1),
        max_bounds: Vector2i::new(commit.max_bounds.0, commit.max_bounds.1),
        content_base_url: commit.content_base_url,
    });

    // Worlds always resolve the spawn from scene metadata (the caller may not know the parcel);
    // other realms only when asked.
    if let (Some(urn), true) = (last_urn, resolve_position) {
        request_set_position(&urn).await;
    }

    globals::engine().bind().set_setting(
        "last_realm_joined".into(),
        realm_url.as_str().to_variant(),
        true,
    );
    globals::metrics().bind_mut().update_realm(realm_url);

    realm_node.signals().realm_changed().emit();
    true
}

async fn fetch_world_scenes(world_name: &str, content_url: &str) -> Vec<EntityUrn> {
    let url = format!("{}/scenes", realm::dcl_world_url(world_name));
    let Some(listing) = http::get_json(&url).await else {
        tracing::error!("[REALM] Failed to fetch world scenes");
        return Vec::new();
    };
    let result = world_scene_rows(&listing, content_url);
    if result.len() > 1 {
        godot_print!(
            "[REALM] Loaded {} scenes for world {world_name}",
            result.len()
        );
    }
    result
}

/// Teleports to the base parcel of the scene a world join lands in.
async fn request_set_position(urn: &EntityUrn) {
    let Some(entity) = http::get_json(&format!("{}{}", urn.base_url, urn.entity_id)).await else {
        tracing::error!("Rejected request async_request_set_position: {}", urn.urn);
        return;
    };
    let base = entity
        .pointer("/metadata/scene/base")
        .and_then(Value::as_str)
        .unwrap_or("0,0");
    let mut parts = base
        .split(',')
        .map(|part| part.trim().parse::<i32>().unwrap_or(0));
    let (x, y) = (parts.next().unwrap_or(0), parts.next().unwrap_or(0));
    crate::world::api::teleport_to(Vector2i::new(x, y));
}

/// Reads the scene urn of a world without making it the current realm: the portable
/// experience that lives in it is spawned from there.
pub async fn world_scene_urns(world: &str) -> Option<Vec<String>> {
    let url = realm::normalize_realm_url(world);
    let about = http::get_json(&format!("{url}about")).await?;
    if realm::about_problem(&about).is_some() {
        return None;
    }
    let content_url = urls::worlds_content_server().replace("/world/", "/") + "contents/";
    let rows = fetch_world_scenes(world, &content_url).await;
    let commit = realm::commit_from_about(&about, &rows);
    Some(match commit.scenes_urn_override {
        Some(urns) => urns,
        None => about
            .pointer("/configurations/scenesUrn")
            .and_then(Value::as_array)
            .map(|urns| {
                urns.iter()
                    .filter_map(|u| u.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default(),
    })
}

/// Makes `realm_string` the current realm with no resolve and no explorer: the offline scene
/// renderer's entry. Private worlds are refused up front; a failure is announced through
/// `realm_change_failed`.
pub async fn set_realm(realm_node: Gd<DclRealm>, realm_string: &str) -> bool {
    let realm_url = realm::normalize_realm_url(realm_string);
    let world = super::world_access::world_name_from_realm(realm_string);
    if !world.is_empty() {
        let address = globals::player_identity()
            .bind()
            .get_address_str()
            .to_string();
        if !super::world_access::is_allowed(&world, &address).await {
            globals::scene_runner()
                .bind_mut()
                .loading_realm_change_failed(
                    realm_string.into(),
                    "private_world_access_denied".into(),
                );
            realm_node
                .signals()
                .realm_access_denied()
                .emit(realm_string, &world);
            return false;
        }
    }

    realm_node.signals().realm_changing().emit();
    let about = http::get_json(&format!("{realm_url}about")).await;
    let problem = match &about {
        None => Some("invalid response".to_string()),
        Some(about) => realm::about_problem(about).map(str::to_string),
    };
    if let Some(reason) = problem {
        tracing::error!("[REALM] Rejected request change realm to: {realm_string}: {reason}");
        globals::scene_runner()
            .bind_mut()
            .loading_realm_change_failed(realm_string.into(), reason.as_str().into());
        realm_node
            .signals()
            .realm_change_failed()
            .emit(realm_string, &reason);
        return false;
    }
    globals::scene_runner().bind_mut().loading_mark_about_end();
    let landing = Landing {
        realm_string: realm_string.to_string(),
        realm_url,
        about: about.unwrap_or_default(),
        listing: None,
        credential: String::new(),
        wants_spawn: false,
    };
    commit(realm_node, landing).await
}
