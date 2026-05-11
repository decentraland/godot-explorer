//! Bake a decimated copy of an `ArrayMesh` suitable for use as
//! `ArrayMesh.shadow_mesh`. The renderer substitutes this lower-poly
//! geometry during the shadow pass; the visible pass keeps the source.

use godot::classes::mesh::{ArrayType, PrimitiveType};
use godot::classes::ArrayMesh;
use godot::prelude::*;

pub struct ShadowBakeResult {
    pub mesh: Gd<ArrayMesh>,
    pub source_index_total: u64,
    pub shadow_index_total: u64,
}

/// Keep 1 of every `STRIDE` triangles. We bypass `SurfaceTool::generate_lod`
/// (which wraps `meshopt_simplify` and refuses to decimate non-manifold
/// geometry — DCL user-authored GLBs are lousy with non-manifold edges)
/// and do a topology-blind stride. Silhouette + depth are the only outputs
/// that matter for shadow rasterization, so dropping interior triangles is
/// safe.
const STRIDE: usize = 4;

pub fn bake_shadow_mesh(source: &Gd<ArrayMesh>) -> Option<ShadowBakeResult> {
    let surface_count = source.get_surface_count();
    if surface_count <= 0 {
        return None;
    }

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

        let triangle_count = (src_indices.len() as usize) / 3;
        let kept = triangle_count.div_ceil(STRIDE);
        let mut decimated = PackedInt32Array::new();
        for t in 0..kept {
            let src_t = t * STRIDE;
            if src_t >= triangle_count {
                break;
            }
            let base = src_t * 3;
            decimated.push(src_indices.get(base).unwrap_or(0));
            decimated.push(src_indices.get(base + 1).unwrap_or(0));
            decimated.push(src_indices.get(base + 2).unwrap_or(0));
        }
        if decimated.len() < 3 || decimated.len() % 3 != 0 {
            continue;
        }

        // Build a fresh VarArray. Cloning src_arrays and replacing the INDEX
        // slot leaves the source's full index buffer in place under godot-rust
        // 0.4.5's Variant CoW semantics, so we copy slot-by-slot instead.
        let array_max = ArrayType::MAX.ord() as usize;
        let mut shadow_arrays = VarArray::new();
        shadow_arrays.resize(array_max, &Variant::nil());
        for ai in 0..array_max {
            if ai == ArrayType::INDEX.ord() as usize {
                shadow_arrays.set(ai, &decimated.to_variant());
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
