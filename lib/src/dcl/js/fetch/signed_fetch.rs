use std::{cell::RefCell, rc::Rc, sync::Arc};

use deno_core::{error::AnyError, op2, OpState};
use http::Uri;

use crate::{
    auth::{ephemeral_auth_chain::EphemeralAuthChain, wallet::sign_request},
    dcl::DclSceneRealmData,
    realm::scene_definition::SceneEntityDefinition,
};

use serde::Serialize;

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct SignedFetchMetaRealm {
    hostname: String,
    protocol: String,
    server_name: String,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct SignedFetchMeta {
    origin: Option<String>,
    scene_id: Option<String>,
    parcel: Option<String>,
    tld: Option<String>,
    network: Option<String>,
    is_guest: Option<bool>,
    realm: SignedFetchMetaRealm,
    signer: String,
}

#[op2(async)]
#[serde]
pub async fn op_signed_fetch_headers(
    op_state: Rc<RefCell<OpState>>,
    #[string] uri: String,
    #[string] method: Option<String>,
) -> Result<Vec<(String, String)>, AnyError> {
    let wallet = op_state
        .borrow()
        .borrow::<Option<EphemeralAuthChain>>()
        .clone();

    if let Some(ephemeral_wallet) = wallet {
        let scene_entity_definition = op_state
            .borrow()
            .borrow::<Arc<SceneEntityDefinition>>()
            .clone();

        let realm_info = op_state.borrow().borrow::<DclSceneRealmData>().clone();

        // get host name from url
        let realm_uri = Uri::try_from(realm_info.base_url)?;
        let hostname = realm_uri
            .host()
            .ok_or(anyhow::Error::msg("Invalid host name"))?;
        let network = "mainnet"; // TODO: this could be taken from `network_id`?

        let meta: SignedFetchMeta = SignedFetchMeta {
            origin: Some("https://decentraland.org".to_owned()),
            scene_id: Some(scene_entity_definition.id.clone()),
            parcel: Some(format!(
                "{},{}",
                scene_entity_definition.get_base_parcel().x,
                scene_entity_definition.get_base_parcel().y
            )),
            tld: Some("org".to_owned()),
            network: Some(network.to_owned()),
            is_guest: Some(false),
            realm: SignedFetchMetaRealm {
                hostname: hostname.to_owned(),
                protocol: "v3".to_owned(),
                server_name: realm_info.realm_name,
            },
            signer: "decentraland-kernel-scene".to_owned(),
        };

        let headers = sign_request(
            method.as_deref().unwrap_or("get"),
            &Uri::try_from(uri)?,
            &ephemeral_wallet,
            meta,
        )
        .await;

        Ok(headers)
    } else {
        Err(anyhow::Error::msg("There is no wallet to sign headers."))
    }
}
