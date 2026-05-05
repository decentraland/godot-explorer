use std::sync::Arc;

use ethers_core::types::H160;
use serde::Deserialize;

use crate::{
    comms::profile::{SerializedProfile, UserProfile},
    godot_classes::dcl_global::DclGlobal,
    http_request::{
        http_queue_requester::HttpQueueRequester,
        request_response::{RequestOption, ResponseType},
    },
    urls,
};

#[derive(Deserialize)]
struct RegistryProfileEntry {
    avatars: Vec<SerializedProfile>,
}

pub(crate) fn prepare_request_requirements() -> (String, String, Arc<HttpQueueRequester>) {
    let global = DclGlobal::singleton();
    let realm = global.bind().get_realm();
    let lamda_server_base_url = realm.bind().get_lambda_server_base_url().to_string();
    let profile_base_url = realm.bind().get_profile_content_url().to_string();
    let http_requester = global
        .bind()
        .get_http_requester()
        .bind()
        .get_http_queue_requester();

    let lamda_server_base_url = if lamda_server_base_url.is_empty() {
        urls::peer_lambdas()
    } else {
        lamda_server_base_url
    };

    (lamda_server_base_url, profile_base_url, http_requester)
}

/// Fetch a profile from the asset-bundle-registry centralized endpoint.
/// The registry polls all DAO catalysts every ~5 seconds, so it returns
/// the latest deployed profile regardless of which catalyst the user deployed to.
pub(crate) async fn request_registry_profile(
    user_id: H160,
    profile_base_url: &str,
    http_requester: Arc<HttpQueueRequester>,
) -> Result<UserProfile, anyhow::Error> {
    let url = urls::asset_bundle_registry_profiles();
    let body = serde_json::json!({ "ids": [format!("{:#x}", user_id)] }).to_string();
    let headers = {
        let mut map = std::collections::HashMap::new();
        map.insert("Content-Type".to_string(), "application/json".to_string());
        Some(map)
    };

    let response = http_requester
        .request(
            RequestOption::new(
                0,
                url,
                http::Method::POST,
                ResponseType::AsString,
                Some(body.into_bytes()),
                headers,
                None,
            ),
            0,
        )
        .await
        .map_err(|v| anyhow::Error::msg(v.error_message))?;

    match &response.response_data {
        Ok(crate::http_request::request_response::ResponseEnum::String(json)) => {
            let entries: Vec<RegistryProfileEntry> = serde_json::from_str(json.as_str())
                .map_err(|e| anyhow::anyhow!("error parsing registry response: {}", e))?;
            let mut content = entries
                .into_iter()
                .next()
                .and_then(|e| e.avatars.into_iter().next())
                .ok_or_else(|| anyhow::anyhow!("profile not found in registry"))?;
            content.convert_snapshots();
            Ok(UserProfile {
                version: content.version as u32,
                content,
                base_url: format!("{}contents/", profile_base_url).to_owned(),
            })
        }
        Err(e) => Err(anyhow::anyhow!("registry not reached: {:?}", e)),
        _ => Err(anyhow::anyhow!("registry not reached")),
    }
}

pub(crate) async fn request_lambda_profile(
    user_id: H160,
    lamda_server_base_url: &str,
    profile_base_url: &str,
    http_requester: Arc<HttpQueueRequester>,
) -> Result<UserProfile, anyhow::Error> {
    let url = format!("{}profiles/{:#x}", lamda_server_base_url, user_id);
    let response = http_requester
        .request(
            RequestOption::new(
                0,
                url,
                http::Method::GET,
                ResponseType::AsString,
                None,
                None,
                None,
            ),
            0,
        )
        .await
        .map_err(|v| anyhow::Error::msg(v.error_message))?;

    UserProfile::from_lambda_response(&response, profile_base_url)
}
