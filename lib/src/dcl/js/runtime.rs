use deno_core::{anyhow::anyhow, error::AnyError, op2, OpDecl, OpState};
use serde::Serialize;

use std::{cell::RefCell, rc::Rc, sync::Arc};

use crate::{
    content::content_mapping::ContentMappingAndUrlRef,
    dcl::{
        scene_apis::{ContentMapping, GetSceneInformationResponse},
        DclSceneRealmData,
    },
    godot_classes::dcl_global_time::DclGlobalTime,
    realm::scene_definition::SceneEntityDefinition,
};

use super::SceneEnv;

pub fn ops() -> Vec<OpDecl> {
    vec![
        op_get_file_url(),
        op_get_realm(),
        op_get_scene_information(),
        op_get_world_time(),
    ]
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GetFileUrlResponse {
    url: String,
    hash: String,
}

#[op2]
#[serde]
fn op_get_file_url(
    op_state: Rc<RefCell<OpState>>,
    #[string] filename: String,
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

    Err(anyhow!("not found {filename}"))
}

#[op2]
#[serde]
fn op_get_realm(op_state: &mut OpState) -> DclSceneRealmData {
    op_state.borrow::<DclSceneRealmData>().clone()
}

#[op2(fast)]
fn op_get_world_time(_op_state: &mut OpState) -> f64 {
    let scene_env = op_state.borrow::<SceneEnv>();
    if scene_env.fixed_skybox_time {
        54000.0 // 3pm
    } else {
        DclGlobalTime::get_world_time()
    }
}

#[op2]
#[serde]
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
