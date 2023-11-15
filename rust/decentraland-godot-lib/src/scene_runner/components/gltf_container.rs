use crate::{
    dcl::{
        components::SceneComponentId,
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    godot_classes::dcl_gltf_container::{DclGltfContainer, GltfContainerLoadingState},
    scene_runner::scene::Scene,
};
use godot::prelude::*;

pub fn update_gltf_container(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_lww_components = &scene.current_dirty.lww_components;
    let scene_id = scene.scene_id.0;
    let gltf_container_component = SceneCrdtStateProtoComponents::get_gltf_container(crdt_state);

    if let Some(gltf_container_dirty) = dirty_lww_components.get(&SceneComponentId::GLTF_CONTAINER)
    {
        for entity in gltf_container_dirty {
            let new_value = gltf_container_component.get(entity);
            if new_value.is_none() {
                continue;
            }

            let new_value = new_value.unwrap();
            let (_godot_entity_node, mut node_3d) = godot_dcl_scene.ensure_node_3d(entity);

            let new_value = new_value.value.clone();
            let existing = node_3d.try_get_node_as::<Node>(NodePath::from("GltfContainer"));

            if new_value.is_none() {
                if let Some(gltf_node) = existing {
                    node_3d.remove_child(gltf_node);
                    scene.gltf_loading.remove(entity);
                }
            } else if let Some(new_value) = new_value {
                let visible_meshes_collision_mask =
                    new_value.visible_meshes_collision_mask.unwrap_or(0) as i32;
                let invisible_meshes_collision_mask =
                    new_value.invisible_meshes_collision_mask.unwrap_or(3) as i32;

                if let Some(mut gltf_node) = existing {
                    gltf_node.call_deferred(
                        StringName::from(GodotString::from("change_gltf")),
                        &[
                            Variant::from(GodotString::from(new_value.src)),
                            Variant::from(visible_meshes_collision_mask),
                            Variant::from(invisible_meshes_collision_mask),
                        ],
                    );
                    scene.gltf_loading.insert(*entity);
                } else {
                    let mut new_gltf = godot::engine::load::<PackedScene>(
                        "res://src/decentraland_components/gltf_container.tscn",
                    )
                    .instantiate()
                    .unwrap()
                    .cast::<DclGltfContainer>();

                    new_gltf
                        .bind_mut()
                        .set_dcl_gltf_src(GodotString::from(new_value.src));
                    new_gltf.bind_mut().set_dcl_scene_id(scene_id);
                    new_gltf.bind_mut().set_dcl_entity_id(entity.as_i32());
                    new_gltf
                        .bind_mut()
                        .set_dcl_visible_cmask(visible_meshes_collision_mask);
                    new_gltf
                        .bind_mut()
                        .set_dcl_invisible_cmask(invisible_meshes_collision_mask);

                    new_gltf.set_name(GodotString::from("GltfContainer"));
                    node_3d.add_child(new_gltf.clone().upcast());

                    scene.gltf_loading.insert(*entity);
                }
            }
        }
    }
}

pub fn sync_gltf_loading_state(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let gltf_container_loading_state_component =
        SceneCrdtStateProtoComponents::get_gltf_container_loading_state_mut(crdt_state);

    for entity in scene.gltf_loading.clone().iter() {
        let gltf_node = godot_dcl_scene
            .ensure_node_3d(entity)
            .1
            .try_get_node_as::<DclGltfContainer>(NodePath::from("GltfContainer"));

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
            gltf_container_loading_state_component.put(*entity, Some(crate::dcl::components::proto_components::sdk::components::PbGltfContainerLoadingState { current_state: current_state_godot.to_i32() }));
        }

        if current_state_godot == GltfContainerLoadingState::Finished
            || current_state_godot == GltfContainerLoadingState::FinishedWithError
            || current_state_godot == GltfContainerLoadingState::NotFound
        {
            scene.gltf_loading.remove(entity);
        }
    }
}
