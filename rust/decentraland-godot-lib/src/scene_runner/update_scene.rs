use godot::prelude::{Share, Transform3D};

use super::{
    components::{
        gltf_container::update_gltf_container, material::update_material,
        mesh_collider::update_mesh_collider, mesh_renderer::update_mesh_renderer,
        pointer_events::update_scene_pointer_events, text_shape::update_text_shape,
        transform_and_parent::update_transform_and_parent,
    },
    scene_manager::Scene,
};
use crate::dcl::{
    components::{transform_and_parent::DclTransformAndParent, SceneEntityId},
    crdt::{
        grow_only_set::GenericGrowOnlySetComponentOperation,
        last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
        SceneCrdtStateProtoComponents,
    },
};

pub fn update_scene(
    _dt: f64,
    scene: &mut Scene,
    crdt_state: &mut SceneCrdtState,
    camera_global_transform: &Transform3D,
    player_global_transform: &Transform3D,
) {
    scene.waiting_for_updates = false;

    update_deleted_entities(scene);
    update_transform_and_parent(scene, crdt_state);
    update_mesh_renderer(scene, crdt_state);
    update_scene_pointer_events(scene, crdt_state);
    update_material(scene, crdt_state);
    update_text_shape(scene, crdt_state);
    update_mesh_collider(scene, crdt_state);
    update_gltf_container(scene, crdt_state);

    let camera_transform = DclTransformAndParent::from_godot(
        camera_global_transform,
        scene.godot_dcl_scene.root_node.get_position(),
    );
    let player_transform = DclTransformAndParent::from_godot(
        player_global_transform,
        scene.godot_dcl_scene.root_node.get_position(),
    );
    crdt_state
        .get_transform_mut()
        .put(SceneEntityId::PLAYER, Some(player_transform));
    crdt_state
        .get_transform_mut()
        .put(SceneEntityId::CAMERA, Some(camera_transform));

    let pointer_events_result_component =
        SceneCrdtStateProtoComponents::get_pointer_events_result_mut(crdt_state);

    let results = scene.pointer_events_result.drain(0..);
    for (entity, value) in results {
        pointer_events_result_component.append(entity, value);
    }
}

fn update_deleted_entities(scene: &mut Scene) {
    if scene.current_dirty.entities.died.is_empty() {
        return;
    }

    let mut godot_dcl_scene = &mut scene.godot_dcl_scene;
    let died = &scene.current_dirty.entities.died;

    for (entity_id, node) in godot_dcl_scene.entities.iter_mut() {
        if died.contains(&node.computed_parent) && *entity_id != node.computed_parent {
            node.base
                .reparent(godot_dcl_scene.root_node.share().upcast(), false);
            node.computed_parent = SceneEntityId::ROOT;
            godot_dcl_scene.unparented_entities.insert(*entity_id);
            godot_dcl_scene.hierarchy_dirty = true;
        }
    }

    for deleted_entity in died.iter() {
        let node = godot_dcl_scene.ensure_node_mut(deleted_entity);
        node.base.share().free();
        godot_dcl_scene.entities.remove(deleted_entity);
    }
}
