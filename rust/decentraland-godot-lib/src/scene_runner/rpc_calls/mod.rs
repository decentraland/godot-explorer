mod handle_restricted_actions;

use crate::{
    common::rpc::{RpcCall, RpcCalls},
    dcl::crdt::SceneCrdtState,
};

use self::handle_restricted_actions::{change_realm, move_player_to, teleport_to};

use super::scene::Scene;

pub fn process_rpcs(scene: &Scene, crdt_state: &mut SceneCrdtState, rpc_calls: &RpcCalls) {
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
                move_player_to(scene, crdt_state, position_target, camera_target, response);
            }
            RpcCall::TeleportTo {
                world_coordinates,
                response,
            } => {
                teleport_to(scene, world_coordinates, response)
            }
        }
    }
}
