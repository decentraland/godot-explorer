use crate::{
    dcl::{
        components::SceneComponentId,
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
        DirtyComponents,
    },
    scene_runner::godot_dcl_scene::GodotDclScene,
};
use godot::{
    engine::{node::InternalMode, packed_scene::GenEditState},
    prelude::*,
};

pub fn update_gltf_container(
    godot_dcl_scene: &mut GodotDclScene,
    crdt_state: &mut SceneCrdtState,
    dirty_components: &DirtyComponents,
) {
    let scene_id = godot_dcl_scene.scene_id.0;
    if let Some(gltf_container_dirty) = dirty_components.get(&SceneComponentId::GLTF_CONTAINER) {
        let gltf_container_component =
            SceneCrdtStateProtoComponents::get_gltf_container(crdt_state);

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
                if existing.is_some() {
                    // remove
                }
            } else if let Some(new_value) = new_value {
                if let Some(_existing) = existing {
                    // update
                } else {
                    // create
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

                    node.base.add_child(
                        new_gltf.share().upcast(),
                        false,
                        InternalMode::INTERNAL_MODE_DISABLED,
                    );

                    // scene.objs.push(new_mesh_instance_3d.share().upcast());
                }
            }
        }
    }
}
