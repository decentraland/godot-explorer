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

/// Reads `PBPhysicsCombinedImpulse` on the player entity and queues a one-shot
/// impulse for every CRDT write the scene makes to it.
///
/// We deliberately mirror unity-explorer's `SDKExternalPhysicsSystems`, which
/// gates on its per-write `IsDirty` flag and **does not read `event_id`**.
/// The proto comment makes `event_id` sound load-bearing for renderer-side
/// dedup, but the shipped Unity client ignores it — and production scenes
/// like Genesis Plaza's `bouncePad.ts` rely on that, calling
/// `PhysicsCombinedImpulse.createOrReplace(player, { eventId: 0, vector })`
/// every trigger enter. To match the Unity behavior bar, we fire once per
/// CRDT-dirty signal: `createOrReplace` (even with a stale `event_id`)
/// produces one CRDT message → one renderer fire.
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
