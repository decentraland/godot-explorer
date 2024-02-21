use std::time::Instant;

use godot::prelude::{Callable, GString, ToGodot, Transform3D, VariantArray};

#[cfg(feature = "use_ffmpeg")]
use super::components::{audio_stream::update_audio_stream, video_player::update_video_player};

use super::{
    components::{
        animator::update_animator,
        audio_source::update_audio_source,
        avatar_attach::update_avatar_attach,
        avatar_data::update_avatar_scene_updates,
        avatar_modifier_area::update_avatar_modifier_area,
        avatar_shape::update_avatar_shape,
        billboard::update_billboard,
        camera_mode_area::update_camera_mode_area,
        gltf_container::{sync_gltf_loading_state, update_gltf_container},
        material::update_material,
        mesh_collider::update_mesh_collider,
        mesh_renderer::update_mesh_renderer,
        nft_shape::update_nft_shape,
        pointer_events::update_scene_pointer_events,
        raycast::update_raycasts,
        text_shape::update_text_shape,
        transform_and_parent::update_transform_and_parent,
        tween::update_tween,
        ui::scene_ui::update_scene_ui,
        visibility::update_visibility,
    },
    deleted_entities::update_deleted_entities,
    rpc_calls::process_rpcs,
    scene::{Dirty, Scene, SceneType, SceneUpdateState},
};
use crate::{
    dcl::{
        components::{
            proto_components::sdk::components::{
                PbCameraMode, PbEngineInfo, PbPointerLock, PbUiCanvasInformation,
            },
            transform_and_parent::DclTransformAndParent,
            SceneEntityId,
        },
        crdt::{
            grow_only_set::GenericGrowOnlySetComponentOperation,
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtStateProtoComponents,
        },
        RendererResponse, SceneId,
    },
    godot_classes::dcl_global::DclGlobal,
    scene_runner::components::avatar_shape::update_avatar_shape_emote_command,
};

// @returns true if the scene was full processed, or false if it remains something to process
#[allow(clippy::too_many_arguments)]
pub fn _process_scene(
    scene: &mut Scene,
    end_time_us: i64,
    frames_count: u64,
    camera_global_transform: &Transform3D,
    player_global_transform: &Transform3D,
    camera_mode: i32,
    console: Callable,
    current_parcel_scene_id: &SceneId,
    ref_time: &Instant,
    ui_canvas_information: &PbUiCanvasInformation,
) -> bool {
    let crdt = scene.dcl_scene.scene_crdt.clone();
    let Ok(mut crdt_state) = crdt.try_lock() else {
        return false;
    };
    let crdt_state = &mut crdt_state;
    let mut current_time_us;

    loop {
        let before_compute_update = std::time::Instant::now();

        let should_break = match scene.current_dirty.update_state {
            SceneUpdateState::None => {
                let engine_info_component =
                    SceneCrdtStateProtoComponents::get_engine_info_mut(crdt_state);
                let tick_number =
                    if let Some(entry) = engine_info_component.get(&SceneEntityId::ROOT) {
                        if let Some(value) = entry.value.as_ref() {
                            value.tick_number + 1
                        } else {
                            0
                        }
                    } else {
                        0
                    };

                scene.tick_number = tick_number;

                // fix: if the scene is loading, we need to wait until it finishes before spawn the next tick
                // tick 0 => onStart() => tick=1 => first onUpdate() => tick=2 => second onUpdate() => tick= 3
                if tick_number <= 3 && !scene.gltf_loading.is_empty() {
                    sync_gltf_loading_state(scene, crdt_state, ref_time, end_time_us);

                    let mut scene_node = scene.godot_dcl_scene.root_node_3d.bind_mut();
                    scene_node.gltf_loading_count = scene.gltf_loading.len() as i32;
                    scene_node.max_gltf_loaded_count = scene_node
                        .gltf_loading_count
                        .max(scene_node.max_gltf_loaded_count);
                    return false;
                }

                scene
                    .godot_dcl_scene
                    .root_node_3d
                    .bind_mut()
                    .last_tick_number = tick_number as i32;

                engine_info_component.put(
                    SceneEntityId::ROOT,
                    Some(PbEngineInfo {
                        tick_number,
                        frame_number: frames_count as u32,
                        total_runtime: (Instant::now() - scene.start_time).as_secs_f32(),
                    }),
                );

                if tick_number == 0 {
                    let filter_by_scene_id = if let SceneType::Parcel = scene.scene_type {
                        Some(*current_parcel_scene_id)
                    } else {
                        None
                    };

                    let primary_player_inside = if let SceneType::Parcel = scene.scene_type {
                        *current_parcel_scene_id == scene.scene_id
                    } else {
                        true
                    };

                    DclGlobal::singleton()
                        .bind()
                        .avatars
                        .bind()
                        .first_sync_crdt_state(
                            crdt_state,
                            filter_by_scene_id,
                            primary_player_inside,
                        );
                }

                false
            }
            SceneUpdateState::PrintLogs => {
                // enable logs
                for log in &scene.current_dirty.logs {
                    let mut arguments = VariantArray::new();
                    arguments.push((scene.scene_id.0).to_variant());
                    arguments.push((log.level as i32).to_variant());
                    arguments.push((log.timestamp as f32).to_variant());
                    arguments.push(GString::from(&log.message).to_variant());
                    console.callv(arguments);
                }
                false
            }
            SceneUpdateState::DeletedEntities => {
                update_deleted_entities(scene);
                false
            }
            SceneUpdateState::Tween => {
                update_tween(scene, crdt_state);
                false
            }
            SceneUpdateState::TransformAndParent => {
                !update_transform_and_parent(scene, crdt_state, ref_time, end_time_us)
            }
            SceneUpdateState::VisibilityComponent => {
                update_visibility(scene, crdt_state);
                false
            }
            SceneUpdateState::MeshRenderer => {
                !update_mesh_renderer(scene, crdt_state, ref_time, end_time_us)
            }
            SceneUpdateState::ScenePointerEvents => {
                update_scene_pointer_events(scene, crdt_state);
                false
            }
            SceneUpdateState::Material => {
                update_material(scene, crdt_state);
                false
            }
            SceneUpdateState::TextShape => {
                update_text_shape(scene, crdt_state);
                false
            }
            SceneUpdateState::Billboard => {
                update_billboard(scene, crdt_state, camera_global_transform);
                false
            }
            SceneUpdateState::MeshCollider => {
                update_mesh_collider(scene, crdt_state);
                false
            }
            SceneUpdateState::GltfContainer => {
                !update_gltf_container(scene, crdt_state, ref_time, end_time_us)
            }
            SceneUpdateState::SyncGltfContainer => {
                !sync_gltf_loading_state(scene, crdt_state, ref_time, end_time_us)
            }
            SceneUpdateState::NftShape => {
                update_nft_shape(scene, crdt_state);
                false
            }
            SceneUpdateState::Animator => {
                update_animator(scene, crdt_state);
                false
            }
            SceneUpdateState::AvatarShape => {
                update_avatar_shape(scene, crdt_state);
                false
            }
            SceneUpdateState::AvatarShapeEmoteCommand => {
                update_avatar_shape_emote_command(scene, crdt_state);
                false
            }
            SceneUpdateState::Raycasts => {
                update_raycasts(scene, crdt_state);
                false
            }
            SceneUpdateState::AvatarAttach => {
                update_avatar_attach(scene, crdt_state);
                false
            }
            #[cfg(feature = "use_ffmpeg")]
            SceneUpdateState::VideoPlayer => {
                update_video_player(scene, crdt_state, current_parcel_scene_id);
                false
            }
            #[cfg(feature = "use_ffmpeg")]
            SceneUpdateState::AudioStream => {
                update_audio_stream(scene, crdt_state, current_parcel_scene_id);
                false
            }
            SceneUpdateState::AvatarModifierArea => {
                update_avatar_modifier_area(scene, crdt_state);
                false
            }
            SceneUpdateState::CameraModeArea => {
                update_camera_mode_area(scene, crdt_state);
                false
            }
            SceneUpdateState::AudioSource => {
                update_audio_source(scene, crdt_state, current_parcel_scene_id);
                false
            }
            SceneUpdateState::SceneUi => {
                update_scene_ui(
                    scene,
                    crdt_state,
                    ui_canvas_information,
                    current_parcel_scene_id,
                );
                false
            }
            SceneUpdateState::ComputeCrdtState => {
                update_avatar_scene_updates(scene, crdt_state);

                if scene.godot_dcl_scene.hierarchy_changed_3d {
                    scene
                        .godot_dcl_scene
                        .root_node_3d
                        .call_deferred("emit_signal".into(), &["tree_changed".to_variant()]);
                    scene.godot_dcl_scene.hierarchy_changed_3d = false;
                }

                // Set transform
                let camera_transform = DclTransformAndParent::from_godot(
                    camera_global_transform,
                    scene.godot_dcl_scene.root_node_3d.get_position(),
                );
                let player_transform = DclTransformAndParent::from_godot(
                    player_global_transform,
                    scene.godot_dcl_scene.root_node_3d.get_position() - godot::builtin::Vector3::new(0.0, 0.88, 0.0),
                );
                crdt_state
                    .get_transform_mut()
                    .put(SceneEntityId::PLAYER, Some(player_transform));
                crdt_state
                    .get_transform_mut()
                    .put(SceneEntityId::CAMERA, Some(camera_transform));

                // Set camera mode
                let maybe_current_camera_mode =
                    SceneCrdtStateProtoComponents::get_camera_mode(crdt_state)
                        .get(&SceneEntityId::CAMERA)
                        .and_then(|camera_mode_value| {
                            camera_mode_value.value.as_ref().map(|v| v.mode)
                        });

                if maybe_current_camera_mode != Some(camera_mode) {
                    let camera_mode_component = PbCameraMode { mode: camera_mode };
                    SceneCrdtStateProtoComponents::get_camera_mode_mut(crdt_state)
                        .put(SceneEntityId::CAMERA, Some(camera_mode_component));
                }

                // Set PointerLock
                let maybe_is_pointer_locked =
                    SceneCrdtStateProtoComponents::get_pointer_lock(crdt_state)
                        .get(&SceneEntityId::CAMERA)
                        .and_then(|pointer_lock_value| {
                            pointer_lock_value
                                .value
                                .as_ref()
                                .map(|v| v.is_pointer_locked)
                        });

                let is_pointer_locked = godot::prelude::Input::singleton().get_mouse_mode()
                    == godot::engine::input::MouseMode::MOUSE_MODE_CAPTURED;
                if maybe_is_pointer_locked != Some(is_pointer_locked) {
                    let pointer_lock_component = PbPointerLock { is_pointer_locked };
                    SceneCrdtStateProtoComponents::get_pointer_lock_mut(crdt_state)
                        .put(SceneEntityId::CAMERA, Some(pointer_lock_component));
                }

                // Process pointer events
                let pointer_events_result_component =
                    SceneCrdtStateProtoComponents::get_pointer_events_result_mut(crdt_state);

                let results = scene.pointer_events_result.drain(0..);
                for (entity, mut value) in results {
                    value.timestamp = scene.tick_number;
                    pointer_events_result_component.append(entity, value);
                }

                let mut ui_results = scene.godot_dcl_scene.ui_results.borrow_mut();
                let results = ui_results.pointer_event_results.drain(0..);
                for (entity, value) in results {
                    pointer_events_result_component.append(entity, value);
                }

                let incoming_comms_message = DclGlobal::singleton()
                    .bind_mut()
                    .comms
                    .bind_mut()
                    .get_pending_messages(&scene.scene_entity_definition.id);

                // Set renderer response to the scene
                let dirty_crdt_state = crdt_state.take_dirty();
                scene.current_dirty.renderer_response = Some(RendererResponse::Ok {
                    dirty_crdt_state: Box::new(dirty_crdt_state),
                    incoming_comms_message,
                });
                false
            }
            SceneUpdateState::ProcessRpcs => {
                let rpc_calls = std::mem::take(&mut scene.current_dirty.rpc_calls);
                process_rpcs(scene, current_parcel_scene_id, rpc_calls);
                false
            }
            SceneUpdateState::SendToThread => {
                // The scene is already processed, but the message was not sent to the thread yet
                if scene.dcl_scene.main_sender_to_thread.capacity() > 0 {
                    let response = scene.current_dirty.renderer_response.take().unwrap();
                    if let Err(_err) = scene
                        .dcl_scene
                        .main_sender_to_thread
                        .blocking_send(response)
                    {
                        // TODO: handle fail sending to thread
                    }

                    scene.current_dirty = scene.enqueued_dirty.pop().unwrap_or(Dirty {
                        waiting_process: false,
                        entities: Default::default(),
                        lww_components: Default::default(),
                        gos_components: Default::default(),
                        logs: Vec::new(),
                        renderer_response: None,
                        update_state: SceneUpdateState::Processed,
                        rpc_calls: Vec::new(),
                    });

                    return true;
                }
                return false;
            }
            SceneUpdateState::Processed => {
                return true;
            }
        };

        const TICK_TIME_LOGABLE_MS: i64 = 16000;
        let this_update_ms = (std::time::Instant::now() - before_compute_update).as_micros() as i64;
        if this_update_ms > TICK_TIME_LOGABLE_MS {
            tracing::warn!(
                "Scene \"{:?}\"(tick={:?}) in state {:?} takes more than {TICK_TIME_LOGABLE_MS}: {:?}ms",
                scene.scene_entity_definition.get_title(),
                scene.tick_number,
                scene.current_dirty.update_state,
                this_update_ms
            );
        }

        if should_break {
            return false;
        }

        scene.current_dirty.update_state = scene.current_dirty.update_state.next();

        current_time_us = (std::time::Instant::now() - *ref_time).as_micros() as i64;
        if current_time_us > end_time_us {
            return false;
        }
    }
}
