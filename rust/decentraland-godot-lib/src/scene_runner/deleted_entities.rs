use crate::dcl::components::SceneEntityId;

use super::scene::Scene;

pub fn update_deleted_entities(scene: &mut Scene) {
    if scene.current_dirty.entities.died.is_empty() {
        return;
    }

    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let died = &scene.current_dirty.entities.died;

    for (entity_id, node) in godot_dcl_scene.entities.iter_mut() {
        if died.contains(&node.computed_parent) && *entity_id != node.computed_parent {
            node.base
                .reparent_ex(godot_dcl_scene.root_node.clone().upcast())
                .keep_global_transform(false)
                .done();
            node.computed_parent = SceneEntityId::ROOT;
            godot_dcl_scene.unparented_entities.insert(*entity_id);
            godot_dcl_scene.hierarchy_dirty = true;
        }
    }

    for deleted_entity in died.iter() {
        let node = godot_dcl_scene.ensure_node_mut(deleted_entity);
        node.base.clone().free();
        godot_dcl_scene.entities.remove(deleted_entity);
        scene.audio_sources.remove(deleted_entity);
        scene.audio_streams.remove(deleted_entity);
        scene.audio_video_players.remove(deleted_entity);
    }
}
