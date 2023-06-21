use crate::{
    dcl::{
        components::SceneComponentId,
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    scene_runner::scene_manager::Scene,
};
use godot::{
    engine::{node::InternalMode, packed_scene::GenEditState},
    prelude::*,
};

// see gltf_container.gd
enum GodotGltfState {
    Unknown = 0,
    #[allow(dead_code)]
    Loading = 1,
    NotFound = 2,
    FinishedWithError = 3,
    Finished = 4,
}

pub fn update_gltf_container(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_lww_components = &scene.current_dirty.lww_components;
    let scene_id = godot_dcl_scene.scene_id.0;
    let gltf_container_component = SceneCrdtStateProtoComponents::get_gltf_container(crdt_state);

    if let Some(gltf_container_dirty) = dirty_lww_components.get(&SceneComponentId::GLTF_CONTAINER)
    {
        for entity in gltf_container_dirty {
            let new_value = gltf_container_component.get(*entity);
            if new_value.is_none() {
                continue;
            }

            let new_value = new_value.unwrap();
            let node = godot_dcl_scene.ensure_node_mut(entity);

            let new_value = new_value.value.clone();
            let existing = node
                .base
                .try_get_node_as::<Node>(NodePath::from("GltfContainer"));

            if new_value.is_none() {
                if let Some(gltf_node) = existing {
                    node.base.remove_child(gltf_node);
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
                    .instantiate(GenEditState::GEN_EDIT_STATE_DISABLED)
                    .unwrap();

                    new_gltf.set(
                        StringName::from("dcl_gltf_src"),
                        Variant::from(GodotString::from(new_value.src)),
                    );

                    new_gltf.set(StringName::from("dcl_scene_id"), Variant::from(scene_id));
                    new_gltf.set(
                        StringName::from("dcl_entity_id"),
                        Variant::from(entity.as_usize() as i32),
                    );
                    new_gltf.set(
                        StringName::from("dcl_visible_cmask"),
                        Variant::from(visible_meshes_collision_mask),
                    );
                    new_gltf.set(
                        StringName::from("dcl_invisible_cmask"),
                        Variant::from(invisible_meshes_collision_mask),
                    );
                    new_gltf.set_name(GodotString::from("GltfContainer"));
                    node.base.add_child(
                        new_gltf.share().upcast(),
                        false,
                        InternalMode::INTERNAL_MODE_DISABLED,
                    );

                    scene.gltf_loading.insert(*entity);
                }
            }
        }
    }

    let gltf_container_loading_state_component =
        SceneCrdtStateProtoComponents::get_gltf_container_loading_state_mut(crdt_state);

    for entity in scene.gltf_loading.clone().iter() {
        let gltf_node = godot_dcl_scene
            .ensure_node_mut(entity)
            .base
            .try_get_node_as::<Node>(NodePath::from("GltfContainer"));

        let current_state = match gltf_container_loading_state_component.get(*entity) {
            Some(state) => match state.value.as_ref() {
                Some(value) => value.current_state,
                _ => GodotGltfState::Unknown as i32,
            },
            None => GodotGltfState::Unknown as i32,
        };

        let current_state_godot = match gltf_node {
            Some(gltf_node) => {
                let gltf_state = gltf_node.get(StringName::from(GodotString::from("gltf_state")));
                let gltf_state = i32::try_from_variant(&gltf_state);
                match gltf_state {
                    Ok(gltf_state) => gltf_state,
                    Err(err) => {
                        godot_print!("Error getting gltf_state: {:?}", err);
                        GodotGltfState::Unknown as i32
                    }
                }
            }
            None => GodotGltfState::Unknown as i32,
        };

        if current_state_godot != current_state {
            gltf_container_loading_state_component.put(*entity, Some(crate::dcl::components::proto_components::sdk::components::PbGltfContainerLoadingState { current_state: current_state_godot }));

            if current_state_godot == GodotGltfState::Finished as i32
                || current_state_godot == GodotGltfState::FinishedWithError as i32
                || current_state_godot == GodotGltfState::NotFound as i32
            {
                scene.gltf_loading.remove(entity);
            }
        }
    }
}
