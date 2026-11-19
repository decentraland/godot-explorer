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
    let unix_time = crate::utils::clock::unix_time_ms();

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

    let mut fields = vec![("entityId".to_owned(), cid.clone())];
    fields.extend(entity_authchain.formdata());
    let form = dcl_core::profile::multipart::build(&fields, &cid, deployment.as_bytes());

    // ADR-290: No snapshot images are uploaded anymore

    Ok((form.content_type, form.body))
}
