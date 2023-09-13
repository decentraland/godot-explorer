use std::{cell::RefCell, collections::HashMap, rc::Rc};

use deno_core::{error::AnyError, op, ByteString, Op, OpDecl, OpState};

pub fn ops() -> Vec<OpDecl> {
    vec![op_fetch_custom::DECL]
}

#[op]
pub async fn op_fetch_custom(
    state: Rc<RefCell<OpState>>,
    method: ByteString,
    url: String,
    headers: HashMap<String, String>,
    has_body: bool,
    data: String,
    redirect: ByteString,
    timeout: u32,
) -> Result<(), AnyError> {
    return Err(anyhow::Error::msg("not implemented"));
}
