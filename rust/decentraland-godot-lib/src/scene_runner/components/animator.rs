use crate::{
    dcl::{
        components::SceneComponentId,
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    godot_classes::{animator_controller::apply_anims, dcl_gltf_container::DclGltfContainer},
    scene_runner::{godot_dcl_scene::GodotEntityNode, scene::Scene},
};
use godot::prelude::*;

fn get_gltf_container(godot_entity_node: &mut GodotEntityNode) -> Option<Gd<DclGltfContainer>> {
    godot_entity_node
        .base_3d
        .as_ref()?
        .try_get_node_as::<DclGltfContainer>(NodePath::from("GltfContainer"))
}

pub fn update_animator(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_lww_components = &scene.current_dirty.lww_components;
    if let Some(animator_dirty) = dirty_lww_components.get(&SceneComponentId::ANIMATOR) {
        let animator_component = SceneCrdtStateProtoComponents::get_animator(crdt_state);

        for entity in animator_dirty {
            let new_value = animator_component.get(entity);
            if new_value.is_none() {
                scene.dup_animator.remove(entity);
                continue;
            }

            let entry = new_value.unwrap();
            let (godot_entity_node, _node_3d) = godot_dcl_scene.ensure_node_3d(entity);

            let Some(gltf_container_node) = get_gltf_container(godot_entity_node) else {
                let value = entry.value.clone();
                if let Some(value) = value {
                    scene.dup_animator.insert(*entity, value);
                } else {
                    scene.dup_animator.remove(entity);
                }
                continue;
            };

            let value = entry.value.clone().unwrap_or_default();
            apply_anims(gltf_container_node.upcast(), &value);

            if entry.value.is_none() {
                scene.dup_animator.remove(entity);
            } else {
                scene.dup_animator.insert(*entity, value);
            }
        }
    }
}
