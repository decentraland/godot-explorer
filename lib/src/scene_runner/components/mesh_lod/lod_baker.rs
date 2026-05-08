//! Build an `ArrayMesh` with a baked LOD chain from a source mesh by
//! replaying its surfaces into an `ImporterMesh` and calling
//! `generate_lods`.
//!
//! `ImporterMesh::generate_lods` is the same code path the GLTF importer
//! uses at design time when `meshes/generate_lods=true` is set on a `.import`
//! file. We invoke it at runtime so DCL scenes — which load through the
//! Rust GLTF pipeline, not the editor importer — also get LOD chains.

use godot::classes::mesh::ArrayType;
use godot::classes::{ArrayMesh, ImporterMesh};
use godot::prelude::*;

/// Default normal-merge angle (degrees) used by Godot's GLTF importer.
const NORMAL_MERGE_ANGLE_DEG: f32 = 60.0;
/// Default normal-split angle (degrees) used by Godot's GLTF importer.
const NORMAL_SPLIT_ANGLE_DEG: f32 = 25.0;

pub struct BakeResult {
    pub mesh: Gd<ArrayMesh>,
    pub source_index_total: u64,
    pub lod0_index_total: u64,
}

pub fn bake_lods(source: &Gd<ArrayMesh>) -> Option<BakeResult> {
    let surface_count = source.get_surface_count();
    if surface_count <= 0 {
        return None;
    }

    let mut importer = ImporterMesh::new_gd();
    let empty_bones = VarArray::new();

    let mut source_index_total: u64 = 0;
    for s in 0..surface_count {
        let arrays = source.surface_get_arrays(s);
        if let Ok(idx) = arrays
            .at(ArrayType::INDEX.ord() as usize)
            .try_to::<PackedInt32Array>()
        {
            source_index_total = source_index_total.saturating_add(idx.len() as u64);
        }
        let primitive = source.surface_get_primitive_type(s);
        importer.add_surface(primitive, &arrays);
        let material = source.surface_get_material(s);
        importer.set_surface_material(s, material.as_ref());
    }

    importer.generate_lods(NORMAL_MERGE_ANGLE_DEG, NORMAL_SPLIT_ANGLE_DEG, &empty_bones);

    let baked = importer.get_mesh()?;

    // LOD0 == original geometry; pulling its index total back out gives us
    // a sanity check that surfaces survived the round-trip.
    let mut lod0_index_total: u64 = 0;
    for s in 0..baked.get_surface_count() {
        let arrays = baked.surface_get_arrays(s);
        if let Ok(idx) = arrays
            .at(ArrayType::INDEX.ord() as usize)
            .try_to::<PackedInt32Array>()
        {
            lod0_index_total = lod0_index_total.saturating_add(idx.len() as u64);
        }
    }

    let mut baked_marked = baked.clone();
    baked_marked.set_meta("dcl_mesh_lod_baked", &true.to_variant());

    Some(BakeResult {
        mesh: baked,
        source_index_total,
        lod0_index_total,
    })
}
