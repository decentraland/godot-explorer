use std::{cell::RefCell, rc::Rc};

use deno_core::{anyhow::anyhow, error::AnyError, op2, OpDecl, OpState};
use http::Uri;

use crate::dcl::scene_apis::RpcCall;

pub fn ops() -> Vec<OpDecl> {
    vec![
        op_change_realm(),
        op_open_nft_dialog(),
        op_open_external_url(),
        op_move_player_to(),
        op_teleport_to(),
        op_trigger_emote(),
        op_trigger_scene_emote(),
    ]
}

#[op2(async)]
async fn op_change_realm(
    op_state: Rc<RefCell<OpState>>,
    #[string] realm: String,
    #[string] message: Option<String>,
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

#[op2(async)]
async fn op_open_nft_dialog(
    op_state: Rc<RefCell<OpState>>,
    #[string] urn: String,
) -> Result<(), AnyError> {
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

#[op2(async)]
async fn op_open_external_url(
    op_state: Rc<RefCell<OpState>>,
    #[string] url: String,
) -> Result<(), AnyError> {
    let parsed_url = match url.parse::<Uri>() {
        Ok(parsed_url) if parsed_url.scheme_str() == Some("https") => parsed_url,
        Ok(_) => return Err(anyhow!("URL does not use HTTPS")),
        Err(_) => return Err(anyhow!("Invalid URL")),
    };

    let (sx, rx) = tokio::sync::oneshot::channel::<Result<(), String>>();

    op_state
        .borrow_mut()
        .borrow_mut::<Vec<RpcCall>>()
        .push(RpcCall::OpenExternalUrl {
            url: parsed_url,
            response: sx.into(),
        });

    rx.await
        .map_err(|e| anyhow::anyhow!(e))?
        .map_err(|e| anyhow!(e))
}

#[op2(async)]
#[allow(clippy::too_many_arguments)]
async fn op_move_player_to(
    op_state: Rc<RefCell<OpState>>,
    position_x: f32,
    position_y: f32,
    position_z: f32,
    camera: bool,
    maybe_camera_x: f32,
    maybe_camera_y: f32,
    maybe_camera_z: f32,
) -> Result<(), AnyError> {
    let (sx, rx) = tokio::sync::oneshot::channel::<Result<(), String>>();

    let position_target = [position_x, position_y, position_z];
    let camera_target = if camera {
        Some([maybe_camera_x, maybe_camera_y, maybe_camera_z])
    } else {
        None
    };

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

#[op2(async)]
async fn op_teleport_to(
    op_state: Rc<RefCell<OpState>>,
    world_coordinates_x: i32,
    world_coordinates_y: i32,
) -> Result<(), AnyError> {
    let (sx, rx) = tokio::sync::oneshot::channel::<Result<(), String>>();

    op_state
        .borrow_mut()
        .borrow_mut::<Vec<RpcCall>>()
        .push(RpcCall::TeleportTo {
            world_coordinates: [world_coordinates_x, world_coordinates_y],
            response: sx.into(),
        });

    rx.await
        .map_err(|e| anyhow::anyhow!(e))?
        .map_err(|e| anyhow!(e))
}

#[op2(async)]
async fn op_trigger_emote(
    op_state: Rc<RefCell<OpState>>,
    #[string] emote_id: String,
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

#[op2(async)]
async fn op_trigger_scene_emote(
    op_state: Rc<RefCell<OpState>>,
    #[string] emote_src: String,
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
