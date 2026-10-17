use deno_core::{anyhow::anyhow, error::AnyError, op2, OpDecl, OpState};
use std::{cell::RefCell, rc::Rc};

use crate::dcl::scene_apis::{PortableLocation, RpcCall, SpawnResponse};

// list of op declarations
pub fn ops() -> Vec<OpDecl> {
    vec![op_portable_spawn(), op_portable_list(), op_portable_kill()]
}

#[op2(async)]
#[serde]
async fn op_portable_spawn(
    state: Rc<RefCell<OpState>>,
    #[string] pid: Option<String>,
    #[string] ens: Option<String>,
) -> Result<SpawnResponse, AnyError> {
    let (sx, rx) = tokio::sync::oneshot::channel::<Result<SpawnResponse, String>>();

    let location = match (pid, ens) {
        (Some(urn), None) => PortableLocation::Urn(urn.clone()),
        (None, Some(ens)) => PortableLocation::Ens(ens.clone()),
        _ => anyhow::bail!("provide exactly one of `pid` and `ens`"),
    };

    state
        .borrow_mut()
        .borrow_mut::<Vec<RpcCall>>()
        .push(RpcCall::SpawnPortable {
            location,
            response: sx.into(),
        });

    rx.await
        .map_err(|e| anyhow::anyhow!(e))?
        .map_err(|e| anyhow!(e))
}

#[op2(async)]
async fn op_portable_kill(
    state: Rc<RefCell<OpState>>,
    #[string] pid: String,
) -> Result<bool, AnyError> {
    let (sx, rx) = tokio::sync::oneshot::channel::<bool>();

    // might not be a urn, who even knows

    state
        .borrow_mut()
        .borrow_mut::<Vec<RpcCall>>()
        .push(RpcCall::KillPortable {
            location: PortableLocation::Urn(pid.clone()),
            response: sx.into(),
        });

    rx.await.map_err(|e| anyhow::anyhow!(e))
}

#[op2(async)]
#[serde]
async fn op_portable_list(state: Rc<RefCell<OpState>>) -> Vec<SpawnResponse> {
    let (sx, rx) = tokio::sync::oneshot::channel::<Vec<SpawnResponse>>();

    state
        .borrow_mut()
        .borrow_mut::<Vec<RpcCall>>()
        .push(RpcCall::ListPortables {
            response: sx.into(),
        });

    rx.await.unwrap_or_default()
}
