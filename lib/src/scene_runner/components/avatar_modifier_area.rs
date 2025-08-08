use crate::{
    dcl::{
        components::{proto_components, SceneComponentId},
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    godot_classes::dcl_avatar_modifier_area_3d::DclAvatarModifierArea3D,
    scene_runner::scene::Scene,
};
use godot::classes::{PackedScene, ResourceLoader};
use godot::prelude::*;

pub fn update_avatar_modifier_area(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_lww_components = &scene.current_dirty.lww_components;
    let avatar_modifier_area_component =
        SceneCrdtStateProtoComponents::get_avatar_modifier_area(crdt_state);

    if let Some(avatar_modifier_area_dirty) =
        dirty_lww_components.get(&SceneComponentId::AVATAR_MODIFIER_AREA)
    {
        for entity in avatar_modifier_area_dirty {
            let new_value = avatar_modifier_area_component.get(entity);

            let Some(new_value) = new_value else {
                continue; // no value, continue
            };

            let (_godot_entity_node, mut node_3d) = godot_dcl_scene.ensure_node_3d(entity);

            let new_value = new_value.value.clone();

            let existing = node_3d.try_get_node_as::<Node>(&NodePath::from("AvatarModifierArea"));

            if new_value.is_none() {
                if let Some(mut avatar_modifier_area_node) = existing {
                    avatar_modifier_area_node.queue_free();
                    node_3d.remove_child(&avatar_modifier_area_node);
                }
            } else if let Some(new_value) = new_value {
                let area = new_value
                    .area
                    .unwrap_or(proto_components::common::Vector3::default());
                let modifiers = new_value.modifiers.into_iter().collect();
                let exclude_ids = new_value
                    .exclude_ids
                    .into_iter()
                    .map(GString::from)
                    .collect();

                if let Some(avatar_modifier_area_node) = existing {
                    let mut avatar_modifier_area_3d =
                        avatar_modifier_area_node.cast::<DclAvatarModifierArea3D>();

                    avatar_modifier_area_3d
                        .bind_mut()
                        .set_area(Vector3::new(area.x, area.y, area.z));
                    avatar_modifier_area_3d
                        .bind_mut()
                        .set_avatar_modifiers(modifiers);
                    avatar_modifier_area_3d
                        .bind_mut()
                        .set_exclude_ids(exclude_ids);
                } else {
                    let mut avatar_modifier_area = ResourceLoader::singleton()
                        .load("res://src/decentraland_components/avatar_modifier_area.tscn")
                        .unwrap()
                        .cast::<PackedScene>()
                        .instantiate()
                        .unwrap()
                        .cast::<DclAvatarModifierArea3D>();

                    avatar_modifier_area
                        .bind_mut()
                        .set_area(Vector3::new(area.x, area.y, area.z));
                    avatar_modifier_area
                        .bind_mut()
                        .set_avatar_modifiers(modifiers);
                    avatar_modifier_area.bind_mut().set_exclude_ids(exclude_ids);
                    avatar_modifier_area.set_name(&GString::from("AvatarModifierArea"));
                    node_3d.add_child(&avatar_modifier_area.clone().upcast::<Node>());
                }
            }
        }
    }
}
