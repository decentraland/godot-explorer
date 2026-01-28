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
use godot::{classes::Node, prelude::*};

#[allow(dead_code)]
trait ToDictionaryColorObject {
    fn to_dictionary_color_object(&self) -> VarDictionary;
}

impl ToDictionaryColorObject for crate::dcl::components::proto_components::common::Color3 {
    fn to_dictionary_color_object(&self) -> VarDictionary {
        let mut dictionary = VarDictionary::new();
        dictionary.set("r", self.r);
        dictionary.set("g", self.g);
        dictionary.set("b", self.b);
        dictionary.set("a", 1.0);
        let mut ret_dictionary = VarDictionary::new();
        ret_dictionary.set("color", dictionary);
        ret_dictionary
    }
}
impl ToDictionaryColorObject for crate::dcl::components::proto_components::common::Color4 {
    fn to_dictionary_color_object(&self) -> VarDictionary {
        let mut dictionary = VarDictionary::new();
        dictionary.set("r", self.r);
        dictionary.set("g", self.g);
        dictionary.set("b", self.b);
        dictionary.set("a", self.a);
        let mut ret_dictionary = VarDictionary::new();
        ret_dictionary.set("color", dictionary);
        ret_dictionary
    }
}

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
            let existing = node_3d.try_get_node_as::<Node>("AvatarShape");

            if new_value.is_none() {
                if let Some(mut avatar_node) = existing {
                    avatar_node.queue_free();
                    node_3d.remove_child(&avatar_node);
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
                    force_render: None,
                    show_only_wearables: new_value.show_only_wearables.unwrap_or(false),
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

                // Build AvatarShape-specific config dictionary
                let mut avatar_shape_config = VarDictionary::new();
                avatar_shape_config.set("is_avatar_shape", true);
                if let Some(expression_trigger_id) = &new_value.expression_trigger_id {
                    // Check if this is a scene emote (GLB/GLTF path)
                    let is_scene_emote = expression_trigger_id.contains(".glb")
                        || expression_trigger_id.contains(".gltf");

                    if is_scene_emote {
                        // Resolve the GLB path to hash and build a scene-emote URN
                        if let Some(emote_hash) = scene
                            .content_mapping
                            .get_scene_emote_hash(expression_trigger_id)
                        {
                            // Build URN: urn:decentraland:off-chain:scene-emote:{sceneId}-{glbHash}-{loop}
                            let scene_emote_urn = format!(
                                "urn:decentraland:off-chain:scene-emote:{}-{}-false",
                                scene.scene_entity_definition.id, emote_hash.glb_hash
                            );
                            tracing::debug!(
                                "AvatarShape expression_trigger: scene emote '{}' -> {}",
                                expression_trigger_id,
                                scene_emote_urn
                            );
                            avatar_shape_config.set("expression_trigger_id", scene_emote_urn);
                        } else {
                            tracing::warn!(
                                "AvatarShape expression_trigger: scene emote '{}' not found in content mapping",
                                expression_trigger_id
                            );
                        }
                    } else {
                        // URN or default emote - pass as-is
                        avatar_shape_config
                            .set("expression_trigger_id", expression_trigger_id.clone());
                    }
                }
                if let Some(expression_trigger_timestamp) = new_value.expression_trigger_timestamp {
                    avatar_shape_config
                        .set("expression_trigger_timestamp", expression_trigger_timestamp);
                }

                if let Some(mut avatar_node) = existing {
                    avatar_node.call_deferred(
                        "async_update_avatar",
                        &[
                            new_avatar_data.to_variant(),
                            avatar_name.to_variant(),
                            avatar_shape_config.to_variant(),
                        ],
                    );
                } else {
                    let mut new_avatar_shape = godot::tools::load::<PackedScene>(
                        "res://src/decentraland_components/avatar/avatar.tscn",
                    )
                    .instantiate()
                    .unwrap();

                    new_avatar_shape.set("skip_process", &true.to_variant());
                    new_avatar_shape.set_name("AvatarShape");
                    node_3d.add_child(&new_avatar_shape.clone().upcast::<Node>());

                    // Remove trigger detection for AvatarShapes - scene NPCs should not trigger areas
                    new_avatar_shape.call("remove_trigger_detection", &[]);

                    new_avatar_shape.call_deferred(
                        "async_update_avatar",
                        &[
                            new_avatar_data.to_variant(),
                            avatar_name.to_variant(),
                            avatar_shape_config.to_variant(),
                        ],
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
            let Some(node_3d) = godot_dcl_scene.get_node_or_null_3d(entity) else {
                continue;
            };

            let Some(mut avatar_node) = node_3d.try_get_node_as::<Node>("AvatarShape") else {
                continue;
            };

            let emote = emotes
                .back()
                .expect("emotes should have at least one element");

            let local_emote = emote.emote_urn.contains(".glb") || emote.emote_urn.contains(".gltf");
            tracing::debug!(
                "AvatarEmoteCommand: emote_urn={}, loop={}, is_local={}",
                emote.emote_urn,
                emote.r#loop,
                local_emote
            );

            if local_emote {
                let Some(emote_hash) = scene.content_mapping.get_scene_emote_hash(&emote.emote_urn)
                else {
                    tracing::warn!(
                        "AvatarEmoteCommand: scene emote '{}' not found in content mapping",
                        emote.emote_urn
                    );
                    continue;
                };
                tracing::debug!(
                    "AvatarEmoteCommand: playing scene emote glb_hash={}, audio_hash={:?}",
                    emote_hash.glb_hash,
                    emote_hash.audio_hash
                );
                let emote_data = emote_hash.to_godot_data(emote.r#loop);
                avatar_node.call_deferred("async_play_scene_emote", &[emote_data.to_variant()]);
            } else {
                tracing::debug!(
                    "AvatarEmoteCommand: playing wearable emote urn={}",
                    emote.emote_urn
                );
                avatar_node.call_deferred("async_play_emote", &[emote.emote_urn.to_variant()]);
            }
        }
    }
}
