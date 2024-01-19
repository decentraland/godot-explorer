use godot::{
    engine::PanelContainer,
    prelude::{Gd, Node},
};

use super::scene::Scene;

pub fn get_explorer_node(scene: &Scene) -> Gd<Node> {
    scene
        .godot_dcl_scene
        .root_node_3d
        .get_node("/root/explorer".into())
        .expect("Missing explorer node.")
}

pub fn get_dialog_stack_node(scene: &Scene) -> Gd<PanelContainer> {
    scene
        .godot_dcl_scene
        .root_node_3d
        .get_node("/root/explorer/UI/DialogStack".into())
        .expect("DialogStack not found")
        .cast::<PanelContainer>()
}

pub fn get_realm_node(scene: &Scene) -> Gd<Node> {
    scene
        .godot_dcl_scene
        .root_node_3d
        .get_node("/root/realm".into())
        .expect("Missing realm node")
}

pub fn get_avatar_node(scene: &Scene) -> Gd<Node> {
    scene
        .godot_dcl_scene
        .root_node_3d
        .get_node("/root/explorer/world/Player/Avatar".into())
        .expect("Missing Player Avatar Node")
}
