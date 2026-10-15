use dcl_core::{destination, realm};
use godot::classes::Json;
use godot::prelude::*;

/// Static access to the core's readers of what a place's servers say (`dcl_core::destination`).
#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub struct DclDestinationRules {
    _base: Base<RefCounted>,
}

fn to_json(value: &Variant) -> serde_json::Value {
    serde_json::from_str(&Json::stringify(value).to_string()).unwrap_or(serde_json::Value::Null)
}

fn to_variant(value: &serde_json::Value) -> Variant {
    Json::parse_string(&value.to_string())
}

#[godot_api]
impl DclDestinationRules {
    /// `{ "x,y": entity }` for every parcel of a world's `/scenes` listing.
    #[func]
    pub fn world_parcel_index(listing: VarDictionary) -> VarDictionary {
        let mut index = VarDictionary::new();
        for (parcel, entity) in destination::world_parcel_index(&to_json(&listing.to_variant())) {
            index.set(parcel.as_str(), &to_variant(&entity));
        }
        index
    }

    /// Rows `{ urn, entityId, baseUrl }` for every scene of a world's `/scenes` listing.
    #[func]
    pub fn world_scene_urns(listing: VarDictionary, base_content_url: GString) -> VarArray {
        destination::world_scene_rows(
            &to_json(&listing.to_variant()),
            &base_content_url.to_string(),
        )
        .into_iter()
        .map(|row| {
            let mut dict = VarDictionary::new();
            dict.set("urn", row.urn.as_str());
            dict.set("entityId", row.entity_id.as_str());
            dict.set("baseUrl", row.base_url.as_str());
            dict.to_variant()
        })
        .collect()
    }

    #[func]
    pub fn comms_handshake_url(about: VarDictionary) -> GString {
        destination::comms_handshake_url(&to_json(&about.to_variant()))
            .as_str()
            .into()
    }

    #[func]
    pub fn origin_of(url: GString) -> GString {
        destination::origin_of(&url.to_string()).as_str().into()
    }

    #[func]
    pub fn handshake_metadata(realm_url: GString, secret: GString) -> GString {
        destination::handshake_metadata(&realm_url.to_string(), &secret.to_string())
            .as_str()
            .into()
    }

    #[func]
    pub fn is_rate_limited(error_message: GString) -> bool {
        destination::is_rate_limited(&error_message.to_string())
    }

    /// `{ id, title, creator, image_url, asset_count }`; `contents_base_url` addresses content
    /// files when `is_content_base` is set, otherwise it is the content server root.
    #[func]
    pub fn scene_info(
        entity: VarDictionary,
        base_url: GString,
        is_content_base: bool,
    ) -> VarDictionary {
        let entity = to_json(&entity.to_variant());
        let base_url = base_url.to_string();
        let info = if is_content_base {
            destination::scene_info_with_files(&entity, &base_url)
        } else {
            destination::scene_info(&entity, &base_url)
        };
        let mut dict = VarDictionary::new();
        dict.set("id", info.id.as_str());
        dict.set("title", info.title.as_str());
        dict.set("creator", info.creator.as_str());
        dict.set("image_url", info.image_url.as_str());
        dict.set("asset_count", info.asset_count as i64);
        dict
    }

    /// `[min, max]` parcel bounds advertised by `/about`, with the Genesis City fallback.
    #[func]
    pub fn parse_bounds(about: VarDictionary) -> VarArray {
        let commit = realm::commit_from_about(&to_json(&about.to_variant()), &[]);
        let vec2i = |(x, y): (i32, i32)| Vector2i::new(x, y).to_variant();
        varray![&vec2i(commit.min_bounds), &vec2i(commit.max_bounds)]
    }
}
