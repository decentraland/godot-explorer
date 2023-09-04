use deno_core::{anyhow::anyhow, error::AnyError, op, Op, OpDecl, OpState};

use serde::Serialize;
use std::{cell::RefCell, rc::Rc};

use super::SceneContentMapping;

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
    println!("Start");
    let (base_url, hash) = {
        let state = op_state.borrow();
        let SceneContentMapping(base_url, content_mapping) = state.borrow::<SceneContentMapping>();
        let file = content_mapping.get(&filename);
        let hash = match file {
            Some(e) => e,
            None => return Err(anyhow!("not found"))
        };
        (base_url.clone(), hash.clone())
    };

    let url = format!("{base_url}{hash}");

    println!("url {}", url);

    let response = reqwest::get(url).await.map_err(|e| anyhow!(e))?;;
    println!("Response");
    match response.status() {
        reqwest::StatusCode::OK => {
            let content = response.bytes().await.map_err(|e| anyhow!(e))?;;
            let content = content.to_vec();
            println!("Done...");
            return Ok(ReadFileResponse {
                content,
                hash,
            });
        }
        _ => {
            println!("Not found...");
            return Err(anyhow!("not found"));
        }
    };

}
