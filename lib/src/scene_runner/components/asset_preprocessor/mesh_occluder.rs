//! Spawn an `ArrayOccluder3D` that mirrors the source mesh's geometry,
//! decimated aggressively. Way more accurate than `BoxOccluder3D` for
//! actual building shapes — culls things behind the real silhouette
//! instead of just an AABB.

use godot::classes::base_material_3d::Transparency;
use godot::classes::{
    ArrayMesh, BaseMaterial3D, BoxOccluder3D, MeshInstance3D, OccluderInstance3D,
};
use godot::obj::NewAlloc;
use godot::prelude::*;

// Lowered from 5.0 → 2.0 because DCL buildings are split into many MIs
// each smaller than 5m. THIN_RATIO=0.05 lets thin walls and floor
// planes through (they make great occluders for everything behind
// them); the per-axis inset below stops their BoxOccluder from
// overshooting on the thin axis.
const MIN_AABB_DIAG_M: f32 = 2.0;
const THIN_RATIO: f32 = 0.05;
/// BoxOccluder inset on the *fat* axes — shrink 15 % so a player
/// walking past doesn't false-cull the visible mesh's edge.
const BOX_INSET_FAT: f32 = 0.85;
/// BoxOccluder inset on the *thin* axis — shrink 50 %. The bug was
/// a wall 4×3×0.2 with uniform 15 % inset producing a 3.4×2.55×0.17
/// occluder whose half-thickness (0.085 m) still extends past
/// decals/letters/posters placed 0.05 m in front of the wall and
/// false-culls them. With 50 % inset on the thin axis the same wall
/// gives 3.4×2.55×0.10 → half-thickness 0.05 m, just at the
/// decal-clearance edge.
const BOX_INSET_THIN: f32 = 0.50;
/// Axis lengths under this are treated as "thin" and get the
/// aggressive inset above.
const THIN_AXIS_M: f32 = 0.5;

pub fn try_spawn_for(
    mi: &mut Gd<MeshInstance3D>,
    mesh: &Gd<ArrayMesh>,
    scene_root: &Gd<godot::classes::Node>,
) -> bool {
    if mi.has_meta("dcl_preproc_occluder") {
        return false;
    }

    let aabb = mesh.get_aabb();
    let size = aabb.size;
    let diag = (size.x * size.x + size.y * size.y + size.z * size.z).sqrt();
    if diag < MIN_AABB_DIAG_M {
        return false;
    }
    let max_axis = size.x.max(size.y).max(size.z);
    let min_axis = size.x.min(size.y).min(size.z);
    if max_axis > 0.0 && min_axis / max_axis < THIN_RATIO {
        return false;
    }

    if !is_opaque_material(mi, mesh) {
        return false;
    }

    // Per-axis BoxOccluder inset. Fat axes shrink 15 %; thin axes
    // (under THIN_AXIS_M) shrink 50 % so a wall's BoxOccluder
    // doesn't overshoot decals/posters at ~5 cm clearance in front
    // of it. Without this, the previous uniform 15 % inset caused
    // sign letters and bushes against walls to be false-culled.
    let axis_inset = |a: f32| {
        if a < THIN_AXIS_M {
            BOX_INSET_THIN
        } else {
            BOX_INSET_FAT
        }
    };
    let inset_size = Vector3::new(
        size.x * axis_inset(size.x),
        size.y * axis_inset(size.y),
        size.z * axis_inset(size.z),
    );
    let center_local = aabb.position + size * 0.5;

    let mut box_occluder = BoxOccluder3D::new_gd();
    box_occluder.set_size(inset_size);

    let mut occluder_instance = OccluderInstance3D::new_alloc();
    occluder_instance.set_occluder(&box_occluder.upcast::<godot::classes::Occluder3D>());
    occluder_instance.set_position(center_local);

    mi.add_child(&occluder_instance.clone().upcast::<godot::classes::Node>());
    // PackedScene::pack only serializes descendants whose owner is in
    // the saved subtree. Without an owner the OccluderInstance3D gets
    // dropped on save_node_as_scene, so the device never sees the
    // baked BoxOccluder. Anchor on the scene root explicitly.
    occluder_instance.set_owner(scene_root);

    mi.set_meta("dcl_preproc_occluder", &true.to_variant());
    true
}

fn is_opaque_material(mi: &Gd<MeshInstance3D>, mesh: &Gd<ArrayMesh>) -> bool {
    let material = mi
        .get_active_material(0)
        .or_else(|| mi.get_surface_override_material(0))
        .or_else(|| mesh.surface_get_material(0));
    let Some(mat) = material else {
        return false;
    };
    if let Ok(base) = mat.try_cast::<BaseMaterial3D>() {
        return base.get_transparency() == Transparency::DISABLED;
    }
    // Unknown material kind — be conservative, skip.
    false
}
