use std::{cell::RefCell, rc::Rc};

use crate::{
    common::rpc::RpcResultSender,
    dcl::{
        components::SceneEntityId,
        crdt::{last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState},
    },
    godot_classes::dcl_confirm_dialog::DclConfirmDialog,
    scene_runner::scene::Scene,
};

use godot::prelude::{GodotString, Variant, Vector2i, Vector3};

pub fn change_realm(
    scene: &Scene,
    to: &String,
    message: &Option<String>,
    response: &RpcResultSender<Result<(), String>>,
) {
    if let Some(confirm_dialog) = scene
        .godot_dcl_scene
        .root_node_3d
        .get_node("/root/explorer/UI/ConfirmDialog".into())
    {
        let mut confirm_dialog = confirm_dialog.cast::<DclConfirmDialog>();

        // Show node :)
        confirm_dialog.show();

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

        confirm_dialog.set_texts(
            "Change Realm",
            description.as_str(),
            "Let's go!",
            "No thanks",
        );


        let mut realm_node = scene
            .godot_dcl_scene
            .root_node_3d
            .get_node("/root/realm".into())
            .expect("Missing realm node");

        // clone data that is going to the callback
        let to = to.clone();

        let response = response.clone();
        confirm_dialog.set_confirm_callback(move |ok| {
            if ok {
                realm_node.call(
                    "set_realm".into(),
                    &[Variant::from(GodotString::from(to.clone()))],
                );
                response.send(Ok(()));
            } else {
                response.send(Err("User rejected to change realm".to_string()));
            }
        });
    } else {
        println!("Error: ConfirmDialog not found");
        response.send(Err("EngineError: ConfirmDialog not found".to_string()));
    }
}

fn _player_is_inside_scene(scene: &Scene, player_position: &Vector3) -> bool
{
    // Check if player is inside the scene that requested the move
    let player_parcel = Vector2i::new(
        (player_position.x / 16.0).floor() as i32,
        (player_position.z / 16.0).floor() as i32,
    );

    scene.definition.parcels.contains(&player_parcel)
}

// Allows to move a player inside the scene
pub fn move_player_to(
    scene: &Scene,
    crdt_state: &SceneCrdtState,
    position_target: &[f32; 3],
    camera_target: &Option<[f32; 3]>,
    response: &RpcResultSender<Result<(), String>>,
) {
    let mut explorer_node = scene
        .godot_dcl_scene
        .root_node_3d
        .get_node("/root/explorer".into())
        .expect("Missing explorer node.");

    let base_parcel = scene.definition.base;
    let scene_position = Vector3::new(
        base_parcel.x as f32 * 16.0,
        0.0,
        -base_parcel.y as f32 * 16.0,
    );

    // Calculate real target position
    let position_target = Vector3::new(position_target[0], position_target[1], -position_target[2]);

    let position_target = position_target - scene_position;

    // Check if player is inside the scene that requested the move
    let player_translation = crdt_state
        .get_transform()
        .get(SceneEntityId::PLAYER)
        .unwrap()
        .value
        .as_ref()
        .unwrap()
        .translation;

    if !_player_is_inside_scene(&scene, &player_translation) {
        response.send(Err("Player position is outside the scene".to_string()));
        return;
    }

    // Check if the target position is inside the scene that requested the move
    let target_parcel = Vector2i::new(
        (position_target.x / 16.0).floor() as i32,
        (position_target.y / 16.0).floor() as i32,
    );

    if !scene.definition.parcels.contains(&target_parcel) {
        response.send(Err("Target position is outside the scene".to_string()));
        return;
    }

    // Set player position
    explorer_node.call("move_to".into(), &[Variant::from(position_target)]);

    // Set camera to look at camera target position
    if let Some(camera_target) = camera_target {
        let camera_target = Vector3::new(camera_target[0], camera_target[1], -camera_target[2]);

        let camera_target = camera_target - scene_position;

        explorer_node.call("player_look_at".into(), &[Variant::from(camera_target)]);
    }

    response.send(Ok(()));
}

// Teleport user to world coordinates
pub fn teleport_to(
    scene: &Scene,
    crdt_state: &SceneCrdtState,
    world_coordinates: &[i32; 2],
    response: &RpcResultSender<Result<(), String>>,
) {
    // Check if player is inside the scene that requested the move
    let player_translation = crdt_state
        .get_transform()
        .get(SceneEntityId::PLAYER)
        .unwrap()
        .value
        .as_ref()
        .unwrap()
        .translation;

    if !_player_is_inside_scene(&scene, &player_translation) {
        response.send(Err("Player position is outside the scene".to_string()));
        return;
    }

    let mut confirm_dialog = scene
        .godot_dcl_scene
        .root_node_3d
        .get_node("/root/explorer/UI/ConfirmDialog".into())
        .expect("ConfirmDialog not found")
        .cast::<DclConfirmDialog>();

    // Show node :)
    confirm_dialog.show();

    let mut confirm_dialog = confirm_dialog.bind_mut();

    let description = format!(
        "The scene wants to teleport you to {},{} position\n",
        world_coordinates[0], world_coordinates[1],
    );

    confirm_dialog.set_texts(
        "Teleport To",
        description.as_str(),
        "Let's go!",
        "No thanks",
    );

    let target_parcel = Vector2i::new(world_coordinates[0], world_coordinates[1]);

    let explorer_node = scene
        .godot_dcl_scene
        .root_node_3d
        .get_node("/root/explorer".into())
        .expect("Missing explorer node.");

    let response = response.clone();
    confirm_dialog.set_confirm_callback(move |ok| {
        if ok {
            let mut explorer_node = explorer_node.clone();
            explorer_node.call("teleport_to".into(), &[Variant::from(target_parcel)]);

            response.send(Ok(()));
        } else {
            response.send(Err("User rejected to teleport".to_string()));
        }
    });
}
