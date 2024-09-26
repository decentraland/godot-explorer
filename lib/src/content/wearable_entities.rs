use super::content_provider::ContentProviderContext;
use crate::{
    avatars::item::ItemEntityDefinition,
    http_request::request_response::{RequestOption, ResponseEnum, ResponseType},
};
use godot::{
    builtin::{meta::ToGodot, Variant},
    obj::Gd,
    prelude::*,
};
use serde::Serialize;
use std::{collections::HashMap, sync::Arc};

#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub struct WearableManyResolved {
    pub wearable_map: HashMap<String, Arc<ItemEntityDefinition>>,
}

impl WearableManyResolved {
    pub fn from_gd(wearable_map: HashMap<String, Arc<ItemEntityDefinition>>) -> Gd<Self> {
        Gd::from_init_fn(|_base| Self { wearable_map })
    }
}

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
        match ItemEntityDefinition::from_json_ex(ipfs_content_base_url.clone(), pointer.take()) {
            Ok(wearable_data) => {
                wearable_map.insert(wearable_data.id.clone(), Arc::new(wearable_data));
            }
            Err(e) => {
                tracing::error!("Error parsing wearable data: {:?}", e);
            }
        }
    }

    Ok(Some(
        WearableManyResolved::from_gd(wearable_map).to_variant(),
    ))
}
