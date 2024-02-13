use deno_core::{anyhow::anyhow, error::AnyError, op, Op, OpDecl, OpState};
use serde::Serialize;

use std::{cell::RefCell, rc::Rc, sync::Arc};

use crate::{
    content::content_mapping::ContentMappingAndUrlRef,
    dcl::scene_apis::{ContentMapping, GetRealmResponse, GetSceneInformationResponse, RpcCall},
    realm::scene_definition::SceneEntityDefinition,
};

pub fn ops() -> Vec<OpDecl> {
    vec![
        op_get_file_url::DECL,
        op_get_realm::DECL,
        op_get_scene_information::DECL,
    ]
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GetFileUrlResponse {
    url: String,
    hash: String,
}

#[op(v8)]
fn op_get_file_url(
    op_state: Rc<RefCell<OpState>>,
    filename: String,
) -> Result<GetFileUrlResponse, AnyError> {
    let state = op_state.borrow();
    let content_mapping = state.borrow::<ContentMappingAndUrlRef>();
    let filename = filename.to_lowercase();
    let base_url = content_mapping.base_url.as_str();
    let file = content_mapping.get_hash(filename.as_str());

    if let Some(hash) = file {
        let url = format!("{base_url}{hash}");
        return Ok(GetFileUrlResponse {
            url,
            hash: hash.to_string(),
        });
    }

    Err(anyhow!("not found"))
}

#[op]
async fn op_get_realm(op_state: Rc<RefCell<OpState>>) -> Result<GetRealmResponse, AnyError> {
    let (sx, rx) = tokio::sync::oneshot::channel::<GetRealmResponse>();

    op_state
        .borrow_mut()
        .borrow_mut::<Vec<RpcCall>>()
        .push(RpcCall::GetRealm {
            response: sx.into(),
        });

    rx.await.map_err(|e| anyhow!(e))
}

#[op]
fn op_get_scene_information(op_state: &mut OpState) -> GetSceneInformationResponse {
    let scene_entity_definition = op_state.borrow::<Arc<SceneEntityDefinition>>().clone();

    let content: Vec<ContentMapping> = scene_entity_definition
        .content_mapping
        .files()
        .iter()
        .map(|(file, hash)| ContentMapping {
            file: file.clone(),
            hash: hash.clone(),
        })
        .collect();

    let metadata_json =
        serde_json::ser::to_string(&scene_entity_definition.scene_meta_scene).unwrap();

    GetSceneInformationResponse {
        urn: scene_entity_definition.id.clone(),
        content,
        metadata_json,
        base_url: scene_entity_definition.content_mapping.base_url.clone(),
    }
}
