use std::{
    cell::RefCell,
    collections::HashMap,
    sync::{atomic::Ordering, Mutex},
    time::Instant,
};

use godot::{
    obj::Singleton,
    prelude::{varray, Callable, ToGodot, Transform3D},
};

/// Per-state cumulative CPU timing across all scene threads. Read+reset from
/// the GP benchmark runner to dump a per-state breakdown of where the per-frame
/// scene_runner cost actually goes (the existing 16ms warning logs only catch
/// load-time spikes — steady-state per-frame timing needs aggregation).
///
/// Gated on `STATE_TIMING_ENABLED` because the per-state lock acquire is
/// hit ~30 times per scene per tick, and across many scene threads the
/// global mutex serializes everything — measured 50 % FPS regression on
/// Genesis Plaza when always-on. The bench runner flips it on right before
/// the sampling window.
static STATE_TIMING: Mutex<Option<HashMap<&'static str, (u64, u64)>>> = Mutex::new(None);
static STATE_TIMING_ENABLED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

#[inline]
fn record_state_timing(state_name: &'static str, us: u64) {
    if !STATE_TIMING_ENABLED.load(Ordering::Relaxed) {
        return;
    }
    if let Ok(mut guard) = STATE_TIMING.lock() {
        let map = guard.get_or_insert_with(HashMap::new);
        let entry = map.entry(state_name).or_insert((0u64, 0u64));
        entry.0 = entry.0.saturating_add(us);
        entry.1 = entry.1.saturating_add(1);
    }
}

/// Drain and clear the per-state timing buckets. Returns a multiline string
/// like `MeshRenderer=12345us(120)\n...` for embedding in benchmark JSON.
/// Side-effect: leaves recording disabled so post-sampling work doesn't
/// pollute the next sample window.
pub fn drain_state_timing() -> String {
    STATE_TIMING_ENABLED.store(false, Ordering::Relaxed);
    let Ok(mut guard) = STATE_TIMING.lock() else {
        return String::new();
    };
    let Some(map) = guard.take() else {
        return String::new();
    };
    let mut entries: Vec<(&'static str, u64, u64)> =
        map.into_iter().map(|(k, (us, n))| (k, us, n)).collect();
    entries.sort_unstable_by_key(|e| std::cmp::Reverse(e.1));
    let mut out = String::new();
    for (name, us, n) in entries {
        out.push_str(&format!("{}={}us({})\n", name, us, n));
    }
    out
}

pub fn reset_state_timing() {
    if let Ok(mut guard) = STATE_TIMING.lock() {
        *guard = None;
    }
    STATE_TIMING_ENABLED.store(true, Ordering::Relaxed);
}

fn state_name(state: &super::scene::SceneUpdateState) -> &'static str {
    use super::scene::SceneUpdateState as S;
    match state {
        S::None => "None",
        S::PrintLogs => "PrintLogs",
        S::DeletedEntities => "DeletedEntities",
        S::Tween => "Tween",
        S::TransformAndParent => "TransformAndParent",
        S::VisibilityComponent => "VisibilityComponent",
        S::MeshRenderer => "MeshRenderer",
        S::ScenePointerEvents => "ScenePointerEvents",
        S::Material => "Material",
        S::TextShape => "TextShape",
        S::Billboard => "Billboard",
        S::MeshCollider => "MeshCollider",
        S::GltfContainer => "GltfContainer",
        S::SyncGltfContainer => "SyncGltfContainer",
        S::GltfNodeModifiers => "GltfNodeModifiers",
        S::NftShape => "NftShape",
        S::Animator => "Animator",
        S::AvatarShape => "AvatarShape",
        S::AvatarShapeEmoteCommand => "AvatarShapeEmoteCommand",
        S::Raycasts => "Raycasts",
        S::AvatarAttach => "AvatarAttach",
        S::SceneUi => "SceneUi",
        S::VideoPlayer => "VideoPlayer",
        S::AudioStream => "AudioStream",
        S::AvatarModifierArea => "AvatarModifierArea",
        S::AvatarLocomotionSettings => "AvatarLocomotionSettings",
        S::CameraModeArea => "CameraModeArea",
        S::InputModifier => "InputModifier",
        S::SkyboxTime => "SkyboxTime",
        S::TriggerArea => "TriggerArea",
        S::VirtualCameras => "VirtualCameras",
        S::AudioSource => "AudioSource",
        S::ProcessRpcs => "ProcessRpcs",
        S::ComputeCrdtState => "ComputeCrdtState",
        S::SendToThread => "SendToThread",
        S::Processed => "Processed",
    }
}

use super::{
    components::{
        animator::update_animator,
        audio_source::update_audio_source,
        avatar_attach::update_avatar_attach,
        avatar_data::update_avatar_scene_updates,
        avatar_locomotion_settings::update_avatar_locomotion_settings,
        avatar_modifier_area::update_avatar_modifier_area,
        avatar_shape::update_avatar_shape,
        billboard::update_billboard,
        camera_mode_area::update_camera_mode_area,
        gltf_container::{sync_gltf_loading_state, update_gltf_container},
        gltf_node_modifiers::{
            update_gltf_node_modifiers, update_modifier_textures, update_modifier_video_textures,
        },
        input_modifier::update_input_modifier,
        material::{update_material, update_video_material_textures},
        mesh_collider::update_mesh_collider,
        mesh_renderer::update_mesh_renderer,
        nft_shape::update_nft_shape,
        pointer_events::update_scene_pointer_events,
        raycast::update_raycasts,
        realm_info::sync_realm_info,
        skybox_time::update_skybox_time,
        text_shape::update_text_shape,
        transform_and_parent::update_transform_and_parent,
        trigger_area::update_trigger_area,
        tween::update_tween,
        ui::scene_ui::update_scene_ui,
        video_player::update_video_player,
        visibility::update_visibility,
    },
    deleted_entities::update_deleted_entities,
    pool_manager::PoolManager,
    rpc_calls::process_rpcs,
    scene::{Dirty, Scene, SceneType, SceneUpdateState},
};
use crate::{
    dcl::{
        components::{
            proto_components::sdk::components::{
                PbCameraMode, PbEngineInfo, PbMainCamera, PbPointerLock, PbUiCanvasInformation,
            },
            transform_and_parent::DclTransformAndParent,
            SceneComponentId, SceneEntityId,
        },
        crdt::{
            grow_only_set::GenericGrowOnlySetComponentOperation,
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtStateProtoComponents,
        },
        RendererResponse, SceneId,
    },
    godot_classes::dcl_global::DclGlobal,
    scene_runner::components::{
        audio_stream::update_audio_stream, avatar_shape::update_avatar_shape_emote_command,
        virtual_cameras::update_main_and_virtual_cameras,
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
    pool_manager: &RefCell<PoolManager>,
    force_complete: bool,
    bench_disable_tweens: bool,
    bench_disable_transforms: bool,
) -> bool {
    let crdt = scene.dcl_scene.scene_crdt.clone();

    // When force_complete is set, use a generous 2-second time budget so the state machine
    // processes to completion in a single call. This prevents the scene thread from being
    // blocked indefinitely when the normal time budget would cause deferral across frames,
    // while still capping execution to avoid freezing the client on buggy scenes.
    const FORCE_COMPLETE_BUDGET_US: i64 = 2_000_000; // 2 seconds
    let effective_end_time_us = if force_complete {
        (ref_time.elapsed().as_micros() as i64).saturating_add(FORCE_COMPLETE_BUDGET_US)
    } else {
        end_time_us
    };

    // Outer loop: handles both locked (CRDT) and unlocked phases.
    // States after ComputeCrdtState (ProcessRpcs, SendToThread, Processed) don't need
    // the CRDT lock. Releasing it before the channel send avoids holding the shared mutex
    // while the scene thread may be trying to lock it after receiving.
    loop {
        // Phase 1: Handle states that don't need the CRDT lock
        match scene.current_dirty.update_state {
            SceneUpdateState::ProcessRpcs => {
                let rpc_calls = std::mem::take(&mut scene.current_dirty.rpc_calls);
                process_rpcs(scene, current_parcel_scene_id, rpc_calls);
                scene.current_dirty.update_state = scene.current_dirty.update_state.next();
                continue;
            }
            SceneUpdateState::SendToThread => {
                let cap = scene.dcl_scene.main_sender_to_thread.capacity();
                if cap > 0 {
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
            _ => {} // Fall through to Phase 2
        }

        // Phase 2: Process states that need the CRDT lock
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
                    if tick_number <= 3 && !scene.gltf_loading.is_empty() && !force_complete {
                        sync_gltf_loading_state(scene, crdt_state, ref_time, effective_end_time_us);

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

                        let main_camera =
                            SceneCrdtStateProtoComponents::get_main_camera_mut(crdt_state);
                        main_camera.put(
                            SceneEntityId::CAMERA,
                            Some(PbMainCamera {
                                virtual_camera_entity: None,
                            }),
                        );
                    }

                    // PbRealmInfo
                    sync_realm_info(scene, crdt_state);

                    false
                }
                SceneUpdateState::PrintLogs => {
                    // enable logs
                    for log in &scene.current_dirty.logs {
                        let arguments = varray![
                            scene.scene_id.0,
                            log.level as i32,
                            log.timestamp as f32,
                            log.message.to_godot()
                        ];
                        console.callv(&arguments);
                    }
                    false
                }
                SceneUpdateState::DeletedEntities => {
                    update_deleted_entities(scene, &mut pool_manager.borrow_mut());
                    false
                }
                SceneUpdateState::Tween => {
                    if !bench_disable_tweens {
                        update_tween(scene, crdt_state);
                    }
                    false
                }
                SceneUpdateState::TransformAndParent => {
                    if bench_disable_transforms {
                        // Drop the dirty set for this component without applying it,
                        // so the next state advances normally and we don't re-enter
                        // this branch every tick.
                        scene
                            .current_dirty
                            .lww_components
                            .remove(&SceneComponentId::TRANSFORM);
                        false
                    } else {
                        !update_transform_and_parent(
                            scene,
                            crdt_state,
                            ref_time,
                            effective_end_time_us,
                        )
                    }
                }
                SceneUpdateState::VisibilityComponent => {
                    update_visibility(scene, crdt_state);
                    false
                }
                SceneUpdateState::MeshRenderer => {
                    !update_mesh_renderer(scene, crdt_state, ref_time, effective_end_time_us)
                }
                SceneUpdateState::ScenePointerEvents => {
                    update_scene_pointer_events(scene, crdt_state);
                    false
                }
                SceneUpdateState::Material => {
                    update_material(scene, crdt_state);
                    // Update video textures separately (needs mutable access to video_players)
                    update_video_material_textures(scene);
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
                    !update_gltf_container(scene, crdt_state, ref_time, effective_end_time_us)
                }
                SceneUpdateState::SyncGltfContainer => {
                    !sync_gltf_loading_state(scene, crdt_state, ref_time, effective_end_time_us)
                }
                SceneUpdateState::GltfNodeModifiers => {
                    tracing::debug!("Entering GltfNodeModifiers state");
                    let still_processing = !update_gltf_node_modifiers(
                        scene,
                        crdt_state,
                        ref_time,
                        effective_end_time_us,
                    );
                    tracing::debug!(
                        "GltfNodeModifiers update complete, still_processing={}",
                        still_processing
                    );
                    // Only check textures when we're done with the main update (avoid redundant work)
                    if !still_processing {
                        // Check and apply pending textures for modifier materials
                        update_modifier_textures(scene);
                        // Update video textures (needs mutable access to video_players)
                        update_modifier_video_textures(scene);
                    }
                    still_processing
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
                SceneUpdateState::VideoPlayer => {
                    update_video_player(scene, crdt_state, current_parcel_scene_id);
                    false
                }
                SceneUpdateState::AudioStream => {
                    update_audio_stream(scene, crdt_state, current_parcel_scene_id);
                    false
                }
                SceneUpdateState::AvatarModifierArea => {
                    update_avatar_modifier_area(scene, crdt_state);
                    false
                }
                SceneUpdateState::AvatarLocomotionSettings => {
                    let changed = update_avatar_locomotion_settings(scene, crdt_state);
                    // Emit signal deferred if locomotion settings changed for the current scene
                    if changed && scene.scene_id == *current_parcel_scene_id {
                        let settings = scene.locomotion_settings.clone();
                        DclGlobal::singleton()
                            .bind()
                            .scene_runner
                            .clone()
                            .call_deferred(
                                "emit_signal",
                                &[
                                    "locomotion_settings_changed".to_variant(),
                                    settings.to_variant(),
                                ],
                            );
                    }
                    false
                }
                SceneUpdateState::CameraModeArea => {
                    update_camera_mode_area(scene, crdt_state);
                    false
                }
                SceneUpdateState::InputModifier => {
                    update_input_modifier(scene, crdt_state, current_parcel_scene_id);
                    false
                }
                SceneUpdateState::SkyboxTime => {
                    update_skybox_time(scene, crdt_state, current_parcel_scene_id);
                    false
                }
                SceneUpdateState::TriggerArea => {
                    update_trigger_area(
                        scene,
                        crdt_state,
                        &mut pool_manager.borrow_mut(),
                        current_parcel_scene_id,
                    );
                    false
                }
                SceneUpdateState::VirtualCameras => {
                    update_main_and_virtual_cameras(scene, crdt_state);
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
                            .call_deferred("emit_signal", &["tree_changed".to_variant()]);
                        scene.godot_dcl_scene.hierarchy_changed_3d = false;
                    }

                    // Set transforms
                    {
                        let camera_transform = DclTransformAndParent::from_godot(
                            camera_global_transform,
                            scene.godot_dcl_scene.root_node_3d.get_position(),
                        );
                        let player_transform = DclTransformAndParent::from_godot(
                            player_global_transform,
                            scene.godot_dcl_scene.root_node_3d.get_position(),
                        );

                        let transform_mut = crdt_state.get_transform_mut();

                        let stored_player_transform = transform_mut
                            .get(&SceneEntityId::PLAYER)
                            .and_then(|value| value.value.as_ref());
                        if stored_player_transform.map(|value| &value.translation)
                            != Some(&player_transform.translation)
                        {
                            transform_mut.put(SceneEntityId::PLAYER, Some(player_transform));
                        }

                        let stored_camera_transform = transform_mut
                            .get(&SceneEntityId::CAMERA)
                            .and_then(|value| value.value.as_ref());
                        if stored_camera_transform != Some(&camera_transform) {
                            transform_mut.put(SceneEntityId::CAMERA, Some(camera_transform));
                        }
                    }

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

                    let is_pointer_locked = godot::classes::Input::singleton().get_mouse_mode()
                        == godot::classes::input::MouseMode::CAPTURED;
                    if maybe_is_pointer_locked != Some(is_pointer_locked) {
                        let pointer_lock_component = PbPointerLock { is_pointer_locked };
                        SceneCrdtStateProtoComponents::get_pointer_lock_mut(crdt_state)
                            .put(SceneEntityId::CAMERA, Some(pointer_lock_component));
                    }

                    // Process pointer events
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

                    // Process trigger area results
                    if !scene.trigger_area_results.is_empty() {
                        let trigger_area_result_component =
                            SceneCrdtStateProtoComponents::get_trigger_area_result_mut(crdt_state);
                        let results = scene.trigger_area_results.drain(0..);
                        for (entity, value) in results {
                            trigger_area_result_component.append(entity, value);
                        }
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
                // These states don't need the CRDT lock — handled above after lock is dropped
                SceneUpdateState::ProcessRpcs
                | SceneUpdateState::SendToThread
                | SceneUpdateState::Processed => break,
            };

            const TICK_TIME_LOGABLE_MS: i64 = 16000;
            let this_update_us =
                (std::time::Instant::now() - before_compute_update).as_micros() as i64;
            record_state_timing(
                state_name(&scene.current_dirty.update_state),
                this_update_us as u64,
            );
            if this_update_us > TICK_TIME_LOGABLE_MS {
                tracing::warn!(
                "Scene \"{:?}\"(tick={:?}) in state {:?} takes more than {TICK_TIME_LOGABLE_MS}: {:?}us",
                scene.scene_entity_definition.get_title(),
                scene.tick_number,
                scene.current_dirty.update_state,
                this_update_us
            );
            }

            if should_break && !force_complete {
                return false;
            }

            scene.current_dirty.update_state = scene.current_dirty.update_state.next();

            current_time_us = (std::time::Instant::now() - *ref_time).as_micros() as i64;
            if current_time_us > effective_end_time_us {
                return false;
            }
        }
        // CRDT lock (crdt_state) dropped here, outer loop continues to Phase 1
    }
}
