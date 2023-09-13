use deno_core::{anyhow::anyhow, error::AnyError, op, Op, OpDecl, OpState};

use serde::Serialize;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

use crate::http_request::{
    http_requester::HttpRequester,
    request_response::{RequestOption, ResponseEnum, ResponseType},
};

use super::SceneContentMapping;

// use crate::interface::crdt_context::CrdtContext;

// list of op declarations
pub fn ops() -> Vec<OpDecl> {
    vec![op_read_file::DECL]
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReadFileResponse {
    content: Vec<u8>,
    hash: String,
}

#[op(v8)]
async fn op_read_file(
    op_state: Rc<RefCell<OpState>>,
    filename: String,
) -> Result<ReadFileResponse, AnyError> {
    let state = op_state.borrow();
    let SceneContentMapping(base_url, content_mapping) = state.borrow::<SceneContentMapping>();
    let file = content_mapping.get(&filename);

    tracing::info!("op_read_file: {}", filename);

    if let Some(hash) = file {
        let hash = hash.clone();
        let url = format!("{base_url}{hash}");
        drop(state);

        let client = reqwest::Client::new();
        tracing::info!("requesting to {}", url);

        let response = HttpRequester::do_request(
            &client,
            RequestOption::new(
                0,
                url,
                reqwest::Method::GET,
                ResponseType::AsBytes,
                None,
                None,
            ),
        )
        .await;

        tracing::info!("ok request");

        match response {
            Ok(response) => {
                if let Ok(ResponseEnum::Bytes(content)) = response.response_data {
                    return Ok(ReadFileResponse { content, hash });
                } else {
                    tracing::info!("wrong response");
                }
            }
            Err(error) => {
                tracing::error!("error polling http_requester {}", error);
            }
        }
    }

    tracing::error!("error polling http_requester unknown");
    Err(anyhow!("not found"))
}
