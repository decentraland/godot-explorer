use godot::prelude::Share;

use super::{
    components::{
        mesh_renderer::update_mesh_renderer, transform_and_parent::update_transform_and_parent,
    },
    Scene,
};
use crate::dcl::{components::SceneEntityId, crdt::SceneCrdtState, DirtyComponents, DirtyEntities};

pub fn update_scene(
    _dt: f64,
    scene: &mut Scene,
    crdt_state: &mut SceneCrdtState,
    dirty_entities: &DirtyEntities,
    dirty_components: &DirtyComponents,
) {
    scene.waiting_for_updates = false;

    update_deleted_entities(&mut scene.godot_dcl_scene, &dirty_entities.died);
    update_transform_and_parent(&mut scene.godot_dcl_scene, crdt_state, dirty_components);
    update_mesh_renderer(&mut scene.godot_dcl_scene, crdt_state, dirty_components);
}

fn update_deleted_entities(
    godot_dcl_scene: &mut super::godot_dcl_scene::GodotDclScene,
    died: &std::collections::HashSet<crate::dcl::components::SceneEntityId>,
) {
    if died.is_empty() {
        return;
    }

    for (entity_id, node) in godot_dcl_scene.entities.iter_mut() {
        if died.contains(&node.computed_parent) && *entity_id != node.computed_parent {
            node.base
                .reparent(godot_dcl_scene.root_node.share().upcast(), false);
            node.computed_parent = SceneEntityId::ROOT;
            godot_dcl_scene.unparented_entities.insert(*entity_id);
        }
    }

    for deleted_entity in died.iter() {
        let node = godot_dcl_scene.ensure_node_mut(deleted_entity);
        node.base.share().free();
        godot_dcl_scene.entities.remove(deleted_entity);
    }
}
