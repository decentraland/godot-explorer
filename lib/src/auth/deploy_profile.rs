use std::io::Read;

use multihash_codetable::MultihashDigest;
use serde::Serialize;

use crate::comms::profile::UserProfile;

use super::ephemeral_auth_chain::EphemeralAuthChain;

// ADR-290: Profile deployments no longer include snapshot content files.
// Profile images are served on-demand by the profile-images service.
#[derive(Serialize)]
pub struct Deployment<'a> {
    version: &'a str,
    #[serde(rename = "type")]
    ty: &'a str,
    pointers: Vec<String>,
    timestamp: u128,
    content: Vec<()>, // ADR-290: Empty content array, no snapshot files
    metadata: serde_json::Value,
}

pub struct PrepareDeployProfileData {
    pub content_type_str: String,
    pub prepared_data: Vec<u8>,
}

// ADR-290: Snapshots are no longer included in profile deployments.
// Profile images are served on-demand by the profile-images service.
pub async fn prepare_deploy_profile(
    ephemeral_auth_chain: EphemeralAuthChain,
    mut profile: UserProfile,
) -> Result<(String, Vec<u8>), anyhow::Error> {
    let unix_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();

    // ADR-290: Remove snapshots from avatar before deployment
    profile.content.avatar.snapshots = None;

    let user_id = profile
        .content
        .user_id
        .as_ref()
        .unwrap_or(&profile.content.eth_address)
        .clone();

    // ADR-290: Empty content array - no snapshot files uploaded
    let deployment = serde_json::to_string(&Deployment {
        version: "v3",
        ty: "profile",
        pointers: vec![user_id],
        timestamp: unix_time,
        content: vec![], // ADR-290: No snapshot content files
        metadata: serde_json::json!({
            "avatars": [
                profile.content
            ]
        }),
    })?;

    let hash = multihash_codetable::Code::Sha2_256.digest(deployment.as_bytes());
    let cid = cid::Cid::new_v1(0x55, hash).to_string();
    let entity_id_signature = ephemeral_auth_chain
        .ephemeral_wallet()
        .sign_message(cid.clone())
        .await?;

    let mut entity_authchain = ephemeral_auth_chain.auth_chain().clone();
    entity_authchain.add_signed_entity(cid.clone(), entity_id_signature);

    let mut form_data = multipart::client::lazy::Multipart::new();
    form_data.add_text("entityId", cid.clone());
    for (key, data) in entity_authchain.formdata() {
        form_data.add_text(key, data);
    }
    form_data.add_stream(
        cid,
        std::io::Cursor::new(deployment.into_bytes()),
        Option::<&str>::None,
        None,
    );

    // ADR-290: No snapshot images are uploaded anymore

    let mut prepared = form_data.prepare()?;
    let mut prepared_data = Vec::default();
    prepared.read_to_end(&mut prepared_data)?;

    let content_type_str = format!("multipart/form-data; boundary={}", prepared.boundary());
    Ok((content_type_str, prepared_data))
}
