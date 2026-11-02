use std::sync::Arc;

use ethers_core::types::H160;
use serde::Deserialize;

use crate::{
    comms::profile::{ProfileNotFound, SerializedProfile, UserProfile},
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
    request_registry_profile_by_url(
        user_id,
        profile_base_url,
        http_requester,
        urls::asset_bundle_registry_profiles(),
    )
    .await
}

pub(crate) async fn request_registry_profile_by_url(
    user_id: H160,
    profile_base_url: &str,
    http_requester: Arc<HttpQueueRequester>,
    url: String,
) -> Result<UserProfile, anyhow::Error> {
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

    if response.status_code == http::StatusCode::NOT_FOUND {
        return Err(ProfileNotFound.into());
    }
    if !response.status_code.is_success() {
        return Err(anyhow::anyhow!("registry HTTP {}", response.status_code));
    }
    match &response.response_data {
        Ok(crate::http_request::request_response::ResponseEnum::String(json)) => {
            let entries: Vec<RegistryProfileEntry> = serde_json::from_str(json.as_str())
                .map_err(|e| anyhow::anyhow!("error parsing registry response: {}", e))?;
            let mut content = entries
                .into_iter()
                .next()
                .and_then(|e| e.avatars.into_iter().next())
                .ok_or(ProfileNotFound)?;
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
    let url = lambda_profile_url(lamda_server_base_url, user_id);
    request_lambda_profile_by_url(url, profile_base_url, http_requester).await
}

/// Builds the lambda `profiles/` URL tolerating both trailing-slash styles:
/// realm URLs end with `/`, but peer-advertised `lambdasEndpoint` LiveKit
/// metadata (Unity) comes without one — bare concatenation used to produce
/// `…/lambdasprofiles/0x…` (404) and stall remote avatars on the loading
/// ghost for the whole retry chain.
pub(crate) fn lambda_profile_url(lamda_server_base_url: &str, user_id: H160) -> String {
    format!(
        "{}/profiles/{:#x}",
        lamda_server_base_url.trim_end_matches('/'),
        user_id
    )
}

// Fetches and parses a lambda profile from an explicit URL. Used both by the
// h160 path above and by the default-profile path (`profiles/default{N}`),
// whose id is not an eth address.
pub(crate) async fn request_lambda_profile_by_url(
    url: String,
    profile_base_url: &str,
    http_requester: Arc<HttpQueueRequester>,
) -> Result<UserProfile, anyhow::Error> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_profile_is_distinct_from_server_and_parse_failures() {
        use dcl_core::http::request_response::{HttpResponse, ResponseEnum};
        for (status, body, missing) in [
            (404, "not found", true),
            (200, r#"{"avatars":[]}"#, true),
            (503, r#"{"avatars":[]}"#, false),
            (200, "invalid json", false),
        ] {
            let response = HttpResponse {
                request_option: RequestOption::new(
                    0,
                    String::new(),
                    http::Method::GET,
                    ResponseType::AsString,
                    None,
                    None,
                    None,
                ),
                status_code: http::StatusCode::from_u16(status).unwrap(),
                headers: None,
                response_data: Ok(ResponseEnum::String(body.into())),
            };
            let error = UserProfile::from_lambda_response(&response, "").unwrap_err();
            assert_eq!(error.is::<ProfileNotFound>(), missing);
        }
    }

    #[test]
    fn lambda_profile_url_tolerates_both_trailing_slash_styles() {
        let user = H160::from_low_u64_be(0x1234);
        let slashed = lambda_profile_url("https://peer.decentraland.org/lambdas/", user);
        let bare = lambda_profile_url("https://peer.decentraland.org/lambdas", user);
        assert_eq!(slashed, bare);
        assert_eq!(
            slashed,
            "https://peer.decentraland.org/lambdas/profiles/0x0000000000000000000000000000000000001234"
        );
    }
}
