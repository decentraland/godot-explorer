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
use godot::prelude::*;

trait ToDictionaryColorObject {
    fn to_dictionary_color_object(&self) -> Dictionary;
}

impl ToDictionaryColorObject for crate::dcl::components::proto_components::common::Color3 {
    fn to_dictionary_color_object(&self) -> Dictionary {
        let mut dictionary = Dictionary::new();
        dictionary.set("r", self.r);
        dictionary.set("g", self.g);
        dictionary.set("b", self.b);
        dictionary.set("a", 1.0);
        let mut ret_dictionary = Dictionary::new();
        ret_dictionary.set("color", dictionary);
        ret_dictionary
    }
}
impl ToDictionaryColorObject for crate::dcl::components::proto_components::common::Color4 {
    fn to_dictionary_color_object(&self) -> Dictionary {
        let mut dictionary = Dictionary::new();
        dictionary.set("r", self.r);
        dictionary.set("g", self.g);
        dictionary.set("b", self.b);
        dictionary.set("a", self.a);
        let mut ret_dictionary = Dictionary::new();
        ret_dictionary.set("color", dictionary);
        ret_dictionary
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
                // TODO: make dictionary from PbAvatarShape as SerializedProfile
                let mut dictionary = Dictionary::new();
                let eyes = new_value.eye_color.as_ref().unwrap_or(
                    &crate::dcl::components::proto_components::common::Color3 {
                        r: 0.6,
                        g: 0.462,
                        b: 0.356,
                    },
                );
                let hair = new_value.hair_color.as_ref().unwrap_or(
                    &crate::dcl::components::proto_components::common::Color3 {
                        r: 0.283,
                        g: 0.142,
                        b: 0.0,
                    },
                );
                let skin = new_value.skin_color.as_ref().unwrap_or(
                    &crate::dcl::components::proto_components::common::Color3 {
                        r: 0.6,
                        g: 0.462,
                        b: 0.356,
                    },
                );
                dictionary.set(
                    "name",
                    GString::from(new_value.name.as_ref().unwrap_or(&"NPC".to_string())),
                );
                dictionary.set(
                    "bodyShape",
                    GString::from(new_value.body_shape.as_ref().unwrap_or(
                        &"urn:decentraland:off-chain:base-avatars:BaseFemale".to_string(),
                    )),
                );
                dictionary.set("eyes", eyes.to_dictionary_color_object());
                dictionary.set("hair", hair.to_dictionary_color_object());
                dictionary.set("skin", skin.to_dictionary_color_object());

                let wearables = {
                    if new_value.wearables.is_empty() {
                        vec![
                            "urn:decentraland:off-chain:base-avatars:f_eyes_00".to_string(),
                            "urn:decentraland:off-chain:base-avatars:f_eyebrows_00".to_string(),
                            "urn:decentraland:off-chain:base-avatars:f_mouth_00".to_string(),
                            "urn:decentraland:off-chain:base-avatars:standard_hair".to_string(),
                            "urn:decentraland:off-chain:base-avatars:f_simple_yellow_tshirt"
                                .to_string(),
                            "urn:decentraland:off-chain:base-avatars:f_brown_trousers".to_string(),
                            "urn:decentraland:off-chain:base-avatars:bun_shoes".to_string(),
                        ]
                    } else {
                        new_value.wearables
                    }
                };

                dictionary.set(
                    "wearables",
                    wearables
                        .iter()
                        .map(GString::from)
                        .collect::<Array<GString>>()
                        .to_variant(),
                );

                // dictionary.set("emotes", emotes);

                if let Some(mut avatar_node) = existing {
                    avatar_node.call_deferred(
                        StringName::from(GString::from("async_update_avatar")),
                        &[dictionary.to_variant()],
                    );
                } else {
                    let mut new_avatar_shape = godot::engine::load::<PackedScene>(
                        "res://src/decentraland_components/avatar/avatar.tscn",
                    )
                    .instantiate()
                    .unwrap();

                    new_avatar_shape.set(StringName::from("skip_process"), Variant::from(true));

                    new_avatar_shape.set_name(GString::from("AvatarShape"));
                    node_3d.add_child(new_avatar_shape.clone().upcast());

                    new_avatar_shape.call_deferred(
                        StringName::from(GString::from("async_update_avatar")),
                        &[dictionary.to_variant()],
                    );
                }
            }
        }
    }
}
