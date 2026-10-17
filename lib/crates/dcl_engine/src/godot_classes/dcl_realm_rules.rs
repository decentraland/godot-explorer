use dcl_core::realm::{self, EntityUrn, ParamValue};
use godot::classes::Json;
use godot::prelude::*;

/// Static access to the core's realm rules (`dcl_core::realm`).
#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub struct DclRealmRules {
    _base: Base<RefCounted>,
}

fn urn_to_dict(urn: &EntityUrn) -> VarDictionary {
    let mut dict = VarDictionary::new();
    dict.set("urn", urn.urn.as_str());
    dict.set("entityId", urn.entity_id.as_str());
    dict.set("baseUrl", urn.base_url.as_str());
    dict
}

fn urn_from_dict(dict: &VarDictionary) -> Option<EntityUrn> {
    let text = |key: &str| dict.get(key).map(|v| v.to_string());
    Some(EntityUrn {
        urn: text("urn")?,
        entity_id: text("entityId")?,
        base_url: text("baseUrl")?,
    })
}

fn urns_to_array(urns: &[EntityUrn]) -> VarArray {
    urns.iter()
        .map(|urn| urn_to_dict(urn).to_variant())
        .collect()
}

fn vec2i(value: (i32, i32)) -> Vector2i {
    Vector2i::new(value.0, value.1)
}

fn about_to_json(about: &VarDictionary) -> serde_json::Value {
    serde_json::from_str(&Json::stringify(&about.to_variant()).to_string())
        .unwrap_or(serde_json::Value::Null)
}

#[godot_api]
impl DclRealmRules {
    #[func]
    pub fn dao_servers() -> PackedStringArray {
        realm::DAO_SERVERS
            .iter()
            .map(|s| GString::from(*s))
            .collect()
    }

    #[func]
    pub fn is_dcl_ens(name: GString) -> bool {
        realm::is_dcl_ens(&name.to_string())
    }

    #[func]
    pub fn is_genesis_city(name: GString) -> bool {
        realm::is_genesis_city(&name.to_string())
    }

    #[func]
    pub fn is_local_preview(name: GString) -> bool {
        realm::is_local_preview(&name.to_string())
    }

    #[func]
    pub fn dcl_world_url(name: GString) -> GString {
        realm::dcl_world_url(&name.to_string()).as_str().into()
    }

    #[func]
    pub fn ensure_reduce_url(url: GString) -> GString {
        realm::ensure_reduce_url(&url.to_string()).as_str().into()
    }

    #[func]
    pub fn ensure_dcl_ens(url: GString) -> GString {
        realm::ensure_dcl_ens(&url.to_string()).as_str().into()
    }

    #[func]
    pub fn ensure_remove_slash(text: GString) -> GString {
        realm::ensure_remove_slash(&text.to_string())
            .as_str()
            .into()
    }

    #[func]
    pub fn ensure_ends_with_slash(text: GString) -> GString {
        realm::ensure_ends_with_slash(&text.to_string())
            .as_str()
            .into()
    }

    #[func]
    pub fn ensure_starts_with_https(text: GString) -> GString {
        realm::ensure_starts_with_https(&text.to_string())
            .as_str()
            .into()
    }

    #[func]
    pub fn remove_scheme(text: GString) -> GString {
        realm::remove_scheme(&text.to_string()).as_str().into()
    }

    #[func]
    pub fn resolve_realm_url(value: GString) -> GString {
        realm::resolve_realm_url(&value.to_string()).as_str().into()
    }

    #[func]
    pub fn normalize_realm_url(value: GString) -> GString {
        realm::normalize_realm_url(&value.to_string())
            .as_str()
            .into()
    }

    /// `{ key: [value, …] }`; a key with no `=value` contributes `true`.
    #[func]
    pub fn get_params(url: GString) -> VarDictionary {
        let mut dict = VarDictionary::new();
        for (key, values) in realm::get_params(&url.to_string()) {
            let values: VarArray = values
                .into_iter()
                .map(|value| match value {
                    ParamValue::Text(text) => text.to_variant(),
                    ParamValue::Flag => true.to_variant(),
                })
                .collect();
            dict.set(key.as_str(), values);
        }
        dict
    }

    /// `{ urn, entityId, baseUrl }`, or null when the string is not an entity URN.
    #[func]
    pub fn parse_urn(urn: GString) -> Variant {
        realm::parse_urn(&urn.to_string())
            .map(|urn| urn_to_dict(&urn).to_variant())
            .unwrap_or_default()
    }

    /// Empty when the `/about` can be committed, otherwise the reason it cannot.
    #[func]
    pub fn about_problem(about: Variant) -> GString {
        let value = serde_json::from_str(&Json::stringify(&about).to_string())
            .unwrap_or(serde_json::Value::Null);
        realm::about_problem(&value).unwrap_or_default().into()
    }

    /// The realm fields derived from an `/about`: scene_urns, scenes_urn_override (null unless a
    /// world listing replaced the list), global_scene_urns, city_loader_content_base_url,
    /// lambda_server_base_url, realm_name, network_id, serves_local_scenes, min_bounds,
    /// max_bounds, content_base_url. `world_scenes` are parsed URN dictionaries.
    #[func]
    pub fn commit_from_about(about: VarDictionary, world_scenes: VarArray) -> VarDictionary {
        let world_scenes: Vec<EntityUrn> = world_scenes
            .iter_shared()
            .filter_map(|scene| urn_from_dict(&scene.try_to::<VarDictionary>().ok()?))
            .collect();
        let commit = realm::commit_from_about(&about_to_json(&about), &world_scenes);

        let mut dict = VarDictionary::new();
        dict.set("scene_urns", urns_to_array(&commit.scene_urns));
        dict.set(
            "scenes_urn_override",
            commit
                .scenes_urn_override
                .map(|urns| {
                    urns.iter()
                        .map(|urn| urn.as_str().to_variant())
                        .collect::<VarArray>()
                        .to_variant()
                })
                .unwrap_or_default(),
        );
        dict.set(
            "global_scene_urns",
            urns_to_array(&commit.global_scene_urns),
        );
        dict.set(
            "city_loader_content_base_url",
            commit.city_loader_content_base_url.as_str(),
        );
        dict.set(
            "lambda_server_base_url",
            commit.lambda_server_base_url.as_str(),
        );
        dict.set("realm_name", commit.realm_name.as_str());
        dict.set("network_id", commit.network_id);
        dict.set("serves_local_scenes", commit.serves_local_scenes);
        dict.set("min_bounds", vec2i(commit.min_bounds));
        dict.set("max_bounds", vec2i(commit.max_bounds));
        dict.set("content_base_url", commit.content_base_url.as_str());
        dict
    }
}
