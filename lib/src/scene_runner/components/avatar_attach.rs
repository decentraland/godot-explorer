use crate::{
    dcl::{
        components::SceneComponentId,
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    scene_runner::scene::Scene,
};
use godot::builtin::math::FloatExt;
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
                    let transform_component = crdt_state.get_transform();
                    let mut transform = transform_component
                        .get(entity)
                        .and_then(|entry| entry.value.clone())
                        .unwrap_or_default();
                    if !transform.rotation.is_normalized() {
                        if transform.rotation.length_squared() == 0.0 {
                            transform.rotation = godot::prelude::Quaternion::default();
                        } else {
                            transform.rotation = transform.rotation.normalized();
                        }
                    }
                    if !transform.rotation.is_finite() {
                        transform.rotation = godot::prelude::Quaternion::default();
                    }
                    node_3d.set_transform(transform.to_godot_transform_3d_without_scaled());
                    if transform.scale.x.is_zero_approx() {
                        transform.scale.x = 0.00001;
                    }
                    if transform.scale.y.is_zero_approx() {
                        transform.scale.y = 0.00001;
                    }
                    if transform.scale.z.is_zero_approx() {
                        transform.scale.z = 0.00001;
                    }
                    node_3d.set_scale(transform.scale);
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
