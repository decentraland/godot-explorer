use crate::{
    common::rpc::RpcResultSender,
    dcl::SceneId,
    godot_classes::dcl_confirm_dialog::DclConfirmDialog,
    scene_runner::{
        global_get_node_helper::{
            get_avatar_node, get_dialog_stack_node, get_explorer_node, get_realm_node,
        },
        scene::{Scene, SceneType},
    },
};

use godot::prelude::{GodotString, PackedScene, Variant, Vector2i, Vector3};

pub fn change_realm(
    scene: &Scene,
    to: &str,
    message: &Option<String>,
    response: &RpcResultSender<Result<(), String>>,
) {
    // Get nodes
    let mut dialog_stack = get_dialog_stack_node(scene);

    let mut realm_node = get_realm_node(scene);

    let confirm_dialog =
        godot::engine::load::<PackedScene>("res://src/ui/dialogs/confirm_dialog.tscn")
            .instantiate()
            .expect("ConfirmDialog instantiate error");

    // Setup confirm dialog
    dialog_stack.add_child(confirm_dialog.clone());

    // Setup confirm Dialog
    let mut confirm_dialog = confirm_dialog.cast::<DclConfirmDialog>();
    let mut confirm_dialog = confirm_dialog.bind_mut();

    let description = format!(
        "The scene wants to move you to a new realm\nTo: `{}`\nScene message: {}",
        to.clone(),
        if let Some(message) = message {
            message
        } else {
            ""
        }
    );

    // clone data that is going to the callback
    let to = GodotString::from(to);
    let response = response.clone();

    confirm_dialog.setup(
        "Change Realm",
        description.as_str(),
        "Let's go!",
        "No thanks",
        move |ok| {
            if ok {
                realm_node.call("set_realm".into(), &[Variant::from(to)]);
                response.send(Ok(()));
            } else {
                response.send(Err("User rejected to change realm".to_string()));
            }
        },
    );
}

fn _player_is_inside_scene(scene: &Scene, current_parcel_scene_id: &SceneId) -> bool {
    // Check if player is inside the scene that requested the move
    if let SceneType::Parcel = scene.scene_type {
        &scene.scene_id == current_parcel_scene_id
    } else {
        true
    }
}

// Allows to move a player inside the scene
pub fn move_player_to(
    scene: &Scene,
    current_parcel_scene_id: &SceneId,
    position_target: &[f32; 3],
    camera_target: &Option<[f32; 3]>,
    response: &RpcResultSender<Result<(), String>>,
) {
    // Check if player is inside the scene that requested the move
    if !_player_is_inside_scene(scene, current_parcel_scene_id) {
        response.send(Err("Player position is outside the scene".to_string()));
        return;
    }

    let mut explorer_node = get_explorer_node(scene);

    let base_parcel = scene.definition.base;
    let scene_position = Vector3::new(
        base_parcel.x as f32 * 16.0,
        0.0,
        base_parcel.y as f32 * 16.0,
    );

    // Calculate real target position
    let position_target = Vector3::new(position_target[0], position_target[1], position_target[2]);
    let position_target = position_target + scene_position;

    // Check if the target position is inside the scene that requested the move
    let target_parcel = Vector2i::new(
        (position_target.x / 16.0).floor() as i32,
        (position_target.z / 16.0).floor() as i32,
    );

    if !scene.definition.parcels.contains(&target_parcel) {
        response.send(Err("Target position is outside the scene".to_string()));
        return;
    }

    // Set player position
    let position_target = Vector3::new(position_target.x, position_target.y, -position_target.z);
    explorer_node.call("move_to".into(), &[Variant::from(position_target)]);

    // Set camera to look at camera target position
    if let Some(camera_target) = camera_target {
        let camera_target =
            Vector3::new(camera_target[0], camera_target[1], camera_target[2]) + scene_position;
        let camera_target = Vector3::new(camera_target.x, camera_target.y, -camera_target.z);

        explorer_node.call("player_look_at".into(), &[Variant::from(camera_target)]);
    }

    response.send(Ok(()));
}

// Teleport user to world coordinates
pub fn teleport_to(
    scene: &Scene,
    current_parcel_scene_id: &SceneId,
    world_coordinates: &[i32; 2],
    response: &RpcResultSender<Result<(), String>>,
) {
    // Check if player is inside the scene that requested the move
    if !_player_is_inside_scene(scene, current_parcel_scene_id) {
        response.send(Err("Player position is outside the scene".to_string()));
        return;
    }

    // Get Nodes
    let explorer_node = get_explorer_node(scene);

    let mut dialog_stack = get_dialog_stack_node(scene);

    // TODO: We should implement a new Dialog, that shows the thumbnail of the destination
    let confirm_dialog =
        godot::engine::load::<PackedScene>("res://src/ui/dialogs/confirm_dialog.tscn")
            .instantiate()
            .expect("ConfirmDialog instantiate error");

    dialog_stack.add_child(confirm_dialog.clone());

    // Setup confirm Dialog
    let mut confirm_dialog = confirm_dialog.cast::<DclConfirmDialog>();
    let mut confirm_dialog = confirm_dialog.bind_mut();

    let description = format!(
        "The scene wants to teleport you to {},{} position\n",
        world_coordinates[0], world_coordinates[1],
    );

    let target_parcel = Vector2i::new(world_coordinates[0], world_coordinates[1]);

    let response = response.clone();
    confirm_dialog.setup(
        "Teleport To",
        description.as_str(),
        "Let's go!",
        "No thanks",
        move |ok| {
            if ok {
                let mut explorer_node = explorer_node.clone();
                explorer_node.call("teleport_to".into(), &[Variant::from(target_parcel)]);

                response.send(Ok(()));
            } else {
                response.send(Err("User rejected to teleport".to_string()));
            }
        },
    );
}

pub fn trigger_emote(
    scene: &Scene,
    current_parcel_scene_id: &SceneId,
    emote_id: &str,
    response: &RpcResultSender<Result<(), String>>,
) {
    // Check if player is inside the scene that requested the move
    if !_player_is_inside_scene(scene, current_parcel_scene_id) {
        response.send(Err("Player position is outside the scene".to_string()));
        return;
    }

    let mut avatar_node = get_avatar_node(scene);
    avatar_node.call("play_emote".into(), &[Variant::from(emote_id)]);
    avatar_node.call("broadcast_avatar_animation".into(), &[]);
}

pub fn trigger_scene_emote(
    scene: &Scene,
    current_parcel_scene_id: &SceneId,
    emote_src: &str,
    looping: &bool,
    response: &RpcResultSender<Result<(), String>>,
) {
    // Check if player is inside the scene that requested the move
    if !_player_is_inside_scene(scene, current_parcel_scene_id) {
        response.send(Err("Player position is outside the scene".to_string()));
        return;
    }

    let mut avatar_node = get_avatar_node(scene);
    avatar_node.call(
        "play_remote_emote".into(),
        &[Variant::from(emote_src), Variant::from(*looping)],
    );
    avatar_node.call("broadcast_avatar_animation".into(), &[]);
}
