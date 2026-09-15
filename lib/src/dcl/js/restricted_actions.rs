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

#[op2(fast)]
#[allow(clippy::too_many_arguments)]
fn op_move_player_to(
    op_state: Rc<RefCell<OpState>>,
    position_x: f32,
    position_y: f32,
    position_z: f32,
    camera_x: f32,
    camera_y: f32,
    camera_z: f32,
    avatar_x: f32,
    avatar_y: f32,
    avatar_z: f32,
) {
    let position_target = [position_x, position_y, position_z];
    let camera_target = if camera_x.is_nan() || camera_y.is_nan() || camera_z.is_nan() {
        None
    } else {
        Some([camera_x, camera_y, camera_z])
    };
    let avatar_target = if avatar_x.is_nan() || avatar_y.is_nan() || avatar_z.is_nan() {
        None
    } else {
        Some([avatar_x, avatar_y, avatar_z])
    };

    op_state
        .borrow_mut()
        .borrow_mut::<Vec<RpcCall>>()
        .push(RpcCall::MovePlayerTo {
            position_target,
            camera_target,
            avatar_target,
        });
}

type TeleportArgs = (Option<[i32; 2]>, Option<String>);

/// Both fields of TeleportToRequest are optional (protocol#477) but #[op2] params can't be
/// Option, so they cross the boundary as sentinels: `has_coordinates` false means no parcel,
/// an empty `realm` means the player's current one. A request carrying neither is meaningless
/// and is rejected here, before any UI is touched.
fn decode_teleport_args(
    world_coordinates_x: i32,
    world_coordinates_y: i32,
    has_coordinates: bool,
    realm: String,
) -> Result<TeleportArgs, &'static str> {
    let world_coordinates = has_coordinates.then_some([world_coordinates_x, world_coordinates_y]);
    let realm = (!realm.is_empty()).then_some(realm);

    if world_coordinates.is_none() && realm.is_none() {
        return Err("teleportTo requires worldCoordinates, a realm, or both");
    }

    Ok((world_coordinates, realm))
}

#[op2(async)]
async fn op_teleport_to(
    op_state: Rc<RefCell<OpState>>,
    world_coordinates_x: i32,
    world_coordinates_y: i32,
    has_coordinates: bool,
    #[string] realm: String,
) -> Result<(), AnyError> {
    let (world_coordinates, realm) = decode_teleport_args(
        world_coordinates_x,
        world_coordinates_y,
        has_coordinates,
        realm,
    )
    .map_err(|e| anyhow!(e))?;

    let (sx, rx) = tokio::sync::oneshot::channel::<Result<(), String>>();

    op_state
        .borrow_mut()
        .borrow_mut::<Vec<RpcCall>>()
        .push(RpcCall::TeleportTo {
            world_coordinates,
            realm,
            response: sx.into(),
        });

    rx.await
        .map_err(|e| anyhow::anyhow!(e))?
        .map_err(|e| anyhow!(e))
}

// `mask` uses the internal convention: -1 = full body (absent), 0 = AM_UPPER_BODY.
// A sentinel instead of Option because #[op2(fast)] can't take optional params
// (same idiom as movePlayerTo's NaN sentinels in RestrictedActions.js).
#[op2(fast)]
fn op_trigger_emote(op_state: Rc<RefCell<OpState>>, #[string] emote_id: String, #[smi] mask: i32) {
    op_state
        .borrow_mut()
        .borrow_mut::<Vec<RpcCall>>()
        .push(RpcCall::TriggerEmote {
            emote_id,
            mask: mask as i64,
        });
}

#[op2(fast)]
fn op_trigger_scene_emote(
    op_state: Rc<RefCell<OpState>>,
    #[string] emote_src: String,
    looping: bool,
    #[smi] mask: i32,
) {
    op_state
        .borrow_mut()
        .borrow_mut::<Vec<RpcCall>>()
        .push(RpcCall::TriggerSceneEmote {
            emote_src,
            looping,
            mask: mask as i64,
        });
}

#[cfg(test)]
mod tests {
    use super::decode_teleport_args;

    #[test]
    fn coordinates_without_a_realm_stay_in_the_current_realm() {
        let (coordinates, realm) = decode_teleport_args(5, -7, true, String::new()).unwrap();
        assert_eq!(coordinates, Some([5, -7]));
        assert_eq!(realm, None);
    }

    #[test]
    fn coordinates_with_a_realm_carry_both() {
        let (coordinates, realm) =
            decode_teleport_args(5, 5, true, "spacerunner.dcl.eth".to_owned()).unwrap();
        assert_eq!(coordinates, Some([5, 5]));
        assert_eq!(realm.as_deref(), Some("spacerunner.dcl.eth"));
    }

    // No parcel means the realm's own spawn point: the x/y sentinels are ignored, not used as 0,0.
    #[test]
    fn a_realm_without_coordinates_drops_the_coordinate_sentinels() {
        let (coordinates, realm) =
            decode_teleport_args(9, 9, false, "spacerunner.dcl.eth".to_owned()).unwrap();
        assert_eq!(coordinates, None);
        assert_eq!(realm.as_deref(), Some("spacerunner.dcl.eth"));
    }

    #[test]
    fn neither_field_is_rejected() {
        assert!(decode_teleport_args(0, 0, false, String::new()).is_err());
    }
}
