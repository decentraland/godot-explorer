use crate::{
    dcl::{scene_apis::RpcResultSender, SceneId},
    godot_classes::dcl_global::DclGlobal,
    scene_runner::{
        global_get_node_helper::{get_avatar_node, get_dialog_stack_node, get_explorer_node},
        scene::{Scene, SceneType},
    },
};

use godot::{
    meta::ToGodot,
    obj::Singleton,
    prelude::{PackedScene, Variant, Vector2i, Vector3},
};
use http::Uri;

fn _player_is_inside_scene(scene: &Scene, current_parcel_scene_id: &SceneId) -> bool {
    // Check if player is inside the scene that requested the move
    if let SceneType::Parcel = scene.scene_type {
        &scene.scene_id == current_parcel_scene_id
    } else {
        true
    }
}

pub fn change_realm(
    scene: &Scene,
    current_parcel_scene_id: &SceneId,
    to: &str,
    message: &Option<String>,
    response: &RpcResultSender<Result<(), String>>,
) {
    // Check if player is inside the scene that requested the move
    if !_player_is_inside_scene(scene, current_parcel_scene_id) {
        response.send(Err("Primary Player is outside the scene".to_string()));
        return;
    }

    // Get Global node from scene tree (Global is an autoload, not an Engine singleton)
    let Some(tree) = godot::classes::Engine::singleton().get_main_loop() else {
        tracing::error!("Cannot get main loop");
        response.send(Err("modal_manager not available".to_string()));
        return;
    };

    let Some(root) = tree.cast::<godot::classes::SceneTree>().get_root() else {
        tracing::error!("Cannot get root node");
        response.send(Err("modal_manager not available".to_string()));
        return;
    };

    let Some(global) = root.get_node_or_null("/root/Global") else {
        tracing::error!("Cannot get Global node from scene tree");
        response.send(Err("modal_manager not available".to_string()));
        return;
    };

    let modal_manager_variant = global.get("modal_manager");
    let Some(modal_manager) = modal_manager_variant
        .try_to::<godot::prelude::Gd<godot::classes::Node>>()
        .ok()
    else {
        tracing::error!("Cannot convert modal_manager variant to Node");
        response.send(Err("modal_manager not available".to_string()));
        return;
    };

    let mut modal_manager = modal_manager;
    let realm_name = to.to_godot();
    let scene_message = message.clone().unwrap_or_default().to_godot();

    modal_manager.call(
        "async_show_change_realm_modal",
        &[realm_name.to_variant(), scene_message.to_variant()],
    );

    // Send Ok immediately - the modal will handle the actual realm change
    // This matches the behavior where the RPC call succeeds once the modal is shown
    response.send(Ok(()));
}

pub fn open_external_url(
    scene: &Scene,
    current_parcel_scene_id: &SceneId,
    url: &Uri,
    response: &RpcResultSender<Result<(), String>>,
) {
    // Check if player is inside the scene that requested the move
    if !_player_is_inside_scene(scene, current_parcel_scene_id) {
        response.send(Err("Primary Player is outside the scene".to_string()));
        return;
    }

    // Get Global node from scene tree (Global is an autoload, not an Engine singleton)
    let Some(tree) = godot::classes::Engine::singleton().get_main_loop() else {
        tracing::error!("Cannot get main loop");
        response.send(Err("modal_manager not available".to_string()));
        return;
    };

    let Some(root) = tree.cast::<godot::classes::SceneTree>().get_root() else {
        tracing::error!("Cannot get root node");
        response.send(Err("modal_manager not available".to_string()));
        return;
    };

    let Some(global) = root.get_node_or_null("/root/Global") else {
        tracing::error!("Cannot get Global node from scene tree");
        response.send(Err("modal_manager not available".to_string()));
        return;
    };

    let modal_manager_variant = global.get("modal_manager");
    let Some(modal_manager) = modal_manager_variant
        .try_to::<godot::prelude::Gd<godot::classes::Node>>()
        .ok()
    else {
        tracing::error!("Cannot convert modal_manager variant to Node");
        response.send(Err("modal_manager not available".to_string()));
        return;
    };

    let mut modal_manager = modal_manager;
    let godot_url = url.to_string().to_godot();

    modal_manager.call("async_show_external_link_modal", &[godot_url.to_variant()]);

    // Send Ok immediately - the modal will handle the actual URL opening
    // This matches the behavior where the RPC call succeeds once the modal is shown
    response.send(Ok(()));
}

pub fn open_nft_dialog(
    scene: &Scene,
    current_parcel_scene_id: &SceneId,
    urn: &str,
    response: &RpcResultSender<Result<(), String>>,
) {
    // Check if player is inside the scene that requested the move
    if !_player_is_inside_scene(scene, current_parcel_scene_id) {
        response.send(Err("Primary Player is outside the scene".to_string()));
        return;
    }

    // Get nodes
    let mut dialog_stack = get_dialog_stack_node(scene);

    let mut confirm_dialog =
        godot::tools::load::<PackedScene>("res://src/ui/dialogs/nft_dialog.tscn")
            .instantiate()
            .expect("NftDialog instantiate error");

    // Setup confirm dialog
    dialog_stack.add_child(&confirm_dialog.clone());

    confirm_dialog.call("async_load_nft", &[urn.to_variant()]);

    response.send(Ok(()));
}

// Allows to move a player inside the scene
pub fn move_player_to(
    scene: &Scene,
    current_parcel_scene_id: &SceneId,
    position_target: &[f32; 3],
    camera_target: &Option<[f32; 3]>,
    avatar_target: &Option<[f32; 3]>,
) {
    // Check if player is inside the scene that requested the move
    if !_player_is_inside_scene(scene, current_parcel_scene_id) {
        tracing::warn!("movePlayerTo failed: Primary Player is outside the scene");
        return;
    }

    let mut explorer_node = get_explorer_node(scene);

    let base_parcel = scene.scene_entity_definition.scene_meta_scene.scene.base;
    let scene_position = Vector3::new(
        base_parcel.x as f32 * 16.0,
        0.0,
        base_parcel.y as f32 * 16.0,
    );

    // Calculate real target position
    let relative_position_target =
        Vector3::new(position_target[0], position_target[1], position_target[2]);
    let position_target = relative_position_target + scene_position;
    tracing::debug!("move_player_to: relative_position_target={relative_position_target} + scene_position={scene_position} = position_target={position_target}");

    // Check if the target position is inside the scene that requested the move
    let target_parcel = Vector2i::new(
        (position_target.x / 16.0).floor() as i32,
        (position_target.z / 16.0).floor() as i32,
    );

    if !scene
        .scene_entity_definition
        .scene_meta_scene
        .scene
        .parcels
        .contains(&target_parcel)
    {
        tracing::warn!("movePlayerTo failed: Target position is outside the scene");
        return;
    }

    // Set player position
    let position_target = Vector3::new(position_target.x, position_target.y, -position_target.z);
    explorer_node.call(
        "move_to",
        &[Variant::from(position_target), true.to_variant()],
    );

    // Handle avatar and camera targeting according to ADR-257:
    // - avatarTarget: where the avatar body looks
    // - cameraTarget: where the camera looks
    // If only cameraTarget is set (backward compatibility), it affects both avatar and camera
    match (avatar_target, camera_target) {
        (Some(avatar), Some(camera)) => {
            // Both targets provided: independent control
            // Call camera_look_at first (sets player body and camera to face camera target)
            // Then avatar_look_at_independent (sets avatar to face avatar target relative to player)
            let camera_pos = Vector3::new(camera[0], camera[1], camera[2]) + scene_position;
            let camera_pos = Vector3::new(camera_pos.x, camera_pos.y, -camera_pos.z);
            explorer_node.call("camera_look_at", &[Variant::from(camera_pos)]);

            let avatar_pos = Vector3::new(avatar[0], avatar[1], avatar[2]) + scene_position;
            let avatar_pos = Vector3::new(avatar_pos.x, avatar_pos.y, -avatar_pos.z);
            explorer_node.call("avatar_look_at_independent", &[Variant::from(avatar_pos)]);
        }
        (Some(avatar), None) => {
            // Only avatar target: avatar looks at it
            let target_pos = Vector3::new(avatar[0], avatar[1], avatar[2]) + scene_position;
            let target_pos = Vector3::new(target_pos.x, target_pos.y, -target_pos.z);
            explorer_node.call("player_look_at", &[Variant::from(target_pos)]);
        }
        (None, Some(camera)) => {
            // Only camera target: backward compatibility (both avatar and camera look at it)
            let target_pos = Vector3::new(camera[0], camera[1], camera[2]) + scene_position;
            let target_pos = Vector3::new(target_pos.x, target_pos.y, -target_pos.z);
            explorer_node.call("player_look_at", &[Variant::from(target_pos)]);
        }
        (None, None) => {
            // No targets: do nothing
        }
    }
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
        response.send(Err("Primary Player is outside the scene".to_string()));
        return;
    }

    // Get Global node from scene tree (Global is an autoload, not an Engine singleton)
    let Some(tree) = godot::classes::Engine::singleton().get_main_loop() else {
        tracing::error!("Cannot get main loop");
        response.send(Err("modal_manager not available".to_string()));
        return;
    };

    let Some(root) = tree.cast::<godot::classes::SceneTree>().get_root() else {
        tracing::error!("Cannot get root node");
        response.send(Err("modal_manager not available".to_string()));
        return;
    };

    let Some(global) = root.get_node_or_null("/root/Global") else {
        tracing::error!("Cannot get Global node from scene tree");
        response.send(Err("modal_manager not available".to_string()));
        return;
    };

    let modal_manager_variant = global.get("modal_manager");
    let Some(modal_manager) = modal_manager_variant
        .try_to::<godot::prelude::Gd<godot::classes::Node>>()
        .ok()
    else {
        tracing::error!("Cannot convert modal_manager variant to Node");
        response.send(Err("modal_manager not available".to_string()));
        return;
    };

    let mut modal_manager = modal_manager;
    let target_parcel = Vector2i::new(world_coordinates[0], world_coordinates[1]);

    modal_manager.call("async_show_teleport_modal", &[target_parcel.to_variant()]);

    // Send Ok immediately - the modal will handle the actual teleportation
    // This matches the behavior where the RPC call succeeds once the modal is shown
    response.send(Ok(()));
}

pub fn trigger_emote(scene: &Scene, current_parcel_scene_id: &SceneId, emote_id: &str) {
    // Check if player is inside the scene that requested the move
    if !_player_is_inside_scene(scene, current_parcel_scene_id) {
        tracing::warn!("triggerEmote failed: Primary Player is outside the scene");
        return;
    }

    let mut avatar_node = get_avatar_node(scene);
    avatar_node.call("async_play_emote", &[emote_id.to_variant()]);

    // Broadcast emote to other players via comms
    DclGlobal::singleton()
        .bind()
        .get_comms()
        .bind_mut()
        .send_emote(emote_id.to_godot());
}

pub fn trigger_scene_emote(
    scene: &Scene,
    current_parcel_scene_id: &SceneId,
    emote_src: &str,
    looping: &bool,
) {
    tracing::debug!(
        "SCENE_EMOTE_TRIGGERED: src={}, loop={}, scene={}",
        emote_src,
        looping,
        scene.scene_entity_definition.id
    );

    // Check if player is inside the scene that requested the move
    if !_player_is_inside_scene(scene, current_parcel_scene_id) {
        tracing::warn!("triggerSceneEmote failed: Primary Player is outside the scene");
        return;
    }

    tracing::info!(
        "triggerSceneEmote: emote_src={}, looping={}, scene_id={}",
        emote_src,
        looping,
        scene.scene_entity_definition.id
    );

    let Some(emote_hash) = scene.content_mapping.get_scene_emote_hash(emote_src) else {
        tracing::warn!(
            "triggerSceneEmote failed: Emote '{}' not found in content mapping",
            emote_src
        );
        return;
    };

    tracing::info!(
        "triggerSceneEmote: resolved glb_hash={}, audio_hash={:?}",
        emote_hash.glb_hash,
        emote_hash.audio_hash
    );

    // Build scene-emote URN (same format used for comms)
    let scene_emote_urn = format!(
        "urn:decentraland:off-chain:scene-emote:{}-{}-{}",
        scene.scene_entity_definition.id, emote_hash.glb_hash, looping
    );

    let mut avatar_node = get_avatar_node(scene);

    // Register content URL for this scene (so GDScript can look it up)
    avatar_node.call(
        "register_scene_emote_content",
        &[
            scene.scene_entity_definition.id.to_godot().to_variant(),
            scene.content_mapping.base_url.to_godot().to_variant(),
            emote_hash.glb_hash.to_godot().to_variant(),
            emote_hash
                .audio_hash
                .as_deref()
                .unwrap_or_default()
                .to_godot()
                .to_variant(),
        ],
    );

    // Call the SAME function as wearable emotes!
    avatar_node.call("async_play_emote", &[scene_emote_urn.to_variant()]);

    // Broadcast to other players
    tracing::info!("triggerSceneEmote: broadcasting URN={}", scene_emote_urn);
    DclGlobal::singleton()
        .bind()
        .get_comms()
        .bind_mut()
        .send_emote(scene_emote_urn.to_godot());
}
