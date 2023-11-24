use deno_core::{anyhow::anyhow, error::AnyError, op, Op, OpDecl, OpState};
use serde::Serialize;

use std::{cell::RefCell, rc::Rc};

use crate::dcl::scene_apis::{GetRealmResponse, RpcCall};

use super::SceneContentMapping;

pub fn ops() -> Vec<OpDecl> {
    vec![op_get_file_url::DECL, op_get_realm::DECL]
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
    let SceneContentMapping(base_url, content_mapping) = state.borrow::<SceneContentMapping>();
    let file = content_mapping.get(&filename);

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
