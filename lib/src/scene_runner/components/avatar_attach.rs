use crate::{
    dcl::{
        components::SceneComponentId,
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    scene_runner::{
        components::transform_and_parent::apply_dcl_transform_to_node_3d, scene::Scene,
    },
};
use godot::prelude::*;

pub fn update_avatar_attach(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_lww_components = &scene.current_dirty.lww_components;
    let avatar_attach_component = SceneCrdtStateProtoComponents::get_avatar_attach(crdt_state);

    if let Some(avatar_attach_dirty) = dirty_lww_components.get(&SceneComponentId::AVATAR_ATTACH) {
        for entity in avatar_attach_dirty {
            let new_value = avatar_attach_component.get(entity);
            if new_value.is_none() {
                continue;
            }

            let new_value = new_value.unwrap();
            let (_godot_entity_node, mut node_3d) = godot_dcl_scene.ensure_node_3d(entity);

            let new_value = new_value.value.clone();
            let existing = node_3d.try_get_node_as::<Node>("AvatarAttach");

            if new_value.is_none() {
                if let Some(mut avatar_attach_node) = existing {
                    avatar_attach_node.queue_free();
                    node_3d.remove_child(&avatar_attach_node);
                    // Restore the entity's local transform from CRDT state so the
                    // node reverts to where the SDK Transform places it (matches
                    // Unity Renderer behavior — typically the scene origin if the
                    // scene only set AvatarAttach without an explicit Transform).
                    // Without this the node stays frozen at the last bone-aligned
                    // pose that avatar_attach.gd wrote.
                    let mut transform = crdt_state
                        .get_transform()
                        .get(entity)
                        .and_then(|entry| entry.value.clone())
                        .unwrap_or_default();
                    apply_dcl_transform_to_node_3d(&mut transform, &mut node_3d);
                }
            } else if let Some(new_value) = new_value {
                let (mut avatar_attach_node, is_new) = if let Some(avatar_attach_node) = existing {
                    (avatar_attach_node, false)
                } else {
                    let node = godot::tools::load::<PackedScene>(
                        "res://src/decentraland_components/avatar_attach.tscn",
                    )
                    .instantiate()
                    .unwrap();
                    (node, true)
                };

                avatar_attach_node.set(
                    "user_id",
                    &Variant::from(new_value.avatar_id.unwrap_or_default()),
                );

                avatar_attach_node.set("attach_point", &Variant::from(new_value.anchor_point_id));

                if is_new {
                    avatar_attach_node.set_name("AvatarAttach");
                    node_3d.add_child(&avatar_attach_node.clone());
                    avatar_attach_node.call("init", &[]);
                }
            }
        }
    }
}
