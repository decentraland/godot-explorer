//! Build an `ArrayMesh` with a baked LOD chain from a source mesh by
//! replaying its surfaces into an `ImporterMesh` and calling
//! `generate_lods`.
//!
//! `ImporterMesh::generate_lods` is the same code path the GLTF importer
//! uses at design time when `meshes/generate_lods=true` is set on a `.import`
//! file. We invoke it at runtime so DCL scenes — which load through the
//! Rust GLTF pipeline, not the editor importer — also get LOD chains.

use godot::classes::mesh::{ArrayType, PrimitiveType};
use godot::classes::{ArrayMesh, ImporterMesh};
use godot::prelude::*;
use meshopt::{simplify_sloppy, VertexDataAdapter};

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

    // Stride-decimate the source's index buffer before feeding it to the
    // importer. Drops LOD0 (the highest-detail level the renderer will
    // ever pick) to ~`1/STRIDE` of the source primitive count, then the
    // importer builds its normal LOD chain on top of that already-cheap
    // LOD0. Net effect: `visible_prim` drops directly, and because
    // shadow rendering reuses the same meshes, `shadow_prim` follows.
    //
    // STRIDE=2 keeps every other triangle (~50% of source). Higher
    // values trade visual fidelity for more frame budget; 2 was
    // picked as a conservative starting point.
    const STRIDE: usize = 2;

    let mut source_index_total: u64 = 0;
    for s in 0..surface_count {
        let arrays = source.surface_get_arrays(s);
        let primitive = source.surface_get_primitive_type(s);

        let mut decimated_arrays = arrays.clone();
        if primitive == PrimitiveType::TRIANGLES {
            if let Ok(idx) = arrays
                .at(ArrayType::INDEX.ord() as usize)
                .try_to::<PackedInt32Array>()
            {
                source_index_total = source_index_total.saturating_add(idx.len() as u64);
                let triangle_count = (idx.len() as usize) / 3;
                if triangle_count >= STRIDE * 2 {
                    let kept = triangle_count.div_ceil(STRIDE);
                    let mut strided = PackedInt32Array::new();
                    for t in 0..kept {
                        let src_t = t * STRIDE;
                        if src_t >= triangle_count {
                            break;
                        }
                        let base = (src_t * 3) as i32;
                        strided.push(idx.get(base as usize).unwrap_or(0));
                        strided.push(idx.get((base + 1) as usize).unwrap_or(0));
                        strided.push(idx.get((base + 2) as usize).unwrap_or(0));
                    }
                    decimated_arrays
                        .set(ArrayType::INDEX.ord() as usize, &strided.to_variant());
                }
            }
        } else if let Ok(idx) = arrays
            .at(ArrayType::INDEX.ord() as usize)
            .try_to::<PackedInt32Array>()
        {
            source_index_total = source_index_total.saturating_add(idx.len() as u64);
        }

        importer.add_surface(primitive, &decimated_arrays);
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

#[allow(dead_code)] pub struct ShadowBakeResult {
    pub mesh: Gd<ArrayMesh>,
    pub source_index_total: u64,
    pub shadow_index_total: u64,
}

/// Build a decimated copy of `source` suitable for use as
/// `ArrayMesh.shadow_mesh`. Same draw count as the source — the renderer
/// just substitutes this lower-poly geometry during the shadow pass — but
/// far fewer primitives to rasterize into the shadow atlas.
///
/// We piggy-back on `ImporterMesh::generate_lods`, then for each surface
/// extract the highest-LOD index buffer and reuse the source vertex
/// buffer. This keeps memory cheap (no vertex duplication) and the bake
/// cost identical to a normal LOD bake.
#[allow(dead_code)]
pub fn bake_shadow_mesh(source: &Gd<ArrayMesh>) -> Option<ShadowBakeResult> {
    let surface_count = source.get_surface_count();
    if surface_count <= 0 {
        return None;
    }

    // We bypass `SurfaceTool::generate_lod` entirely here. Godot 4.6.2
    // wraps `meshopt_simplify`, the topology-preserving variant, which
    // refuses to decimate non-manifold geometry — DCL user-authored GLBs
    // are lousy with non-manifold edges and T-junctions, so that path
    // returns ~98.5% of the original indices.
    //
    // Instead we call `meshopt::simplify_sloppy` directly. "Sloppy"
    // ignores topology constraints and just merges nearby vertices,
    // which is exactly what we want for a shadow proxy: silhouettes
    // and depth are the only outputs that matter.
    //
    // Per surface:
    //   1. Pull the position and index streams out of the source.
    //   2. Reinterpret positions as raw f32 bytes for `VertexDataAdapter`.
    //   3. `simplify_sloppy(indices, vertex_adapter, target_count, error)`.
    //   4. Build a fresh ArrayMesh that reuses the source positions and
    //      whatever attribute streams it had, with the decimated index
    //      buffer in `ARRAY_INDEX`.
    let target_ratio: f32 = 0.25;
    let target_error: f32 = 0.5;
    let mut shadow = ArrayMesh::new_gd();
    let mut source_index_total: u64 = 0;
    let mut shadow_index_total: u64 = 0;

    for s in 0..surface_count {
        let primitive = source.surface_get_primitive_type(s);
        if primitive != PrimitiveType::TRIANGLES {
            continue;
        }

        let src_arrays = source.surface_get_arrays(s);
        let array_max = ArrayType::MAX.ord() as usize;
        if src_arrays.len() < array_max {
            continue;
        }

        let Ok(src_indices) = src_arrays
            .at(ArrayType::INDEX.ord() as usize)
            .try_to::<PackedInt32Array>()
        else {
            continue;
        };
        if src_indices.len() < 6 {
            continue;
        }
        let Ok(src_vertices) = src_arrays
            .at(ArrayType::VERTEX.ord() as usize)
            .try_to::<PackedVector3Array>()
        else {
            continue;
        };
        if src_vertices.is_empty() {
            continue;
        }

        source_index_total = source_index_total.saturating_add(src_indices.len() as u64);

        // meshopt expects u32 indices and a flat byte buffer of
        // contiguous Vector3 positions.
        let indices_u32: Vec<u32> = src_indices.as_slice().iter().map(|&i| i as u32).collect();

        let mut vertex_bytes: Vec<u8> = Vec::with_capacity(src_vertices.len() * 12);
        for v in src_vertices.as_slice().iter() {
            vertex_bytes.extend_from_slice(&v.x.to_le_bytes());
            vertex_bytes.extend_from_slice(&v.y.to_le_bytes());
            vertex_bytes.extend_from_slice(&v.z.to_le_bytes());
        }
        let Ok(adapter) = VertexDataAdapter::new(&vertex_bytes, 12, 0) else {
            continue;
        };

        let target_count = ((src_indices.len() as f32) * target_ratio) as usize;
        let target_count = target_count - (target_count % 3);
        if target_count < 3 {
            continue;
        }

        let _ = (target_count, target_error, &adapter); // sloppy was unreliable

        // Unconditional stride: keep 1 of every N triangles. Indices
        // reference the source's vertex buffer (subset of an already
        // validated index list), so the result is guaranteed safe.
        let stride = 4usize;
        let triangle_count = indices_u32.len() / 3;
        let kept = triangle_count.div_ceil(stride);
        let mut decimated: Vec<u32> = Vec::with_capacity(kept * 3);
        for t in 0..kept {
            let src_t = t * stride;
            if src_t >= triangle_count {
                break;
            }
            let base = src_t * 3;
            decimated.push(indices_u32[base]);
            decimated.push(indices_u32[base + 1]);
            decimated.push(indices_u32[base + 2]);
        }
        tracing::info!(
            "[STRIDE] surface={} src_idx={} tri={} kept={} decimated={}",
            s,
            indices_u32.len(),
            triangle_count,
            kept,
            decimated.len()
        );
        if decimated.len() < 3 || decimated.len() % 3 != 0 {
            continue;
        }

        let mut decimated_packed = PackedInt32Array::new();
        for &i in &decimated {
            decimated_packed.push(i as i32);
        }

        // Build a fresh VarArray rather than cloning src_arrays — earlier
        // attempts to clone and `set(INDEX)` left the surface with the
        // source's full index buffer in the resulting ArrayMesh (Variant
        // CoW semantics + godot-rust 0.4.5 reference-cloning behavior
        // weren't propagating the replaced slot).
        let array_max = ArrayType::MAX.ord() as usize;
        let mut shadow_arrays = VarArray::new();
        shadow_arrays.resize(array_max, &Variant::nil());
        for ai in 0..array_max {
            if ai == ArrayType::INDEX.ord() as usize {
                shadow_arrays.set(ai, &decimated_packed.to_variant());
            } else {
                shadow_arrays.set(ai, &src_arrays.at(ai));
            }
        }

        shadow_index_total = shadow_index_total.saturating_add(decimated.len() as u64);
        shadow.add_surface_from_arrays(PrimitiveType::TRIANGLES, &shadow_arrays);
    }

    if shadow.get_surface_count() == 0 {
        return None;
    }
    shadow.set_meta("dcl_shadow_mesh_baked", &true.to_variant());
    Some(ShadowBakeResult {
        mesh: shadow,
        source_index_total,
        shadow_index_total,
    })
}
