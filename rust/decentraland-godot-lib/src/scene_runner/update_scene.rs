use std::time::Instant;

use godot::prelude::{Callable, GodotString, ToVariant, Transform3D, VariantArray};

use super::{
    components::{
        animator::update_animator, audio_source::update_audio_source,
        audio_stream::update_audio_stream, avatar_attach::update_avatar_attach,
        avatar_shape::update_avatar_shape, billboard::update_billboard,
        camera_mode_area::update_camera_mode_area, gltf_container::update_gltf_container,
        material::update_material, mesh_collider::update_mesh_collider,
        mesh_renderer::update_mesh_renderer, pointer_events::update_scene_pointer_events,
        raycast::update_raycasts, text_shape::update_text_shape,
        transform_and_parent::update_transform_and_parent, ui::scene_ui::update_scene_ui,
        video_player::update_video_player, visibility::update_visibility,
    },
    deleted_entities::update_deleted_entities,
    rpc_calls::process_rpcs,
    scene::{Dirty, Scene, SceneUpdateState},
};
use crate::{
    common::rpc::RpcCalls,
    dcl::{
        components::{
            proto_components::sdk::components::{
                PbCameraMode, PbEngineInfo, PbUiCanvasInformation,
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
        let should_break = match scene.current_dirty.update_state {
            SceneUpdateState::None => {
                let engine_info_component =
                    SceneCrdtStateProtoComponents::get_engine_info_mut(crdt_state);
                let tick_number =
                    if let Some(entry) = engine_info_component.get(SceneEntityId::ROOT) {
                        if let Some(value) = entry.value.as_ref() {
                            value.tick_number + 1
                        } else {
                            0
                        }
                    } else {
                        0
                    };
                engine_info_component.put(
                    SceneEntityId::ROOT,
                    Some(PbEngineInfo {
                        tick_number,
                        frame_number: frames_count as u32,
                        total_runtime: (Instant::now() - scene.start_time).as_secs_f32(),
                    }),
                );
                false
            }
            SceneUpdateState::PrintLogs => {
                // enable logs
                for log in &scene.current_dirty.logs {
                    let mut arguments = VariantArray::new();
                    arguments.push((scene.scene_id.0 as i32).to_variant());
                    arguments.push((log.level as i32).to_variant());
                    arguments.push((log.timestamp as f32).to_variant());
                    arguments.push(GodotString::from(&log.message).to_variant());
                    console.callv(arguments);
                }
                false
            }
            SceneUpdateState::DeletedEntities => {
                update_deleted_entities(scene);
                false
            }
            SceneUpdateState::TransformAndParent => {
                update_transform_and_parent(scene, crdt_state);
                false
            }
            SceneUpdateState::VisibilityComponent => {
                update_visibility(scene, crdt_state);
                false
            }
            SceneUpdateState::MeshRenderer => {
                update_mesh_renderer(scene, crdt_state);
                false
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
                update_gltf_container(scene, crdt_state);
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
            SceneUpdateState::Raycasts => {
                update_raycasts(scene, crdt_state);
                false
            }
            SceneUpdateState::AvatarAttach => {
                update_avatar_attach(scene, crdt_state);
                false
            }
            SceneUpdateState::VideoPlayer => {
                update_video_player(scene, crdt_state, current_parcel_scene_id);
                false
            }
            SceneUpdateState::AudioStream => {
                update_audio_stream(scene, crdt_state, current_parcel_scene_id);
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
                let camera_transform = DclTransformAndParent::from_godot(
                    camera_global_transform,
                    scene.godot_dcl_scene.root_node_3d.get_position(),
                );
                let player_transform = DclTransformAndParent::from_godot(
                    player_global_transform,
                    scene.godot_dcl_scene.root_node_3d.get_position(),
                );
                crdt_state
                    .get_transform_mut()
                    .put(SceneEntityId::PLAYER, Some(player_transform));
                crdt_state
                    .get_transform_mut()
                    .put(SceneEntityId::CAMERA, Some(camera_transform));

                let maybe_current_camera_mode = {
                    if let Some(camera_mode_value) =
                        SceneCrdtStateProtoComponents::get_camera_mode(crdt_state)
                            .get(SceneEntityId::CAMERA)
                    {
                        camera_mode_value
                            .value
                            .as_ref()
                            .map(|camera_mode_value| camera_mode_value.mode)
                    } else {
                        None
                    }
                };

                if maybe_current_camera_mode.is_none()
                    || maybe_current_camera_mode.unwrap() != camera_mode
                {
                    let camera_mode_component = PbCameraMode { mode: camera_mode };
                    SceneCrdtStateProtoComponents::get_camera_mode_mut(crdt_state)
                        .put(SceneEntityId::CAMERA, Some(camera_mode_component));
                }

                let pointer_events_result_component =
                    SceneCrdtStateProtoComponents::get_pointer_events_result_mut(crdt_state);

                let results = scene.pointer_events_result.drain(0..);
                for (entity, value) in results {
                    pointer_events_result_component.append(entity, value);
                }

                let mut ui_results = scene.godot_dcl_scene.ui_results.borrow_mut();
                let results = ui_results.pointer_event_results.drain(0..);
                for (entity, value) in results {
                    pointer_events_result_component.append(entity, value);
                }

                let dirty = crdt_state.take_dirty();
                scene.current_dirty.renderer_response = Some(RendererResponse::Ok(dirty));
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
                        rpc_calls: RpcCalls::default(),
                    });

                    return true;
                }
                return false;
            }
            SceneUpdateState::Processed => {
                return true;
            }
        };

        if should_break {
            return false;
        }

        // let prev = scene.current_dirty.update_state;
        scene.current_dirty.update_state = scene.current_dirty.update_state.next();

        current_time_us = (std::time::Instant::now() - *ref_time).as_micros() as i64;
        if current_time_us > end_time_us {
            // let diff = current_time_us - end_time_us;
            // if diff > 3000 {
            //     println!("exceed time limit by {:?} in the state {:?}", diff, prev);
            // }
            return false;
        }
    }
}
