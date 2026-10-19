use dcl_core::app::Parcel;
use dcl_core::scene_requests::SceneRequest;

use crate::{
    dcl::{scene_apis::RpcResultSender, SceneId},
    engine::DclEngine,
    godot_classes::dcl_global::DclGlobal,
    scene_runner::scene::{Scene, SceneType},
};

use godot::{
    meta::ToGodot,
    prelude::{Vector2i, Vector3},
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
    if !_player_is_inside_scene(scene, current_parcel_scene_id) {
        response.send(Err("Primary Player is outside the scene".to_string()));
        return;
    }

    DclEngine::scene_request(
        SceneRequest::ChangeRealm {
            realm: to.to_string(),
            message: message.clone(),
        },
        response.clone(),
    );
}

pub fn open_external_url(
    scene: &Scene,
    current_parcel_scene_id: &SceneId,
    url: &Uri,
    response: &RpcResultSender<Result<(), String>>,
) {
    if !_player_is_inside_scene(scene, current_parcel_scene_id) {
        response.send(Err("Primary Player is outside the scene".to_string()));
        return;
    }

    DclEngine::scene_request(
        SceneRequest::OpenExternalUrl {
            url: url.to_string(),
        },
        response.clone(),
    );
}

pub fn open_nft_dialog(
    scene: &Scene,
    current_parcel_scene_id: &SceneId,
    urn: &str,
    response: &RpcResultSender<Result<(), String>>,
) {
    if !_player_is_inside_scene(scene, current_parcel_scene_id) {
        response.send(Err("Primary Player is outside the scene".to_string()));
        return;
    }

    DclEngine::scene_request(
        SceneRequest::OpenNft {
            urn: urn.to_string(),
        },
        response.clone(),
    );
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

    let Some(engine) = DclEngine::get() else {
        return;
    };
    let engine = engine.bind();

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
    engine.move_player_to(position_target, true);

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
            engine.camera_look_at(camera_pos);

            let avatar_pos = Vector3::new(avatar[0], avatar[1], avatar[2]) + scene_position;
            let avatar_pos = Vector3::new(avatar_pos.x, avatar_pos.y, -avatar_pos.z);
            engine.avatar_look_at_independent(avatar_pos);
        }
        (Some(avatar), None) => {
            // Only avatar target: avatar looks at it
            let target_pos = Vector3::new(avatar[0], avatar[1], avatar[2]) + scene_position;
            let target_pos = Vector3::new(target_pos.x, target_pos.y, -target_pos.z);
            engine.player_look_at(target_pos);
        }
        (None, Some(camera)) => {
            // Only camera target: backward compatibility (both avatar and camera look at it)
            let target_pos = Vector3::new(camera[0], camera[1], camera[2]) + scene_position;
            let target_pos = Vector3::new(target_pos.x, target_pos.y, -target_pos.z);
            engine.player_look_at(target_pos);
        }
        (None, None) => {
            // No targets: do nothing
        }
    }
}

// Teleport user to world coordinates, optionally in another realm (protocol#477).
pub fn teleport_to(
    scene: &Scene,
    current_parcel_scene_id: &SceneId,
    world_coordinates: &Option<[i32; 2]>,
    realm: &Option<String>,
    response: &RpcResultSender<Result<(), String>>,
) {
    if !_player_is_inside_scene(scene, current_parcel_scene_id) {
        response.send(Err("Primary Player is outside the scene".to_string()));
        return;
    }

    // Rejected by op_teleport_to already; kept so the promise can never hang.
    if world_coordinates.is_none() && realm.is_none() {
        response.send(Err(
            "teleportTo requires worldCoordinates, a realm, or both".to_string(),
        ));
        return;
    }

    DclEngine::scene_request(
        SceneRequest::Teleport {
            parcel: world_coordinates
                .map(|coordinates| Parcel::new(coordinates[0], coordinates[1])),
            realm: realm.clone(),
        },
        response.clone(),
    );
}

pub fn trigger_emote(scene: &Scene, current_parcel_scene_id: &SceneId, emote_id: &str, mask: i64) {
    // Check if player is inside the scene that requested the move
    if !_player_is_inside_scene(scene, current_parcel_scene_id) {
        tracing::warn!("triggerEmote failed: Primary Player is outside the scene");
        return;
    }

    if let Some(engine) = DclEngine::get() {
        engine.bind().play_emote(emote_id, mask, scene.scene_id.0);
    }

    // Broadcast emote to other players via comms
    DclGlobal::singleton()
        .bind()
        .get_comms()
        .bind_mut()
        .send_emote(emote_id.to_godot(), mask);
}

pub fn trigger_scene_emote(
    scene: &Scene,
    current_parcel_scene_id: &SceneId,
    emote_src: &str,
    looping: &bool,
    mask: i64,
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

    if let Some(engine) = DclEngine::get() {
        let engine = engine.bind();
        // Register content URL for this scene (so GDScript can look it up)
        engine.register_scene_emote_content(
            &scene.scene_entity_definition.id,
            &scene.content_mapping.base_url,
            &emote_hash.glb_hash,
            emote_hash.audio_hash.as_deref().unwrap_or_default(),
        );

        // Call the SAME function as wearable emotes!
        engine.play_emote(&scene_emote_urn, mask, scene.scene_id.0);
    }

    // Broadcast to other players
    tracing::info!("triggerSceneEmote: broadcasting URN={}", scene_emote_urn);
    DclGlobal::singleton()
        .bind()
        .get_comms()
        .bind_mut()
        .send_emote(scene_emote_urn.to_godot(), mask);
}

/// `stopEmote` — end whatever the local player is playing, on behalf of the scene.
///
/// Permanent by construction: unlike the scene-boundary suspend, this also drops a
/// masked emote parked for replay, so walking back into the scene can't resurrect it
/// (Unity does the same via `masked.EmoteUrn = default` in `TryStopEmote`).
///
/// Telling the other players is **Pulse-only**: the stop rides the `set_emoting(false)`
/// edge in `CommunicationManager`, and that whole body sits inside
/// `#[cfg(feature = "use_pulse")]`. On the rfc4/LiveKit path this client only ever sends
/// `PlayerEmote { is_stopping: None }` at trigger time and has no stop message at all, so
/// on a LiveKit-only realm a scene's `stopEmote` ends the emote locally while remote
/// viewers keep looping it. Pre-existing gap, not addressed here.
///
/// The scene id is forwarded so the avatar can scope the stop: a masked emote is only
/// ended by the scene that started it, matching Unity's `TryStopEmote`, which stops the
/// full-body emote globally but touches only its own scene world's masked component.
pub fn stop_emote(scene: &Scene, current_parcel_scene_id: &SceneId) {
    // Same gate as triggerEmote: only the scene the player is standing in may stop it.
    // Note this passes unconditionally for SceneType::Global, which is exactly why the
    // masked case needs the owner check on the GDScript side.
    if !_player_is_inside_scene(scene, current_parcel_scene_id) {
        tracing::warn!("stopEmote failed: Primary Player is outside the scene");
        return;
    }

    if let Some(engine) = DclEngine::get() {
        engine.bind().stop_emote_from_scene(scene.scene_id.0);
    }
}
