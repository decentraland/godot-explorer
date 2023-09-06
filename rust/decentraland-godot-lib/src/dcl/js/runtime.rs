use deno_core::{anyhow::anyhow, error::AnyError, op, Op, OpDecl, OpState};

use serde::Serialize;
use std::{
    cell::RefCell,
    rc::Rc,
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
    let mut http_requester = HttpRequester::new();
    let SceneContentMapping(base_url, content_mapping) = state.borrow::<SceneContentMapping>();
    let file = content_mapping.get(&filename);

    tracing::info!("op_read_file: {}", filename);

    if let Some(hash) = file {
        let url = format!("{base_url}{hash}");
        http_requester.send_request(RequestOption::new(
            0,
            url,
            reqwest::Method::GET,
            ResponseType::AsBytes,
            None,
            None,
        ));

        // wait until the request is done or timeout
        let start_time = Instant::now();
        loop {
            if let Some(response) = http_requester.poll() {
                if let Ok(response) = response {
                    if let Ok(ResponseEnum::Bytes(content)) = response.response_data {
                        return Ok(ReadFileResponse {
                            content,
                            hash: hash.clone(),
                        });
                    }
                }
                break;
            } else {
                std::thread::sleep(Duration::from_millis(10));
            }

            if start_time.elapsed() > Duration::from_secs(10) {
                break;
            }
        }
    }

    Err(anyhow!("not found"))
}
