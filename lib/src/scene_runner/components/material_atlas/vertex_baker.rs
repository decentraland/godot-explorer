//! Bake the atlas layer index into a Mesh's `CUSTOM0` vertex attribute.

use godot::classes::mesh::{ArrayType, PrimitiveType};
use godot::classes::{ArrayMesh, Mesh};
use godot::prelude::*;

/// Concatenate every surface of `source` into ONE combined surface, packing
/// each source-surface's atlas-layer index into the per-vertex CUSTOM0 byte.
///
/// `layers_per_surface[i]` is the layer assigned to surface `i`. All surfaces
/// MUST be mergeable (caller checked) and share the same vertex feature
/// signature — same set of attributes (vertex, normal, uv, color, index).
/// Returns None if any surface lacks vertex/index or attribute shapes diverge.
///
/// Index offsets are accumulated so each surface's indices remap into the
/// global vertex pool. Surfaces with no index buffer are rejected for v1.
///
/// Output mesh has 1 surface — N source draws collapse to 1 draw call.
pub fn bake_combined_surfaces(
    source: &Gd<Mesh>,
    layers_per_surface: &[u32],
) -> Option<Gd<ArrayMesh>> {
    let surface_count = source.get_surface_count() as usize;
    if surface_count == 0 || surface_count != layers_per_surface.len() {
        return None;
    }

    let mut combined_verts = PackedVector3Array::new();
    let mut combined_normals = PackedVector3Array::new();
    let mut combined_uvs = PackedVector2Array::new();
    let mut combined_colors = PackedColorArray::new();
    let mut combined_custom0 = PackedByteArray::new();
    let mut combined_indices = PackedInt32Array::new();
    let mut has_normals = false;
    let mut has_uvs = false;
    let mut has_colors = false;

    let mut vertex_offset: i32 = 0;

    for surf_idx in 0..surface_count {
        let arrays = source.surface_get_arrays(surf_idx as i32);
        if arrays.len() <= ArrayType::INDEX.ord() as usize {
            return None;
        }

        let verts_v = arrays.at(ArrayType::VERTEX.ord() as usize);
        let verts: PackedVector3Array = verts_v.try_to().ok()?;
        if verts.is_empty() {
            continue;
        }
        let v_count = verts.len();

        let layer_byte = layers_per_surface[surf_idx].min(255) as u8;

        for v in verts.as_slice() {
            combined_verts.push(*v);
        }

        let normals_v = arrays.at(ArrayType::NORMAL.ord() as usize);
        if normals_v.get_type() != VariantType::NIL {
            if let Ok(normals) = normals_v.try_to::<PackedVector3Array>() {
                if normals.len() == v_count {
                    for n in normals.as_slice() {
                        combined_normals.push(*n);
                    }
                    has_normals = true;
                }
            }
        } else if has_normals {
            // Mid-stream signature mismatch: bail to avoid corrupt vertex layout.
            return None;
        }

        let uvs_v = arrays.at(ArrayType::TEX_UV.ord() as usize);
        if uvs_v.get_type() != VariantType::NIL {
            if let Ok(uvs) = uvs_v.try_to::<PackedVector2Array>() {
                if uvs.len() == v_count {
                    for u in uvs.as_slice() {
                        combined_uvs.push(*u);
                    }
                    has_uvs = true;
                }
            }
        } else if has_uvs {
            return None;
        }

        let colors_v = arrays.at(ArrayType::COLOR.ord() as usize);
        if colors_v.get_type() != VariantType::NIL {
            if let Ok(colors) = colors_v.try_to::<PackedColorArray>() {
                if colors.len() == v_count {
                    for c in colors.as_slice() {
                        combined_colors.push(*c);
                    }
                    has_colors = true;
                }
            }
        } else if has_colors {
            return None;
        }

        for _ in 0..v_count {
            combined_custom0.push(layer_byte);
            combined_custom0.push(0);
            combined_custom0.push(0);
            combined_custom0.push(0);
        }

        let indices_v = arrays.at(ArrayType::INDEX.ord() as usize);
        let indices: PackedInt32Array = indices_v.try_to().ok()?;
        for idx in indices.as_slice() {
            combined_indices.push(*idx + vertex_offset);
        }

        vertex_offset += v_count as i32;
    }

    if combined_verts.is_empty() {
        return None;
    }

    let mut new_arrays = VariantArray::new();
    new_arrays.resize(ArrayType::MAX.ord() as usize, &Variant::nil());
    new_arrays.set(
        ArrayType::VERTEX.ord() as usize,
        &combined_verts.to_variant(),
    );
    if has_normals {
        new_arrays.set(
            ArrayType::NORMAL.ord() as usize,
            &combined_normals.to_variant(),
        );
    }
    if has_uvs {
        new_arrays.set(ArrayType::TEX_UV.ord() as usize, &combined_uvs.to_variant());
    }
    if has_colors {
        new_arrays.set(
            ArrayType::COLOR.ord() as usize,
            &combined_colors.to_variant(),
        );
    }
    new_arrays.set(
        ArrayType::CUSTOM0.ord() as usize,
        &combined_custom0.to_variant(),
    );
    new_arrays.set(
        ArrayType::INDEX.ord() as usize,
        &combined_indices.to_variant(),
    );

    let mut out = ArrayMesh::new_gd();
    out.add_surface_from_arrays(PrimitiveType::TRIANGLES, &new_arrays);
    Some(out)
}

pub fn bake_layer_into_custom0(source: &Gd<Mesh>, layer: u32) -> Option<Gd<ArrayMesh>> {
    if source.get_surface_count() < 1 {
        return None;
    }

    let arrays = source.surface_get_arrays(0);
    if arrays.len() <= ArrayType::INDEX.ord() as usize {
        return None;
    }

    let verts_v = arrays.at(ArrayType::VERTEX.ord() as usize);
    let verts: PackedVector3Array = verts_v.try_to().ok()?;
    if verts.is_empty() {
        return None;
    }

    // CUSTOM0 in Godot meshes defaults to ARRAY_CUSTOM_RGBA8_UNORM, which
    // expects a `PackedByteArray` of (verts × 4) bytes — RGBA per vertex,
    // each component normalized 0..1 in the vertex shader. Pack the atlas
    // layer index (0..255) into the R channel and read it back in the
    // shader as `CUSTOM0.x * 255.0`. We capped the atlas at 256 layers
    // upstream (atlas.rs CELL_COUNT / state.rs), so 1 byte suffices.
    // Earlier we used PackedFloat32Array here and called add_surface_from_arrays
    // without flags; Godot then refused to register the surface because the
    // default CUSTOM0 format is BYTE and the inferred non-NIL slot bit doesn't
    // override the channel type. Result: "Invalid array format for surface"
    // spam plus a non-existent surface 0 that broke the override-material call.
    let layer_byte = layer.min(255) as u8;
    let mut custom0 = PackedByteArray::new();
    custom0.resize(verts.len() * 4);
    let custom0_slice = custom0.as_mut_slice();
    for i in 0..verts.len() {
        let base = i * 4;
        custom0_slice[base] = layer_byte;
        // [base + 1..=base + 3] left at 0 (resize zero-initializes).
    }

    let mut new_arrays = arrays.clone();
    new_arrays.set(ArrayType::CUSTOM0.ord() as usize, &custom0.to_variant());

    let mut out = ArrayMesh::new_gd();
    out.add_surface_from_arrays(PrimitiveType::TRIANGLES, &new_arrays);
    Some(out)
}
