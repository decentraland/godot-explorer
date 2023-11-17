use std::{cell::RefCell, rc::Rc};

use deno_core::{
    anyhow::{self, anyhow},
    error::AnyError,
    op, Op, OpDecl, OpState,
};

use crate::dcl::scene_apis::RpcCall;

pub fn ops() -> Vec<OpDecl> {
    vec![
        op_change_realm::DECL,
        op_open_nft_dialog::DECL,
        op_move_player_to::DECL,
        op_teleport_to::DECL,
        op_trigger_emote::DECL,
        op_trigger_scene_emote::DECL,
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
        .borrow_mut::<Vec<RpcCall>>()
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
async fn op_open_nft_dialog(op_state: Rc<RefCell<OpState>>, urn: String) -> Result<(), AnyError> {
    let (sx, rx) = tokio::sync::oneshot::channel::<Result<(), String>>();

    op_state
        .borrow_mut()
        .borrow_mut::<Vec<RpcCall>>()
        .push(RpcCall::OpenNftDialog {
            urn,
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
        .borrow_mut::<Vec<RpcCall>>()
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
        .borrow_mut::<Vec<RpcCall>>()
        .push(RpcCall::TeleportTo {
            world_coordinates,
            response: sx.into(),
        });

    rx.await
        .map_err(|e| anyhow::anyhow!(e))?
        .map_err(|e| anyhow!(e))
}

#[op]
async fn op_trigger_emote(
    op_state: Rc<RefCell<OpState>>,
    emote_id: String,
) -> Result<(), AnyError> {
    let (sx, rx) = tokio::sync::oneshot::channel::<Result<(), String>>();

    op_state
        .borrow_mut()
        .borrow_mut::<Vec<RpcCall>>()
        .push(RpcCall::TriggerEmote {
            emote_id,
            response: sx.into(),
        });

    rx.await
        .map_err(|e| anyhow::anyhow!(e))?
        .map_err(|e| anyhow!(e))
}

#[op]
async fn op_trigger_scene_emote(
    op_state: Rc<RefCell<OpState>>,
    emote_src: String,
    looping: bool,
) -> Result<(), AnyError> {
    let (sx, rx) = tokio::sync::oneshot::channel::<Result<(), String>>();

    op_state
        .borrow_mut()
        .borrow_mut::<Vec<RpcCall>>()
        .push(RpcCall::TriggerSceneEmote {
            emote_src,
            looping,
            response: sx.into(),
        });

    rx.await
        .map_err(|e| anyhow::anyhow!(e))?
        .map_err(|e| anyhow!(e))
}
