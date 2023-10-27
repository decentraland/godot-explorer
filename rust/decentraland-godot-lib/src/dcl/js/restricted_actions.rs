use std::{cell::RefCell, rc::Rc};

use deno_core::{
    anyhow::{self, anyhow},
    error::AnyError,
    op, Op, OpDecl, OpState,
};

use crate::common::rpc::{RpcCall, RpcCalls};

pub fn ops() -> Vec<OpDecl> {
    vec![
        op_change_realm::DECL,
        op_move_player_to::DECL,
        op_teleport_to::DECL,
    ]
}

#[op]
async fn op_change_realm(
    op_state: Rc<RefCell<OpState>>,
    realm: String,
    message: Option<String>,
) -> Result<(), AnyError> {
    let (sx, rx) = tokio::sync::oneshot::channel::<Result<(), String>>();

    op_state
        .borrow_mut()
        .borrow_mut::<RpcCalls>()
        .push(RpcCall::ChangeRealm {
            to: realm,
            message,
            response: sx.into(),
        });

    rx.await
        .map_err(|e| anyhow::anyhow!(e))?
        .map_err(|e| anyhow!(e))
}

#[op]
async fn op_move_player_to(
    op_state: Rc<RefCell<OpState>>,
    position_target: [f32; 3],
    camera_target: Option<[f32; 3]>,
) -> Result<(), AnyError> {
    let (sx, rx) = tokio::sync::oneshot::channel::<Result<(), String>>();

    op_state
        .borrow_mut()
        .borrow_mut::<RpcCalls>()
        .push(RpcCall::MovePlayerTo {
            position_target,
            camera_target,
            response: sx.into(),
        });

    rx.await
        .map_err(|e| anyhow::anyhow!(e))?
        .map_err(|e| anyhow!(e))
}

#[op]
async fn op_teleport_to(
    op_state: Rc<RefCell<OpState>>,
    world_coordinates: [i32; 2],
) -> Result<(), AnyError> {
    let (sx, rx) = tokio::sync::oneshot::channel::<Result<(), String>>();

    op_state
        .borrow_mut()
        .borrow_mut::<RpcCalls>()
        .push(RpcCall::TeleportTo {
            world_coordinates,
            response: sx.into(),
        });

    let res = rx.await
        .map_err(|e| anyhow::anyhow!(e))?
        .map_err(|e| anyhow!(e));

    res
}
