mod handle_restricted_actions;
mod portables;

use godot::{meta::ToGodot, obj::NewGd};

use crate::{
    content::content_mapping::DclContentMappingAndUrl,
    dcl::{scene_apis::RpcCall, SceneId},
    godot_classes::{
        dcl_global::DclGlobal,
        rpc_sender::take_and_compare_snapshot_response::DclRpcSenderGetTextureSize,
    },
};

use self::{
    handle_restricted_actions::{
        change_realm, move_player_to, open_external_url, open_nft_dialog, teleport_to,
        trigger_emote, trigger_scene_emote,
    },
    portables::{kill_portable, list_portables, spawn_portable},
};

use super::scene::Scene;

pub fn process_rpcs(scene: &mut Scene, current_parcel_scene_id: &SceneId, rpc_calls: Vec<RpcCall>) {
    for rpc_call in rpc_calls {
        match rpc_call {
            // Restricted Actions
            RpcCall::ChangeRealm {
                to,
                message,
                response,
            } => {
                change_realm(scene, current_parcel_scene_id, &to, &message, &response);
            }
            RpcCall::OpenNftDialog { urn, response } => {
                open_nft_dialog(scene, current_parcel_scene_id, &urn, &response);
            }
            RpcCall::OpenExternalUrl { url, response } => {
                open_external_url(scene, current_parcel_scene_id, &url, &response);
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
            // Portable Experiences
            RpcCall::SpawnPortable { location, response } => {
                spawn_portable(scene, location, response)
            }
            RpcCall::KillPortable { location, response } => kill_portable(location, response),
            RpcCall::ListPortables { response } => list_portables(response),
            RpcCall::SceneTestPlan { body } => {
                tracing::info!("SceneTestPlan: {:?}", body);
                scene.scene_tests = body.tests.iter().map(|v| (v.name.clone(), None)).collect();
                scene.scene_test_plan_received = true;
            }
            RpcCall::SceneTestResult { body } => {
                tracing::info!("SceneTestResult: {:?}", body);
                if let Some(test_entry) = scene.scene_tests.get_mut(&body.name) {
                    *test_entry = Some(body);
                }
            }
            RpcCall::SendAsync { body, response } => {
                DclGlobal::singleton()
                    .bind()
                    .get_player_identity()
                    .bind()
                    .send_async(body, response);
            }
            RpcCall::SendCommsMessage { body } => {
                let scene_id = scene.scene_entity_definition.id.clone();
                let mut comms = DclGlobal::singleton().bind().get_comms();
                let mut communication_manager = comms.bind_mut();
                for data in body {
                    communication_manager.send_scene_message(scene_id.clone(), data);
                }
            }
            RpcCall::GetTextureSize { src, response } => {
                let mut rpc_sender = DclRpcSenderGetTextureSize::new_gd();
                rpc_sender.bind_mut().set_sender(response);

                let content_mapping =
                    DclContentMappingAndUrl::from_ref(scene.content_mapping.clone());

                DclGlobal::singleton().call_deferred(
                    "async_get_texture_size",
                    &[
                        content_mapping.to_variant(),
                        src.to_variant(),
                        rpc_sender.to_variant(),
                    ],
                );
            }
        }
    }
}
