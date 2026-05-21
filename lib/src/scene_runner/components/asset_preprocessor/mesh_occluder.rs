//! Spawn an `ArrayOccluder3D` that mirrors the source mesh's geometry,
//! decimated aggressively. Way more accurate than `BoxOccluder3D` for
//! actual building shapes — culls things behind the real silhouette
//! instead of just an AABB.

use godot::classes::base_material_3d::Transparency;
use godot::classes::mesh::ArrayType;
use godot::classes::{
    ArrayMesh, ArrayOccluder3D, BaseMaterial3D, BoxOccluder3D, ImporterMesh, MeshInstance3D,
    OccluderInstance3D,
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
const OCCLUDER_TARGET_LOD: i32 = 3; // very simplified (ArrayOccluder path)
const NORMAL_MERGE_ANGLE_DEG: f32 = 90.0;
const NORMAL_SPLIT_ANGLE_DEG: f32 = 25.0;
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

pub fn try_spawn_for(mi: &mut Gd<MeshInstance3D>, mesh: &Gd<ArrayMesh>) -> bool {
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
    let axis_inset = |a: f32| if a < THIN_AXIS_M { BOX_INSET_THIN } else { BOX_INSET_FAT };
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

    mi.add_child(&occluder_instance.upcast::<godot::classes::Node>());
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

fn build_occluder_geometry(
    source: &Gd<ArrayMesh>,
) -> Option<(PackedVector3Array, PackedInt32Array)> {
    let surface_count = source.get_surface_count();
    if surface_count <= 0 {
        return None;
    }

    let mut importer = ImporterMesh::new_gd();
    let empty_bones = VarArray::new();
    for s in 0..surface_count {
        let arrays = source.surface_get_arrays(s);
        let primitive = source.surface_get_primitive_type(s);
        importer.add_surface(primitive, &arrays);
    }
    importer.generate_lods(NORMAL_MERGE_ANGLE_DEG, NORMAL_SPLIT_ANGLE_DEG, &empty_bones);

    // Aggregate the highest LOD across all surfaces into a single
    // vertex+index buffer for the ArrayOccluder3D.
    let mut all_verts = PackedVector3Array::new();
    let mut all_idx = PackedInt32Array::new();
    let mut vert_offset: i32 = 0;

    for s in 0..importer.get_surface_count() {
        let lod_count = importer.get_surface_lod_count(s);
        let lod_idx = lod_count.saturating_sub(1).min(OCCLUDER_TARGET_LOD);
        if lod_count == 0 {
            continue;
        }

        let arrays = importer.get_surface_arrays(s);
        let Ok(verts) = arrays
            .at(ArrayType::VERTEX.ord() as usize)
            .try_to::<PackedVector3Array>()
        else {
            continue;
        };
        let lod_indices = importer.get_surface_lod_indices(s, lod_idx);
        if lod_indices.is_empty() {
            continue;
        }

        for i in 0..verts.len() {
            if let Some(v) = verts.get(i) {
                all_verts.push(v);
            }
        }
        for i in 0..lod_indices.len() {
            if let Some(idx) = lod_indices.get(i) {
                all_idx.push(idx + vert_offset);
            }
        }
        vert_offset += verts.len() as i32;
    }

    if all_verts.is_empty() || all_idx.is_empty() {
        return None;
    }
    Some((all_verts, all_idx))
}
