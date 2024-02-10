use std::io::Read;

use anyhow::anyhow;

use multihash_codetable::MultihashDigest;
use serde::Serialize;

use crate::{comms::profile::UserProfile, dcl::common::content_entity::TypedIpfsRef};

use super::ephemeral_auth_chain::EphemeralAuthChain;

#[derive(Serialize)]
pub struct Deployment<'a> {
    version: &'a str,
    #[serde(rename = "type")]
    ty: &'a str,
    pointers: Vec<String>,
    timestamp: u128,
    content: Vec<TypedIpfsRef>,
    metadata: serde_json::Value,
}

pub struct PrepareDeployProfileData {
    pub content_type_str: String,
    pub prepared_data: Vec<u8>,
}

pub async fn prepare_deploy_profile(
    ephemeral_auth_chain: EphemeralAuthChain,
    profile: UserProfile,
) -> Result<(String, Vec<u8>), anyhow::Error> {
    let unix_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let snapshots = profile
        .content
        .avatar
        .snapshots
        .as_ref()
        .ok_or(anyhow!("no snapshots"))?
        .clone();

    let user_id = profile
        .content
        .user_id
        .as_ref()
        .unwrap_or(&profile.content.eth_address)
        .clone();

    let deployment = serde_json::to_string(&Deployment {
        version: "v3",
        ty: "profile",
        pointers: vec![user_id],
        timestamp: unix_time,
        content: vec![
            TypedIpfsRef {
                file: "body.png".to_owned(),
                hash: snapshots.body,
            },
            TypedIpfsRef {
                file: "face256.png".to_owned(),
                hash: snapshots.face256,
            },
        ],
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

    // todo: add images

    let mut prepared = form_data.prepare()?;
    let mut prepared_data = Vec::default();
    prepared.read_to_end(&mut prepared_data)?;

    let content_type_str = format!("multipart/form-data; boundary={}", prepared.boundary());
    Ok((content_type_str, prepared_data))
}
