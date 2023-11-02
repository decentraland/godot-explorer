use crate::{
    common::rpc::{PortableLocation, RpcResultSender, SpawnResponse},
    dcl::SceneId,
    godot_classes::dcl_confirm_dialog::DclConfirmDialog,
    scene_runner::{
        global_get_node_helper::{
            get_avatar_node, get_dialog_stack_node, get_explorer_node, get_realm_node,
        },
        scene::{Scene, SceneType},
    },
};

use godot::prelude::{Gd, GodotString, Node, PackedScene, Variant, Vector2i, Vector3};
use tokio::runtime::Runtime;

pub fn spawn_portable(
    scene: &Scene,
    location: PortableLocation,
    response: RpcResultSender<Result<SpawnResponse, String>>,
) {
}

pub fn kill_portable(scene: &Scene, location: PortableLocation, response: RpcResultSender<bool>) {}

pub fn list_portables(response: RpcResultSender<Vec<SpawnResponse>>) {}
