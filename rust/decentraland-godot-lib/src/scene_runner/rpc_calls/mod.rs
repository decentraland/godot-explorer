mod handle_restricted_actions;

use crate::{
    common::rpc::{RpcCall, RpcCalls},
    dcl::SceneId,
};

use self::handle_restricted_actions::{change_realm, move_player_to, teleport_to};

use super::scene::Scene;

pub fn process_rpcs(scene: &Scene, current_parcel_scene_id: &SceneId, rpc_calls: &RpcCalls) {
    for rpc_call in rpc_calls {
        match rpc_call {
            RpcCall::ChangeRealm {
                to,
                message,
                response,
            } => {
                change_realm(scene, to, message, response);
            }
            RpcCall::MovePlayerTo {
                position_target,
                camera_target,
                response,
            } => {
                move_player_to(
                    scene,
                    current_parcel_scene_id,
                    position_target,
                    camera_target,
                    response,
                );
            }
            RpcCall::TeleportTo {
                world_coordinates,
                response,
            } => teleport_to(scene, current_parcel_scene_id, world_coordinates, response),
        }
    }
}
