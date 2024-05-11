use crate::{
    avatars::avatar_type::DclAvatarWireFormat,
    comms::profile::{AvatarColor, AvatarColor3, AvatarEmote, AvatarWireFormat},
    dcl::{
        components::{proto_components::common::Color3, SceneComponentId},
        crdt::{
            grow_only_set::GenericGrowOnlySetComponentOperation,
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    scene_runner::scene::Scene,
};
use godot::prelude::*;

fn color3_to_avatar_color(color: Color3) -> AvatarColor {
    AvatarColor {
        color: AvatarColor3 {
            r: color.r,
            g: color.g,
            b: color.b,
        },
    }
}

pub fn update_avatar_shape(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_lww_components = &scene.current_dirty.lww_components;
    let avatar_shape_component = SceneCrdtStateProtoComponents::get_avatar_shape(crdt_state);

    if let Some(avatar_shape_dirty) = dirty_lww_components.get(&SceneComponentId::AVATAR_SHAPE) {
        for entity in avatar_shape_dirty {
            let new_value = avatar_shape_component.get(entity);
            if new_value.is_none() {
                continue;
            }

            let new_value = new_value.unwrap();
            let (_godot_entity_node, mut node_3d) = godot_dcl_scene.ensure_node_3d(entity);

            let new_value = new_value.value.clone();
            let existing = node_3d.try_get_node_as::<Node>(NodePath::from("AvatarShape"));

            if new_value.is_none() {
                if let Some(avatar_node) = existing {
                    node_3d.remove_child(avatar_node);
                }
            } else if let Some(new_value) = new_value {
                let avatar_name = new_value.name.unwrap_or("NPC".into());

                let mut new_avatar_data = AvatarWireFormat {
                    wearables: new_value.wearables,
                    emotes: Some(
                        new_value
                            .emotes
                            .iter()
                            .enumerate()
                            .map(|(index, urn)| AvatarEmote {
                                slot: index as u32,
                                urn: urn.clone(),
                            })
                            .collect(),
                    ),
                    ..Default::default()
                };

                if let Some(body_shape) = new_value.body_shape {
                    new_avatar_data.body_shape = Some(body_shape);
                }
                if let Some(eye_color) = new_value.eye_color {
                    new_avatar_data.eyes = Some(color3_to_avatar_color(eye_color));
                }
                if let Some(hair_color) = new_value.hair_color {
                    new_avatar_data.hair = Some(color3_to_avatar_color(hair_color));
                }
                if let Some(skin_color) = new_value.skin_color {
                    new_avatar_data.skin = Some(color3_to_avatar_color(skin_color));
                }

                let new_avatar_data = DclAvatarWireFormat::from_gd(new_avatar_data);

                if let Some(mut avatar_node) = existing {
                    avatar_node.call_deferred(
                        "async_update_avatar".into(),
                        &[new_avatar_data.to_variant(), avatar_name.to_variant()],
                    );
                } else {
                    let mut new_avatar_shape = godot::engine::load::<PackedScene>(
                        "res://src/decentraland_components/avatar/avatar.tscn",
                    )
                    .instantiate()
                    .unwrap();

                    new_avatar_shape.set("skip_process".into(), true.to_variant());
                    new_avatar_shape.set_name(GString::from("AvatarShape"));
                    node_3d.add_child(new_avatar_shape.clone().upcast());

                    new_avatar_shape.call_deferred(
                        "async_update_avatar".into(),
                        &[new_avatar_data.to_variant(), avatar_name.to_variant()],
                    );
                }
            }
        }
    }
}

pub fn update_avatar_shape_emote_command(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_gos_components = &scene.current_dirty.gos_components;
    let avatar_shape_component =
        SceneCrdtStateProtoComponents::get_avatar_emote_command(crdt_state);

    if let Some(avatar_emote_command_dirty) =
        dirty_gos_components.get(&SceneComponentId::AVATAR_EMOTE_COMMAND)
    {
        for entity in avatar_emote_command_dirty.keys() {
            let Some(emotes) = avatar_shape_component.get(entity) else {
                continue;
            };
            if emotes.is_empty() {
                continue;
            }
            let Some(node_3d) = godot_dcl_scene.get_node_3d(entity) else {
                continue;
            };

            let Some(mut avatar_node) =
                node_3d.try_get_node_as::<Node>(NodePath::from("AvatarShape"))
            else {
                continue;
            };

            let emote = emotes
                .back()
                .expect("emotes should have at least one element");

            let local_emote = emote.emote_urn.contains(".glb") || emote.emote_urn.contains(".gltf");
            let urn = if local_emote {
                let Some(file_hash) = scene.content_mapping.get_hash(&emote.emote_urn) else {
                    continue;
                };

                format!(
                    "urn:decentraland:off-chain:scene-emote:{file_hash}-{}",
                    emote.r#loop
                )
            } else {
                emote.emote_urn.clone()
            };

            avatar_node.call_deferred("async_play_emote".into(), &[urn.to_variant()]);
        }
    }
}
