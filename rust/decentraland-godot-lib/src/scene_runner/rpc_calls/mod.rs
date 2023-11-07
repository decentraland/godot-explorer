mod handle_restricted_actions;
mod portables;

use crate::{
    common::rpc::{RpcCall, RpcCalls},
    dcl::SceneId,
};

use self::{
    handle_restricted_actions::{
        change_realm, move_player_to, teleport_to, trigger_emote, trigger_scene_emote,
    },
    portables::{kill_portable, list_portables, spawn_portable},
};

use super::scene::Scene;

pub fn process_rpcs(scene: &Scene, current_parcel_scene_id: &SceneId, rpc_calls: RpcCalls) {
    for rpc_call in rpc_calls {
        match rpc_call {
            RpcCall::ChangeRealm {
                to,
                message,
                response,
            } => {
                change_realm(scene, &to, &message, &response);
            }
            RpcCall::MovePlayerTo {
                position_target,
                camera_target,
                response,
            } => {
                move_player_to(
                    scene,
                    current_parcel_scene_id,
                    &position_target,
                    &camera_target,
                    &response,
                );
            }
            RpcCall::TeleportTo {
                world_coordinates,
                response,
            } => teleport_to(
                scene,
                current_parcel_scene_id,
                &world_coordinates,
                &response,
            ),
            RpcCall::TriggerEmote { emote_id, response } => {
                trigger_emote(scene, current_parcel_scene_id, &emote_id, &response)
            }
            RpcCall::TriggerSceneEmote {
                emote_src,
                looping,
                response,
            } => trigger_scene_emote(
                scene,
                current_parcel_scene_id,
                &emote_src,
                &looping,
                &response,
            ),
            RpcCall::SpawnPortable { location, response } => {
                spawn_portable(scene, location, response)
            }
            RpcCall::KillPortable { location, response } => kill_portable(location, response),
            RpcCall::ListPortables { response } => list_portables(response),
        }
    }
}
