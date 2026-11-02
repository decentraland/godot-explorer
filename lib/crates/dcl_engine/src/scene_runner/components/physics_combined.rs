use godot::builtin::Vector3;

use crate::{
    dcl::{
        components::{proto_components, SceneComponentId, SceneEntityId},
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
        SceneId,
    },
    scene_runner::scene::Scene,
};

/// Convert a DCL scene-space vector to Godot world-space (Z is negated).
fn scene_vec_to_godot(v: &proto_components::common::Vector3) -> Vector3 {
    Vector3::new(v.x, v.y, -v.z)
}

/// Update the per-scene cached force from the player's `PBPhysicsCombinedForce`.
/// Non-current scenes are cleared so wind zones stop the moment the player leaves.
pub fn update_physics_combined_force(
    scene: &mut Scene,
    crdt_state: &mut SceneCrdtState,
    current_parcel_scene_id: &SceneId,
) {
    let is_current_parcel_scene = scene.scene_id == *current_parcel_scene_id;

    if !is_current_parcel_scene {
        if scene.active_external_force != Vector3::ZERO {
            tracing::debug!(
                "physics_combined: force cleared on scene {:?} (no longer current)",
                scene.scene_id
            );
        }
        scene.active_external_force = Vector3::ZERO;
        return;
    }

    let force_component = SceneCrdtStateProtoComponents::get_physics_combined_force(crdt_state);
    let new_force = force_component
        .get(&SceneEntityId::PLAYER)
        .and_then(|entry| entry.value.as_ref())
        .and_then(|pb| pb.vector.as_ref())
        .map(scene_vec_to_godot)
        .unwrap_or(Vector3::ZERO);

    if new_force != scene.active_external_force {
        tracing::debug!(
            "physics_combined: force changed on scene {:?}: {:?} → {:?}",
            scene.scene_id,
            scene.active_external_force,
            new_force
        );
    }
    scene.active_external_force = new_force;
}

/// Queue one impulse per CRDT write of the player's `PBPhysicsCombinedImpulse`.
///
/// The proto says `event_id` is the renderer's dedup key, but production scenes
/// (Genesis Plaza, etc.) ship with `eventId: 0` hardcoded and expect each
/// `createOrReplace` call to fire the impulse. So we gate on the CRDT dirty
/// signal alone — one write, one fire — and ignore `event_id`.
pub fn update_physics_combined_impulse(
    scene: &mut Scene,
    crdt_state: &mut SceneCrdtState,
    current_parcel_scene_id: &SceneId,
) {
    let dirty_lww_components = &scene.current_dirty.lww_components;
    let is_current_parcel_scene = scene.scene_id == *current_parcel_scene_id;

    if !is_current_parcel_scene {
        return;
    }

    let is_dirty = dirty_lww_components
        .get(&SceneComponentId::PHYSICS_COMBINED_IMPULSE)
        .is_some_and(|entities| entities.contains(&SceneEntityId::PLAYER));

    if !is_dirty {
        return;
    }

    let impulse_component = SceneCrdtStateProtoComponents::get_physics_combined_impulse(crdt_state);
    let Some(entry) = impulse_component.get(&SceneEntityId::PLAYER) else {
        return;
    };
    let Some(pb) = entry.value.as_ref() else {
        return;
    };
    let Some(vector) = pb.vector.as_ref() else {
        return;
    };

    let godot_vec = scene_vec_to_godot(vector);
    tracing::debug!(
        "physics_combined: queue impulse event_id={} godot=({:.3},{:.3},{:.3})",
        pb.event_id,
        godot_vec.x,
        godot_vec.y,
        godot_vec.z,
    );
    scene.pending_impulses.push(godot_vec);
}
