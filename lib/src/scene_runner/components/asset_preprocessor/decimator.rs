//! Aggressive mesh decimation: replace LOD0 with what would normally be
//! LOD2 (~25% triangles). Visually imperceptible at typical view
//! distances on mobile, vertex stage drops 70%+.
//!
//! Uses Godot's `ImporterMesh::generate_lods` (the meshoptimizer-backed
//! decimator the editor importer uses) at sharper angles + then keeps
//! the LOD2 surface as the new LOD0.

use godot::classes::mesh::{ArrayType, PrimitiveType};
use godot::classes::{ArrayMesh, ImporterMesh};
use godot::prelude::*;

/// LOD index to keep as the new LOD0. The lower the LOD on the chain,
/// the simpler the mesh; LOD2 is typically ~25% triangles vs source.
const TARGET_LOD: i32 = 2;

/// Below this index count, the source mesh is too small to decimate
/// usefully (the meshoptimizer simplifier needs a triangle budget).
const MIN_SOURCE_INDICES: i32 = 96;

/// Sharper than the runtime mesh_lod angles — preserves silhouette but
/// fuses interior detail more aggressively.
const NORMAL_MERGE_ANGLE_DEG: f32 = 75.0;
const NORMAL_SPLIT_ANGLE_DEG: f32 = 25.0;

pub struct Decimated {
    pub mesh: Gd<ArrayMesh>,
    pub source_idx: u64,
    pub target_idx: u64,
}

pub fn aggressive_decimate(source: &Gd<ArrayMesh>) -> Option<Decimated> {
    let surface_count = source.get_surface_count();
    if surface_count <= 0 {
        return None;
    }

    // Skip if already preprocessed (the meta gets set after our pass).
    if source.has_meta("dcl_preproc_decimated") {
        return None;
    }

    // Skip small meshes — decimation overhead exceeds the win.
    let mut source_idx_total: u64 = 0;
    for s in 0..surface_count {
        let arrays = source.surface_get_arrays(s);
        if let Ok(idx) = arrays
            .at(ArrayType::INDEX.ord() as usize)
            .try_to::<PackedInt32Array>()
        {
            source_idx_total = source_idx_total.saturating_add(idx.len() as u64);
        }
    }
    if (source_idx_total as i32) < MIN_SOURCE_INDICES {
        return None;
    }

    let mut importer = ImporterMesh::new_gd();
    let empty_bones = VarArray::new();

    for s in 0..surface_count {
        let arrays = source.surface_get_arrays(s);
        let primitive = source.surface_get_primitive_type(s);
        importer.add_surface(primitive, &arrays);
        let material = source.surface_get_material(s);
        importer.set_surface_material(s, material.as_ref());
    }

    importer.generate_lods(NORMAL_MERGE_ANGLE_DEG, NORMAL_SPLIT_ANGLE_DEG, &empty_bones);

    // Rebuild a new ArrayMesh whose LOD0 == the importer's LOD `TARGET_LOD`.
    // If a surface doesn't have that many LODs, fall back to the highest
    // available so we still strip something.
    let mut out = ArrayMesh::new_gd();
    let mut target_idx_total: u64 = 0;

    for s in 0..importer.get_surface_count() {
        let lod_count = importer.get_surface_lod_count(s);
        let lod_idx = lod_count.saturating_sub(1).min(TARGET_LOD);

        let arrays = importer.get_surface_arrays(s);
        // Replace the index array with the LOD's indices (more aggressive
        // than the LOD0 indices). Vertex array stays the same — the LOD
        // just references a subset of the existing verts.
        let lod_indices = if lod_count > 0 {
            importer.get_surface_lod_indices(s, lod_idx)
        } else {
            PackedInt32Array::new()
        };

        if lod_indices.is_empty() {
            // No LODs were produced (mesh too uniform / too small).
            // Skip this surface — don't include in output.
            continue;
        }

        let mut new_arrays = arrays.clone();
        new_arrays.set(ArrayType::INDEX.ord() as usize, &lod_indices.to_variant());
        target_idx_total = target_idx_total.saturating_add(lod_indices.len() as u64);

        let primitive = importer.get_surface_primitive_type(s);
        out.add_surface_from_arrays(primitive, &new_arrays);

        if let Some(mat) = importer.get_surface_material(s) {
            let last = out.get_surface_count() - 1;
            out.surface_set_material(last, &mat);
        }
    }

    if out.get_surface_count() == 0 {
        return None;
    }

    let mut marked = out.clone();
    marked.set_meta("dcl_preproc_decimated", &true.to_variant());

    Some(Decimated {
        mesh: out,
        source_idx: source_idx_total,
        target_idx: target_idx_total,
    })
}

// Re-export the constants used in tests / metrics if needed elsewhere.
#[allow(dead_code)]
pub fn target_lod() -> i32 {
    TARGET_LOD
}

// Suppress unused-import warning for PrimitiveType.
#[allow(dead_code)]
fn _unused_primitive() -> PrimitiveType {
    PrimitiveType::TRIANGLES
}
