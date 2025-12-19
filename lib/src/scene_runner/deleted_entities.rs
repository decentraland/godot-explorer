use crate::dcl::components::{
    proto_components::sdk::components::TriggerAreaMeshType, SceneEntityId,
};

use super::{
    components::trigger_area::unregister_trigger_area, pool_manager::PoolManager, scene::Scene,
};

pub fn update_deleted_entities(scene: &mut Scene, pools: &mut PoolManager) {
    if scene.current_dirty.entities.died.is_empty() {
        return;
    }

    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let died = &scene.current_dirty.entities.died;

    for (entity_id, node) in godot_dcl_scene.entities.iter_mut() {
        if died.contains(&node.computed_parent_3d) && *entity_id != node.computed_parent_3d {
            if let Some(node_3d) = node.base_3d.as_mut() {
                node_3d
                    .reparent_ex(godot_dcl_scene.root_node_3d.clone().upcast())
                    .keep_global_transform(false)
                    .done();
            }
            node.computed_parent_3d = SceneEntityId::ROOT;
            godot_dcl_scene.unparented_entities_3d.insert(*entity_id);
            godot_dcl_scene.hierarchy_dirty_3d = true;
        }
    }

    for deleted_entity in died.iter() {
        if let Some(godot_entity_node) = godot_dcl_scene.get_godot_entity_node_mut(deleted_entity) {
            if let Some(node_3d) = godot_entity_node.base_3d.as_mut() {
                node_3d.queue_free();
            }
            if let Some(node_ui) = godot_entity_node.base_ui.as_mut() {
                node_ui.base_control.queue_free();
            }
        }

        godot_dcl_scene.entities.remove(deleted_entity);

        scene.audio_sources.remove(deleted_entity);
        scene.audio_streams.remove(deleted_entity);
        scene.video_players.remove(deleted_entity);
        scene.dup_animator.remove(deleted_entity);
        scene.gltf_loading.remove(deleted_entity);
        scene.continuos_raycast.remove(deleted_entity);
        scene.tweens.remove(deleted_entity);
        scene.texture_animations.remove(deleted_entity);

        // Clean up trigger area - unregister from monitor and release back to pool
        if let Some(instance) = scene.trigger_areas.instances.remove(deleted_entity) {
            unregister_trigger_area(instance.area_rid);
            let pool = pools.physics_area();
            pool.release_area(instance.area_rid);
            match instance.mesh_type {
                TriggerAreaMeshType::TamtBox => {
                    pool.release_box_shape(instance.shape_rid);
                }
                TriggerAreaMeshType::TamtSphere => {
                    pool.release_sphere_shape(instance.shape_rid);
                }
            }
        }

        // Clean up trigger area - unregister from monitor and release back to pool
        if let Some(instance) = scene.trigger_areas.instances.remove(deleted_entity) {
            unregister_trigger_area(instance.area_rid);
            let pool = pools.physics_area();
            pool.release_area(instance.area_rid);
            match instance.mesh_type {
                TriggerAreaMeshType::TamtBox => {
                    pool.release_box_shape(instance.shape_rid);
                }
                TriggerAreaMeshType::TamtSphere => {
                    pool.release_sphere_shape(instance.shape_rid);
                }
            }
        }

        scene.pointer_events_result = scene
            .pointer_events_result
            .drain(..)
            .filter(|(entity, _)| entity != deleted_entity)
            .collect();
    }
}
