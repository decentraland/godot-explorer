use std::{cell::RefCell, rc::Rc};

use deno_core::{OpState, op, OpDecl, Op};

use crate::common::rpc::{RpcCalls, RpcCall};

pub fn ops() -> Vec<OpDecl> {
    vec![op_change_realm::DECL]
}

#[op]
async fn op_change_realm(
    op_state: Rc<RefCell<OpState>>,
    realm: String,
    message: Option<String>,
) -> bool {
    let (sx, rx) = tokio::sync::oneshot::channel::<Result<(), String>>();

    op_state
        .borrow_mut()
        .borrow_mut::<RpcCalls>()
        .push(RpcCall::ChangeRealm {
            to: realm,
            message,
            response: sx.into(),
        });

    matches!(rx.await, Ok(Ok(_)))
}
