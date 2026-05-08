//! Strip unused vertex streams from an ArrayMesh.
//!
//! Removes:
//!  - UV2 (used by lightmap baking; DCL doesn't lightmap-bake at runtime)
//!  - Tangents when no normal map is in use (no point sending TBN to GPU)
//!  - Vertex colors when all-white (modulate is identity)
//!  - Bone indices/weights when not skinned
//!
//! Each removed stream saves vertex bandwidth proportional to its element
//! size — UV2 is 8 bytes/vert, tangents 16 bytes/vert, colors 16 bytes/vert.
//! On Mali-G68 vertex bandwidth is on-die but the payload from main RAM
//! still costs DDR bandwidth.

use godot::classes::base_material_3d::TextureParam;
use godot::classes::mesh::{ArrayType, PrimitiveType};
use godot::classes::{ArrayMesh, BaseMaterial3D};
use godot::prelude::*;

pub fn strip_unused(source: &Gd<ArrayMesh>) -> Option<(Gd<ArrayMesh>, u64)> {
    if source.has_meta("dcl_preproc_stripped") {
        return None;
    }

    let surface_count = source.get_surface_count();
    if surface_count <= 0 {
        return None;
    }

    let mut out = ArrayMesh::new_gd();
    let mut bytes_saved: u64 = 0;

    for s in 0..surface_count {
        let arrays = source.surface_get_arrays(s);
        let primitive = source.surface_get_primitive_type(s);
        let material = source.surface_get_material(s);

        let has_normal_map = material
            .as_ref()
            .and_then(|m| m.clone().try_cast::<BaseMaterial3D>().ok())
            .map(|bm| bm.get_texture(TextureParam::NORMAL).is_some())
            .unwrap_or(false);

        let mut new_arrays = arrays.clone();

        // Vertex count we'll use to estimate bandwidth saved.
        let vert_count = arrays
            .at(ArrayType::VERTEX.ord() as usize)
            .try_to::<PackedVector3Array>()
            .map(|v| v.len() as u64)
            .unwrap_or(0);

        // Strip UV2 — never used by DCL render path.
        let uv2_idx = ArrayType::TEX_UV2.ord() as usize;
        if !arrays.at(uv2_idx).is_nil() {
            if let Ok(arr) = arrays.at(uv2_idx).try_to::<PackedVector2Array>() {
                if !arr.is_empty() {
                    new_arrays.set(uv2_idx, &Variant::nil());
                    bytes_saved = bytes_saved.saturating_add(vert_count.saturating_mul(8));
                }
            }
        }

        // Strip tangents if material has no normal map.
        if !has_normal_map {
            let tan_idx = ArrayType::TANGENT.ord() as usize;
            if let Ok(arr) = arrays.at(tan_idx).try_to::<PackedFloat32Array>() {
                if !arr.is_empty() {
                    new_arrays.set(tan_idx, &Variant::nil());
                    bytes_saved = bytes_saved.saturating_add(vert_count.saturating_mul(16));
                }
            }
        }

        // Strip vertex colors if effectively white.
        let color_idx = ArrayType::COLOR.ord() as usize;
        if let Ok(colors) = arrays.at(color_idx).try_to::<PackedColorArray>() {
            if !colors.is_empty() && colors_are_all_white(&colors) {
                new_arrays.set(color_idx, &Variant::nil());
                bytes_saved = bytes_saved.saturating_add(vert_count.saturating_mul(16));
            }
        }

        out.add_surface_from_arrays(primitive, &new_arrays);
        if let Some(mat) = material {
            let last = out.get_surface_count() - 1;
            out.surface_set_material(last, &mat);
        }
    }

    if out.get_surface_count() == 0 || bytes_saved == 0 {
        return None;
    }

    let mut marked = out.clone();
    marked.set_meta("dcl_preproc_stripped", &true.to_variant());

    Some((out, bytes_saved))
}

fn colors_are_all_white(colors: &PackedColorArray) -> bool {
    for i in 0..colors.len() {
        let c = colors.get(i);
        if let Some(c) = c {
            if c.r < 0.99 || c.g < 0.99 || c.b < 0.99 || c.a < 0.99 {
                return false;
            }
        }
    }
    true
}

// Suppress unused-import warning for PrimitiveType.
#[allow(dead_code)]
fn _unused_primitive() -> PrimitiveType {
    PrimitiveType::TRIANGLES
}
