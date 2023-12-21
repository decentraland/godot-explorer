use deno_core::{
    anyhow::{self, anyhow},
    error::AnyError,
    op, Op, OpDecl, OpState,
};
use std::{cell::RefCell, rc::Rc, sync::Arc};

use crate::{
    auth::{ethereum_provider::EthereumProvider, with_browser_and_server::RPCSendableMessage},
    dcl::scene_apis::RpcCall,
};

// list of op declarations
pub fn ops() -> Vec<OpDecl> {
    vec![op_send_async::DECL]
}

#[op]
async fn op_send_async(
    state: Rc<RefCell<OpState>>,
    method: String,
    params: String,
) -> Result<serde_json::Value, AnyError> {
    let params: Vec<serde_json::Value> = serde_json::from_str(&params)?;

    match method.as_str() {
        "eth_sendTransaction" | "eth_signTypedData_v4" => {
            let (sx, rx) = tokio::sync::oneshot::channel::<Result<serde_json::Value, String>>();

            state
                .borrow_mut()
                .borrow_mut::<Vec<RpcCall>>()
                .push(RpcCall::SendAsync {
                    body: RPCSendableMessage {
                        jsonrpc: "2.0".into(),
                        id: 1,
                        method,
                        params,
                    },
                    response: sx.into(),
                });

            rx.await
                .map_err(|e| anyhow::anyhow!(e))?
                .map_err(|e| anyhow!(e))
        }
        _ => {
            let ethereum_provider = { state.borrow().borrow::<Arc<EthereumProvider>>().clone() };

            ethereum_provider
                .send_async(method.as_str(), params.as_slice())
                .await
        }
    }
}
