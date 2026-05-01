use std::sync::Arc;

use ethers_core::types::H160;

use crate::{
    comms::profile::UserProfile,
    godot_classes::dcl_global::DclGlobal,
    http_request::{
        http_queue_requester::HttpQueueRequester,
        request_response::{RequestOption, ResponseType},
    },
    urls,
};

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
