use godot::prelude::{Gd, Node};

use super::scene::Scene;

pub fn get_explorer_node(scene: &Scene) -> Gd<Node> {
    scene
        .godot_dcl_scene
        .root_node_3d
        .get_node_or_null("/root/explorer")
        .expect("Missing explorer node.")
}

pub fn get_dialog_stack_node(scene: &Scene) -> Gd<Node> {
    scene
        .godot_dcl_scene
        .root_node_3d
        .get_node_or_null("/root/explorer/UI/DialogStack/Stack")
        .expect("DialogStack not found")
        .cast::<Node>()
}

pub fn get_realm_node(scene: &Scene) -> Gd<Node> {
    scene
        .godot_dcl_scene
        .root_node_3d
        .get_node_or_null("/root/realm")
        .expect("Missing realm node")
}

pub fn get_avatar_node(scene: &Scene) -> Gd<Node> {
    scene
        .godot_dcl_scene
        .root_node_3d
        .get_node_or_null("/root/explorer/world/Player/Avatar")
        .expect("Missing Player Avatar Node")
}
