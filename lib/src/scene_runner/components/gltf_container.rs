use std::time::Instant;

use crate::{
    dcl::{
        components::{
            proto_components::sdk::components::PbGltfContainerLoadingState, SceneComponentId,
        },
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    godot_classes::dcl_gltf_container::{DclGltfContainer, GltfContainerLoadingState},
    scene_runner::scene::Scene,
};
use godot::prelude::*;

pub fn update_gltf_container(
    scene: &mut Scene,
    crdt_state: &mut SceneCrdtState,
    ref_time: &Instant,
    end_time_us: i64,
) -> bool {
    // Diagnostic: skip every GLTF instantiation when `--skip-gltf` is set.
    // Used to measure the rendering/processing floor with no scene meshes
    // visible (just sky + UI + avatar). Drains the dirty set so we don't
    // re-process forever.
    if crate::godot_classes::dcl_global::DclGlobal::try_singleton()
        .map(|g| g.bind().cli.bind().skip_gltf_load)
        .unwrap_or(false)
    {
        scene
            .current_dirty
            .lww_components
            .remove(&SceneComponentId::GLTF_CONTAINER);
        return true;
    }

    let mut updated_count = 0;
    let mut current_time_us;

    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let gltf_container_dirty = scene
        .current_dirty
        .lww_components
        .remove(&SceneComponentId::GLTF_CONTAINER);
    let scene_id = scene.scene_id.0;
    let gltf_container_component = SceneCrdtStateProtoComponents::get_gltf_container(crdt_state);

    if let Some(mut gltf_container_dirty) = gltf_container_dirty {
        for entity in gltf_container_dirty.iter() {
            let new_value = gltf_container_component.get(entity);
            if new_value.is_none() {
                continue;
            }

            let new_value = new_value.unwrap();
            let (_godot_entity_node, mut node_3d) = godot_dcl_scene.ensure_node_3d(entity);

            let new_value = new_value.value.clone();
            let existing = node_3d.try_get_node_as::<Node>("GltfContainer");

            if new_value.is_none() {
                if let Some(mut gltf_node) = existing {
                    gltf_node.queue_free();
                    node_3d.remove_child(&gltf_node);
                    // If GLTF was still loading when removed, count it as finished
                    if scene.gltf_loading.remove(entity) {
                        scene.gltf_loading_finished_count += 1;
                    }
                }
            } else if let Some(new_value) = new_value {
                let visible_meshes_collision_mask =
                    new_value.visible_meshes_collision_mask.unwrap_or(0) as i32;
                let invisible_meshes_collision_mask =
                    new_value.invisible_meshes_collision_mask.unwrap_or(3) as i32;

                if let Some(mut gltf_node) = existing {
                    gltf_node.call(
                        "change_gltf",
                        &[
                            new_value.src.to_variant(),
                            visible_meshes_collision_mask.to_variant(),
                            invisible_meshes_collision_mask.to_variant(),
                        ],
                    );
                    if scene.gltf_loading.insert(*entity) {
                        scene.gltf_loading_started_count += 1;
                    }
                } else {
                    // TODO: preload this resource
                    let mut new_gltf = godot::tools::load::<PackedScene>(
                        "res://src/decentraland_components/gltf_container.tscn",
                    )
                    .instantiate()
                    .unwrap()
                    .cast::<DclGltfContainer>();

                    let mut new_gltf_ref = new_gltf.bind_mut();
                    new_gltf_ref.set_dcl_gltf_src(new_value.src.to_godot());
                    new_gltf_ref.set_dcl_scene_id(scene_id);
                    new_gltf_ref.set_dcl_entity_id(entity.as_i32());
                    new_gltf_ref.set_dcl_visible_cmask(visible_meshes_collision_mask);
                    new_gltf_ref.set_dcl_invisible_cmask(invisible_meshes_collision_mask);
                    drop(new_gltf_ref);

                    // If entity has an active tween, mark as kinematic so colliders
                    // are created as KINEMATIC in GDScript's _ready()
                    if scene.kinematic_entities.contains(entity) {
                        new_gltf.set_meta("kinematic_requested", &true.to_variant());
                    }

                    new_gltf.set_name("GltfContainer");
                    node_3d.add_child(&new_gltf.clone().upcast::<Node>());

                    if scene.gltf_loading.insert(*entity) {
                        scene.gltf_loading_started_count += 1;
                    }
                }
            }

            updated_count += 1;
            current_time_us = (std::time::Instant::now() - *ref_time).as_micros() as i64;
            if current_time_us > end_time_us {
                break;
            }
        }

        if updated_count < gltf_container_dirty.len() {
            gltf_container_dirty.drain(0..updated_count);
            scene
                .current_dirty
                .lww_components
                .insert(SceneComponentId::GLTF_CONTAINER, gltf_container_dirty);
            return false;
        }
    }

    true
}

pub fn sync_gltf_loading_state(
    scene: &mut Scene,
    crdt_state: &mut SceneCrdtState,
    ref_time: &Instant,
    end_time_us: i64,
) -> bool {
    let mut current_time_us;

    // Sub-instrumentation (gated on the same toggle as [SCENEPROF]) to settle
    // where SyncGltfContainer's per-tick cost goes: the per-entity node lookup
    // (try_get_node_as) vs the — believed dead — async_deferred_add_child call.
    let prof = crate::scene_runner::update_scene::scene_tick_profiling_enabled();
    let scene_id_i = scene.scene_id.0;
    let tick = scene.tick_number;
    let loading_len = scene.gltf_loading.len();
    let mut n_iter: u64 = 0;
    let mut lookup_us: u64 = 0;
    let mut deferred_calls: u64 = 0;
    let mut deferred_us: u64 = 0;
    let mut done = true;

    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let gltf_container_loading_state_component =
        SceneCrdtStateProtoComponents::get_gltf_container_loading_state_mut(crdt_state);

    for entity in scene.gltf_loading.clone().iter() {
        n_iter += 1;
        let t_lookup = if prof { Some(Instant::now()) } else { None };
        let gltf_node = godot_dcl_scene
            .ensure_node_3d(entity)
            .1
            .try_get_node_as::<DclGltfContainer>("GltfContainer");
        if let Some(t) = t_lookup {
            lookup_us += (Instant::now() - t).as_micros() as u64;
        }

        if let Some(mut gltf_node) = gltf_node.clone() {
            if gltf_node.bind().get_dcl_pending_node().is_some() {
                let t_def = Instant::now();
                gltf_node.call("async_deferred_add_child", &[]);
                deferred_calls += 1;
                deferred_us += (Instant::now() - t_def).as_micros() as u64;
            }
        }

        let current_state = match gltf_container_loading_state_component.get(entity) {
            Some(state) => match state.value.as_ref() {
                Some(value) => GltfContainerLoadingState::from_proto(value.current_state()),
                _ => GltfContainerLoadingState::Unknown,
            },
            None => GltfContainerLoadingState::Unknown,
        };

        let current_state_godot = match gltf_node {
            Some(gltf_node) => {
                GltfContainerLoadingState::from_i32(gltf_node.bind().get_dcl_gltf_loading_state())
            }
            None => GltfContainerLoadingState::Unknown,
        };

        if current_state_godot != current_state {
            let pb_gltf_container_loading_state = PbGltfContainerLoadingState {
                current_state: current_state_godot.to_i32(),
            };
            gltf_container_loading_state_component
                .put(*entity, Some(pb_gltf_container_loading_state));
        }

        if (current_state_godot == GltfContainerLoadingState::Finished
            || current_state_godot == GltfContainerLoadingState::FinishedWithError
            || current_state_godot == GltfContainerLoadingState::NotFound)
            && scene.gltf_loading.remove(entity)
        {
            scene.gltf_loading_finished_count += 1;

            // When GLTF finishes loading, mark entity for GltfNodeModifiers re-application
            // (modifiers need to be applied to newly loaded nodes)
            // Only add if entity actually has the GltfNodeModifiers component
            if current_state_godot == GltfContainerLoadingState::Finished {
                let has_state = scene.gltf_node_modifier_states.contains_key(entity);
                let has_dirty = scene
                    .current_dirty
                    .lww_components
                    .get(&SceneComponentId::GLTF_NODE_MODIFIERS)
                    .is_some_and(|dirty| dirty.contains(entity));
                tracing::debug!(
                    "sync_gltf_loading_state: entity {:?} state=Finished, has_modifier_state={}, has_modifier_dirty={}, gltf_loading.len()={}",
                    entity,
                    has_state,
                    has_dirty,
                    scene.gltf_loading.len()
                );
                if has_state || has_dirty {
                    tracing::debug!("sync_gltf_loading_state: Adding entity {:?} to gltf_node_modifiers_pending (will be removed from gltf_loading)", entity);
                    scene.gltf_node_modifiers_pending.insert(*entity);
                }
            }
        }

        current_time_us = (std::time::Instant::now() - *ref_time).as_micros() as i64;
        if current_time_us > end_time_us {
            done = false;
            break;
        }
    }

    // Stuck-entity watchdog (issue #1640 "infinite loading"): once the loading
    // set has shrunk to a small tail that isn't draining, periodically name each
    // remaining entity (src + Godot loading state) so a hang can be traced to the
    // exact asset that never reaches Finished/Error. Throttled + tail-gated so it
    // stays silent during the bulk load. Cross-reference the entity/src against
    // the LOADPROF `asset.gltf_start` line and the [DLPROF] download timing.
    if prof && tick % 60 == 0 {
        let remaining = scene.gltf_loading.len();
        if remaining > 0 && remaining <= 8 {
            for entity in scene.gltf_loading.iter() {
                let node = godot_dcl_scene
                    .ensure_node_3d(entity)
                    .1
                    .try_get_node_as::<DclGltfContainer>("GltfContainer");
                let (state, src) = match node {
                    Some(n) => {
                        let b = n.bind();
                        (
                            b.get_dcl_gltf_loading_state(),
                            b.get_dcl_gltf_src().to_string(),
                        )
                    }
                    None => (-1, String::from("<no-node>")),
                };
                tracing::info!(
                    "[GLTFSTUCK] scene={} tick={} entity={} state={} src={}",
                    scene_id_i,
                    tick,
                    entity.as_i32(),
                    state,
                    src
                );
            }
        }
    }

    if prof {
        tracing::info!(
            "[GLTFSYNC] scene={} tick={} loading_len={} iterated={} lookup_us={} deferred_calls={} deferred_us={} done={}",
            scene_id_i,
            tick,
            loading_len,
            n_iter,
            lookup_us,
            deferred_calls,
            deferred_us,
            done
        );
    }
    done
}
