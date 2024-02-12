use super::{
    content_mapping::{ContentMappingAndUrl, ContentMappingAndUrlRef},
    content_provider::ContentProviderContext,
};
use crate::{
    dcl::common::{content_entity::EntityDefinitionJson, wearable::WearableEntityMetadata},
    http_request::request_response::{RequestOption, ResponseEnum, ResponseType},
};
use godot::{
    bind::GodotClass,
    builtin::{meta::ToGodot, Variant},
    obj::Gd,
};
use serde::Serialize;
use std::{collections::HashMap, sync::Arc};

#[derive(Serialize)]
struct EntitiesRequest {
    pointers: Vec<String>,
}

pub async fn request_wearables(
    content_server_base_url: String,
    ipfs_content_base_url: String,
    pointers: Vec<String>,
    ctx: ContentProviderContext,
) -> Result<Option<Variant>, anyhow::Error> {
    let url = format!("{content_server_base_url}entities/active");
    let headers = vec![("Content-Type: application/json".to_string())];
    let payload = serde_json::to_string(&EntitiesRequest {
        pointers: pointers.clone(),
    })
    .expect("serialize vec<string>")
    .into_bytes();
    let request_option = RequestOption::new(
        0,
        url,
        http::Method::POST,
        ResponseType::AsJson,
        Some(payload),
        Some(headers),
        None,
    );

    let response = ctx
        .http_queue_requester
        .request(request_option, 0)
        .await
        .map_err(|e| anyhow::Error::msg(e.error_message))?;

    let response = response.response_data.map_err(anyhow::Error::msg)?;

    let ResponseEnum::Json(result) = response else {
        return Err(anyhow::Error::msg("Invalid response"));
    };

    let mut result = result?;

    let Some(entity_pointers) = result.as_array_mut() else {
        return Err(anyhow::Error::msg("Invalid response"));
    };

    let mut wearable_map = HashMap::new();
    for pointer in entity_pointers.iter_mut() {
        let Ok(wearable_data) = WearableEntityDefinition::from_json_ex(
            None,
            ipfs_content_base_url.clone(),
            pointer.take(),
        ) else {
            continue;
        };
        wearable_map.insert(wearable_data.id.clone(), Arc::new(wearable_data));
    }

    Ok(Some(
        WearableManyResolved::from_gd(wearable_map).to_variant(),
    ))
}

pub struct WearableEntityDefinition {
    pub id: String,
    pub entity_definition_json: EntityDefinitionJson,
    pub wearable: WearableEntityMetadata,
    pub content_mapping: ContentMappingAndUrlRef,
}

impl WearableEntityDefinition {
    fn from_json_ex(
        id: Option<String>,
        base_url: String,
        json: serde_json::Value,
    ) -> Result<WearableEntityDefinition, anyhow::Error> {
        let mut entity_definition_json = serde_json::from_value::<EntityDefinitionJson>(json)?;
        let id = id.unwrap_or_else(|| entity_definition_json.id.take().unwrap_or_default());
        let metadata = entity_definition_json
            .metadata
            .take()
            .ok_or(anyhow::Error::msg("missing entity metadata"))?;
        let wearable = serde_json::from_value::<WearableEntityMetadata>(metadata)?;

        let content_mapping_vec = std::mem::take(&mut entity_definition_json.content);
        let content_mapping = Arc::new(ContentMappingAndUrl {
            base_url,
            content: HashMap::from_iter(
                content_mapping_vec
                    .into_iter()
                    .map(|item| (item.file.to_lowercase(), item.hash)),
            ),
        });

        Ok(WearableEntityDefinition {
            id,
            entity_definition_json,
            wearable,
            content_mapping,
        })
    }
}

#[derive(GodotClass)]
#[class(base=RefCounted)]
pub struct WearableManyResolved {
    pub wearable_map: HashMap<String, Arc<WearableEntityDefinition>>,
}

impl WearableManyResolved {
    pub fn from_gd(wearable_map: HashMap<String, Arc<WearableEntityDefinition>>) -> Gd<Self> {
        Gd::from_init_fn(|_base| Self { wearable_map })
    }
}

#[derive(GodotClass)]
#[class(base=RefCounted)]
pub struct DclWearableEntityDefinition {
    pub inner: Arc<WearableEntityDefinition>,
}

impl DclWearableEntityDefinition {
    pub fn from_gd(inner: Arc<WearableEntityDefinition>) -> Gd<Self> {
        Gd::from_init_fn(|_base| Self { inner })
    }
}
