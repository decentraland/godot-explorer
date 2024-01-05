use deno_core::{anyhow::anyhow, error::AnyError, op, Op, OpDecl, OpState};
use serde::Serialize;

use std::{cell::RefCell, rc::Rc};

use crate::{
    content::content_mapping::ContentMappingAndUrlRef,
    dcl::scene_apis::{GetRealmResponse, GetSceneInformationResponse, RpcCall},
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
    let file = content_mapping.content.get(&filename);

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
async fn op_get_scene_information(
    op_state: Rc<RefCell<OpState>>,
) -> Result<GetSceneInformationResponse, AnyError> {
    let (sx, rx) = tokio::sync::oneshot::channel::<GetSceneInformationResponse>();

    op_state
        .borrow_mut()
        .borrow_mut::<Vec<RpcCall>>()
        .push(RpcCall::GetSceneInformation {
            response: sx.into(),
        });

    rx.await.map_err(|e| anyhow!(e))
}
