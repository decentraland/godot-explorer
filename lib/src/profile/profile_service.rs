use crate::{
    avatars::dcl_user_profile::DclUserProfile,
    comms::profile::UserProfile,
    godot_classes::{dcl_global::DclGlobal, promise::Promise},
    scene_runner::tokio_runtime::TokioRuntime,
};
use anyhow::anyhow;
use godot::prelude::*;
use multihash_codetable::MultihashDigest;
use serde::Serialize;
use std::{io::Read, sync::Arc};

// ADR-290: Profile deployments no longer include snapshot content files.
// Profile images are served on-demand by the profile-images service.
#[derive(Serialize)]
struct Deployment<'a> {
    version: &'a str,
    #[serde(rename = "type")]
    ty: &'a str,
    pointers: Vec<String>,
    timestamp: u128,
    content: Vec<()>, // ADR-290: Empty content array, no snapshot files
    metadata: serde_json::Value,
}

#[derive(GodotClass)]
#[class(base=RefCounted)]
pub struct ProfileService {
    base: Base<RefCounted>,
}

#[godot_api]
impl IRefCounted for ProfileService {
    fn init(base: Base<RefCounted>) -> Self {
        Self { base }
    }
}

#[godot_api]
impl ProfileService {
    // ADR-290: Removed generate_snapshots parameter - snapshots are no longer uploaded
    #[func]
    pub fn async_deploy_profile(new_profile: Gd<DclUserProfile>) -> Gd<Promise> {
        // Default behavior: increment version
        Self::async_deploy_profile_with_version_control(new_profile, true)
    }

    // ADR-290: Removed generate_snapshots parameter - snapshots are no longer uploaded
    #[func]
    pub fn async_deploy_profile_with_version_control(
        mut new_profile: Gd<DclUserProfile>,
        increment_version: bool,
    ) -> Gd<Promise> {
        let promise = Promise::new_alloc();
        let promise_instance_id = promise.instance_id();

        // Get player identity
        let mut player_identity = DclGlobal::singleton().bind().get_player_identity();
        let is_guest = player_identity.bind().get_is_guest();

        // Handle guest profile
        if is_guest {
            let profile_dict = new_profile.bind().to_godot_dictionary();
            let mut config = DclGlobal::singleton().bind().get_config();
            config.set("guest_profile", &profile_dict.to_variant());
            config.call("save_to_settings_file", &[]);

            if increment_version {
                new_profile.bind_mut().increment_profile_version();
            }
            player_identity.bind_mut().set_profile(new_profile);

            let mut promise_clone = promise.clone();
            promise_clone.bind_mut().resolve();
            return promise;
        }

        // For non-guest profiles, prepare deployment
        let mut profile_binding = new_profile.bind_mut();
        if increment_version {
            profile_binding.increment_profile_version();
        }
        let profile = profile_binding.inner.clone();
        drop(profile_binding);

        let eth_address = player_identity.bind().get_address_str().to_string();
        let player_identity_id = player_identity.instance_id();
        let new_profile_id = new_profile.instance_id();

        // Get ephemeral auth chain before entering async block
        let ephemeral_auth_chain = match player_identity.bind().get_ephemeral_auth_chain() {
            Some(chain) => chain.clone(),
            None => {
                let mut promise_clone = promise.clone();
                promise_clone
                    .bind_mut()
                    .reject("No ephemeral auth chain available".into());
                return promise;
            }
        };

        // Get HTTP requester Arc and realm URL before entering async block
        let global = DclGlobal::singleton();
        let global_bind = global.bind();
        let http_requester = global_bind.get_http_requester();
        let http_requester_arc = http_requester.bind().get_http_queue_requester();
        let profile_content_url = global_bind
            .get_realm()
            .bind()
            .get_profile_content_url()
            .to_string();

        TokioRuntime::spawn(async move {
            // ADR-290: Snapshots are no longer uploaded with profile deployments
            let result = Self::prepare_and_deploy_profile_internal(
                http_requester_arc,
                ephemeral_auth_chain,
                profile,
                eth_address,
                profile_content_url,
            )
            .await;

            let Ok(mut promise) = Gd::<Promise>::try_from_instance_id(promise_instance_id) else {
                return;
            };

            match result {
                Ok(response) => {
                    // Update player identity with new profile
                    if let Ok(mut player_identity) = Gd::<
                        crate::auth::dcl_player_identity::DclPlayerIdentity,
                    >::try_from_instance_id(
                        player_identity_id
                    ) {
                        if let Ok(new_profile) =
                            Gd::<DclUserProfile>::try_from_instance_id(new_profile_id)
                        {
                            player_identity.bind_mut().set_profile(new_profile);
                        }
                    }

                    // Note: Clearing temporary lists should be done from the main thread
                    // The caller can handle this after the promise resolves

                    promise
                        .bind_mut()
                        .resolve_with_data(serde_json::to_string(&response).unwrap().to_variant());
                }
                Err(err) => {
                    promise.bind_mut().reject(GString::from(
                        format!("Failed to deploy profile: {}", err).as_str(),
                    ));
                }
            }
        });

        promise
    }

    #[func]
    pub fn async_fetch_profile(address: GString, lambda_server_url: GString) -> Gd<Promise> {
        let base_url = lambda_server_url
            .to_string()
            .trim_end_matches('/')
            .to_string();
        let url = format!("{}/profiles/{}", base_url, address);
        tracing::debug!("profile > fetching from: {}", url);
        let http_requester = DclGlobal::singleton().bind().get_http_requester();
        let promise = http_requester.bind().request_json(
            GString::from(url.as_str()),
            godot::classes::http_client::Method::GET,
            GString::new(),
            VarDictionary::new(),
        );
        promise
    }
}

impl ProfileService {
    // ADR-290: Simplified multipart data preparation - no snapshot files included
    fn prepare_multipart_data(
        cid: String,
        entity_authchain: crate::auth::wallet::SimpleAuthChain,
        deployment_bytes: Vec<u8>,
    ) -> Result<(Vec<u8>, String), anyhow::Error> {
        let mut form_data = multipart::client::lazy::Multipart::new();
        form_data.add_text("entityId", cid.clone());
        for (key, data) in entity_authchain.formdata() {
            form_data.add_text(key, data);
        }
        form_data.add_stream(
            cid,
            std::io::Cursor::new(deployment_bytes),
            Option::<&str>::None,
            None,
        );

        // ADR-290: No snapshot images are uploaded anymore

        let mut prepared = form_data.prepare()?;
        let boundary = prepared.boundary().to_string();
        let mut prepared_data = Vec::default();
        prepared.read_to_end(&mut prepared_data)?;

        let content_type = format!("multipart/form-data; boundary={}", boundary);
        Ok((prepared_data, content_type))
    }

    // ADR-290: Snapshots are no longer included in profile deployments
    async fn prepare_and_deploy_profile_internal(
        http_requester: Arc<crate::http_request::http_queue_requester::HttpQueueRequester>,
        ephemeral_auth_chain: crate::auth::ephemeral_auth_chain::EphemeralAuthChain,
        mut profile: UserProfile,
        eth_address: String,
        profile_content_url: String,
    ) -> Result<serde_json::Value, anyhow::Error> {
        // Update profile fields
        profile.content.user_id = Some(eth_address.clone());
        profile.content.eth_address.clone_from(&eth_address);

        // ADR-290: Remove snapshots from avatar before deployment
        profile.content.avatar.snapshots = None;

        // Prepare deployment data
        let unix_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();

        // ADR-290: Empty content array - no snapshot files uploaded
        let deployment = serde_json::to_string(&Deployment {
            version: "v3",
            ty: "profile",
            pointers: vec![eth_address.clone()],
            timestamp: unix_time,
            content: vec![], // ADR-290: No snapshot content files
            metadata: serde_json::json!({
                "avatars": [profile.content]
            }),
        })?;

        let hash = multihash_codetable::Code::Sha2_256.digest(deployment.as_bytes());
        let cid = cid::Cid::new_v1(0x55, hash).to_string();

        // Use the provided ephemeral auth chain

        let entity_id_signature = ephemeral_auth_chain
            .ephemeral_wallet()
            .sign_message(cid.clone())
            .await?;

        let mut entity_authchain = ephemeral_auth_chain.auth_chain().clone();
        entity_authchain.add_signed_entity(cid.clone(), entity_id_signature);

        // Prepare multipart form data (ADR-290: no snapshot files)
        let (prepared_data, content_type) =
            Self::prepare_multipart_data(cid.clone(), entity_authchain, deployment.into_bytes())?;

        // Deploy to server
        let url = format!("{}entities/", profile_content_url);
        tracing::debug!("profile > deploying to: {}", url);

        // Deploy via HTTP request using the Arc<HttpQueueRequester>
        let headers_map = {
            let mut map = std::collections::HashMap::new();
            map.insert("Content-Type".to_string(), content_type);
            Some(map)
        };

        let request_option = crate::http_request::request_response::RequestOption::new(
            0,
            url.clone(),
            http::Method::POST,
            crate::http_request::request_response::ResponseType::AsString,
            Some(prepared_data),
            headers_map,
            None,
        );

        let response = http_requester
            .request(request_option, 1)
            .await
            .map_err(|e| anyhow!("Failed to deploy profile: {:?}", e))?;

        // Check HTTP status code
        let status_code = response.status_code();
        if !(200..=299).contains(&status_code) {
            let error_body = response.get_response_as_string().to_string();
            tracing::error!(
                "profile > deploy failed - HTTP {}: {}",
                status_code,
                error_body
            );
            return Err(anyhow!(
                "Deploy failed with HTTP {}: {}",
                status_code,
                error_body
            ));
        }

        // Parse response
        let response_variant = response.get_response_as_string();
        let response_str = if response_variant.is_nil() {
            return Err(anyhow!("Invalid response format"));
        } else {
            response_variant.to_string()
        };
        let response_json: serde_json::Value = serde_json::from_str(&response_str)
            .map_err(|e| anyhow!("Failed to parse deployment response: {}", e))?;

        if response_json.get("creationTimestamp").is_none() {
            tracing::error!(
                "profile > deploy failed - invalid response: {}",
                response_str
            );
            return Err(anyhow!(
                "Invalid deployment response: missing creationTimestamp"
            ));
        }

        tracing::info!("profile > deploy succeeded for: {}", eth_address);
        Ok(response_json)
    }
}
