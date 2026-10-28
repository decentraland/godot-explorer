mod handle_restricted_actions;
mod portables;

use godot::{meta::ToGodot, obj::NewGd, prelude::*};

use crate::{
    content::{content_mapping::DclContentMappingAndUrl, texture::TextureEntry},
    dcl::{
        scene_apis::{NetworkMessageRecipient, RpcCall},
        SceneId,
    },
    engine::local_task::{await_promise, spawn_local},
    fetcher::globals,
    godot_classes::{
        dcl_global::DclGlobal,
        rpc_sender::take_and_compare_snapshot_response::DclRpcSenderGetTextureSize,
    },
};

use self::{
    handle_restricted_actions::{
        change_realm, move_player_to, open_external_url, open_nft_dialog, stop_emote, teleport_to,
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
                avatar_target,
            } => {
                move_player_to(
                    scene,
                    current_parcel_scene_id,
                    &position_target,
                    &camera_target,
                    &avatar_target,
                );
            }
            RpcCall::TeleportTo {
                world_coordinates,
                realm,
                response,
            } => teleport_to(
                scene,
                current_parcel_scene_id,
                &world_coordinates,
                &realm,
                &response,
            ),
            RpcCall::TriggerEmote { emote_id, mask } => {
                trigger_emote(scene, current_parcel_scene_id, &emote_id, mask)
            }
            RpcCall::TriggerSceneEmote {
                emote_src,
                looping,
                mask,
            } => trigger_scene_emote(scene, current_parcel_scene_id, &emote_src, &looping, mask),
            RpcCall::StopEmote => stop_emote(scene, current_parcel_scene_id),
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
            RpcCall::SendCommsMessage { body, recipient } => {
                let scene_id = scene.scene_entity_definition.id.clone();

                // Override recipient to AuthServer if scene has authoritative multiplayer enabled
                let recipient = if scene
                    .scene_entity_definition
                    .scene_meta_scene
                    .authoritative_multiplayer
                    .unwrap_or(false)
                {
                    NetworkMessageRecipient::AuthServer
                } else {
                    recipient
                };

                let mut comms = DclGlobal::singleton().bind().get_comms();
                let mut communication_manager = comms.bind_mut();
                communication_manager.send_scene_message(scene_id, body, recipient);
            }
            RpcCall::SceneLocaleRequested { locale } => {
                if let Some(global) = DclGlobal::try_singleton() {
                    let mut metrics = global.bind().metrics.clone();
                    metrics.bind_mut().track_scene_locale_requested(
                        scene.scene_entity_definition.id.clone(),
                        locale,
                    );
                }
            }
            RpcCall::GetTextureSize { src, response } => {
                let mut rpc_sender = DclRpcSenderGetTextureSize::new_gd();
                rpc_sender.bind_mut().set_sender(response);

                let content_mapping =
                    DclContentMappingAndUrl::from_ref(scene.content_mapping.clone());

                spawn_local(texture_size(content_mapping, src, rpc_sender));
            }
        }
    }
}

/// The SDK6 adaptation layer asks for a texture's size; a texture that cannot be fetched answers 2048².
async fn texture_size(
    content_mapping: Gd<DclContentMappingAndUrl>,
    src: String,
    mut sender: Gd<DclRpcSenderGetTextureSize>,
) {
    let mut hash = content_mapping.bind().get_hash(src.as_str().into());
    if hash.is_empty() {
        hash = src.as_str().into();
    }
    let promise = globals::content_provider()
        .bind_mut()
        .fetch_texture_by_hash(hash, content_mapping);
    let result = await_promise(promise).await;
    let size = match result.try_to::<Gd<TextureEntry>>() {
        Ok(entry) => entry.bind().original_size,
        Err(_) => {
            tracing::warn!("couldn't get the size of {src}");
            Vector2i::new(2048, 2048)
        }
    };
    sender
        .bind_mut()
        .send(Vector2::new(size.x as f32, size.y as f32).to_variant());
}
