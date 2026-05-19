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

/// Convert a Decentraland scene `Vector3` to a Godot world `Vector3`.
///
/// Godot's Z axis is the negation of Decentraland's Z (see
/// `DclTransformAndParent::to_godot_transform_3d`), so directions/vectors that
/// originate in scene coordinates must have their Z component flipped before
/// being applied to Godot nodes.
fn scene_vec_to_godot(v: &proto_components::common::Vector3) -> Vector3 {
    Vector3::new(v.x, v.y, -v.z)
}

/// Reads `PBPhysicsCombinedForce` on the player entity and stores the resulting
/// continuous force on the scene so the player controller can sample it.
///
/// Force is only applied while the scene is the current parcel scene; for any
/// other scene the stored value is reset to zero. The force vector is the
/// summary of all per-frame forces accumulated by the scene's SDK code.
pub fn update_physics_combined_force(
    scene: &mut Scene,
    crdt_state: &mut SceneCrdtState,
    current_parcel_scene_id: &SceneId,
) {
    let is_current_parcel_scene = scene.scene_id == *current_parcel_scene_id;

    if !is_current_parcel_scene {
        // Discard any stale state so we don't leak forces from a scene the
        // player just left. Mirrors Unity's `ResetExternalForce` on scene exit.
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

/// Reads `PBPhysicsCombinedImpulse` on the player entity and queues a new
/// one-shot impulse when `event_id` advances.
///
/// Mirrors the Unity dirty-flag dedup: each `event_id` value is applied at most
/// once. The first sighting of any `event_id` is always applied; subsequent
/// values are only applied when `event_id` differs from `last_impulse_event_id`.
/// Non-current scenes are ignored entirely so a scene can't push impulses to
/// the player from outside its parcel boundary.
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

    // Only react when the CRDT dirty set says this scene's impulse component
    // changed this tick (avoids re-reading the same event every tick).
    let is_dirty = dirty_lww_components
        .get(&SceneComponentId::PHYSICS_COMBINED_IMPULSE)
        .is_some_and(|entities| entities.contains(&SceneEntityId::PLAYER));

    if !is_dirty {
        return;
    }

    tracing::debug!(
        "physics_combined: impulse dirty on scene {:?} (last_event_id={:?})",
        scene.scene_id,
        scene.last_impulse_event_id,
    );

    let impulse_component = SceneCrdtStateProtoComponents::get_physics_combined_impulse(crdt_state);
    let Some(entry) = impulse_component.get(&SceneEntityId::PLAYER) else {
        tracing::debug!("physics_combined: dirty but no LWW entry for PLAYER — skipping");
        return;
    };
    let Some(pb) = entry.value.as_ref() else {
        tracing::debug!("physics_combined: dirty but entry.value is None — skipping");
        return;
    };
    let Some(vector) = pb.vector.as_ref() else {
        tracing::debug!(
            "physics_combined: dirty but pb.vector is None (event_id={}) — skipping",
            pb.event_id
        );
        return;
    };

    if scene.last_impulse_event_id == Some(pb.event_id) {
        tracing::debug!(
            "physics_combined: dedup hit — event_id={} already applied",
            pb.event_id
        );
        return;
    }

    let godot_vec = scene_vec_to_godot(vector);
    tracing::debug!(
        "physics_combined: queue impulse event_id={} dcl=({:.3},{:.3},{:.3}) godot=({:.3},{:.3},{:.3})",
        pb.event_id,
        vector.x,
        vector.y,
        vector.z,
        godot_vec.x,
        godot_vec.y,
        godot_vec.z,
    );
    scene.last_impulse_event_id = Some(pb.event_id);
    scene.pending_impulses.push(godot_vec);
}
