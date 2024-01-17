use super::{content_mapping::DclContentMappingAndUrl, content_provider::ContentProviderContext};
use crate::http_request::request_response::{RequestOption, ResponseEnum, ResponseType};
use godot::{
    builtin::{meta::ToGodot, Dictionary, GString, Variant, VariantArray},
    engine::{global::Error, Json},
};
use serde::Serialize;
use std::collections::HashMap;

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
        ResponseType::AsString,
        Some(payload),
        Some(headers),
        None,
    );

    let response = ctx
        .http_queue_requester
        .request(request_option, 0)
        .await
        .map_err(|e| anyhow::Error::msg(e.error_message))?;

    let pointers_result = match response.response_data {
        Ok(ResponseEnum::String(result)) => {
            let mut json = Json::new();
            let err = json.parse(GString::from(result));

            if err != Error::OK {
                Err("Couldn't parse wearable entities response".to_string())
            } else {
                match json.get_data().try_to::<VariantArray>() {
                    Ok(array) => Ok(array),
                    Err(_err) => Err("Pointers response is not an array".to_string()),
                }
            }
        }
        _ => Err("Invalid response".to_string()),
    }
    .map_err(anyhow::Error::msg)?;

    let mut dictionary_result = Dictionary::new();
    for item in pointers_result.iter_shared() {
        let Ok(mut dict) = item.try_to::<Dictionary>() else {
            continue;
        };

        let Some(pointers) = dict.get("pointers") else {
            continue;
        };
        let Ok(pointers) = pointers.try_to::<VariantArray>() else {
            continue;
        };

        for pointer in pointers.iter_shared() {
            dictionary_result.set(pointer.to_string().to_lowercase(), item.clone());
        }

        let Some(content_array) = dict.get("content") else {
            continue;
        };
        let Ok(content_array) = content_array.try_to::<VariantArray>() else {
            continue;
        };

        let mut content_mapping_hashmap = HashMap::new();
        for content_item in content_array.iter_shared() {
            let Ok(content_dict) = content_item.try_to::<Dictionary>() else {
                continue;
            };
            let Some(file) = content_dict.get("file") else {
                continue;
            };
            let Ok(file) = file.try_to::<GString>() else {
                continue;
            };
            let Some(hash) = content_dict.get("hash") else {
                continue;
            };
            let Ok(hash) = hash.try_to::<GString>() else {
                continue;
            };
            content_mapping_hashmap.insert(file.to_string().to_lowercase(), hash.to_string());
        }

        dict.set(
            "content",
            DclContentMappingAndUrl::from_values(
                ipfs_content_base_url.clone(),
                content_mapping_hashmap,
            ),
        );
    }

    for pointer in pointers {
        let pointer = pointer.to_lowercase();
        if !dictionary_result.contains_key(pointer.as_str()) {
            dictionary_result.set(pointer, Variant::nil());
        }
    }

    Ok(Some(dictionary_result.to_variant()))
}
