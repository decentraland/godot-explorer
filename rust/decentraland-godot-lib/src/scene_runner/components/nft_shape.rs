use crate::{
    dcl::{
        components::{proto_components::common::Color3, SceneComponentId},
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    scene_runner::scene::Scene,
};
use godot::prelude::*;

pub fn update_nft_shape(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_lww_components = &scene.current_dirty.lww_components;
    let nft_shape_component = SceneCrdtStateProtoComponents::get_nft_shape(crdt_state);

    if let Some(nft_shape_dirty) = dirty_lww_components.get(&SceneComponentId::NFT_SHAPE) {
        for entity in nft_shape_dirty {
            let new_value = nft_shape_component.get(entity);

            let Some(new_value) = new_value else {
                continue; // no value, continue
            };

            let (_godot_entity_node, mut node_3d) = godot_dcl_scene.ensure_node_3d(entity);

            let new_value = new_value.value.clone();

            let existing = node_3d.try_get_node_as::<Node>(NodePath::from("NFTShape"));

            if new_value.is_none() {
                if let Some(nft_shape_node) = existing {
                    node_3d.remove_child(nft_shape_node);
                }
            } else if let Some(new_value) = new_value {
                let urn = new_value.urn.to_godot();
                let style = new_value.style.unwrap_or(0); // default NFT_CLASSIC=0
                let color = new_value.color.unwrap_or(Color3 {
                    r: 0.6404918,
                    g: 0.611472,
                    b: 0.8584906,
                });
                let color = color.to_godot();

                let mut nft_shape_3d = if let Some(nft_shape_3d) = existing {
                    nft_shape_3d
                } else {
                    let mut nft_shape_3d = godot::engine::load::<PackedScene>(
                        "res://src/decentraland_components/nft_shape.tscn",
                    )
                    .instantiate()
                    .unwrap();

                    nft_shape_3d.set_name(GodotString::from("NFTShape"));
                    node_3d.add_child(nft_shape_3d.clone().upcast());
                    nft_shape_3d
                };

                nft_shape_3d.call(
                    "co_load_nft".into(),
                    &[urn.to_variant(), style.to_variant(), color.to_variant()],
                );
            }
        }
    }
}
