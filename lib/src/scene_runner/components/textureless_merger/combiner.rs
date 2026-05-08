//! Bake N source `MeshInstance3D`s into one combined `ArrayMesh`.
//!
//! Each source contributes:
//! - vertices transformed by `(world_transform · cell_local_inverse)` so the
//!   merged mesh sits at the bucket's cell center (AABB stays cell-sized →
//!   frustum culling still works).
//! - normals transformed by `basis.inverse().transposed()`.
//! - per-vertex `COLOR` = source material's `albedo_color`.
//! - indices offset by the running vertex count, so each source's
//!   intra-mesh sharing is preserved (no 6× bloat).
//!
//! The output material is a plain `StandardMaterial3D` with
//! `vertex_color_use_as_albedo = true`.
//!
//! Intentional limits:
//! - Single-surface sources only — guaranteed by the classifier.
//! - UVs preserved as-is (no texture sampling on the merged material, but
//!   keeping UVs avoids rejection by drivers that expect a UV channel).
//! - No tangent / weights / bones channels.

use godot::classes::base_material_3d::{CullMode, Transparency};
use godot::classes::mesh::{ArrayType, PrimitiveType};
use godot::classes::{ArrayMesh, Material, MeshInstance3D, StandardMaterial3D};
use godot::prelude::*;

use super::cell_grid::cell_center;

#[derive(Debug, Clone)]
pub struct MeshPart {
    pub source_mi: Gd<MeshInstance3D>,
    pub albedo_color: Color,
    pub world_transform: Transform3D,
}

pub struct BuiltMesh {
    pub mesh: Gd<ArrayMesh>,
    pub material: Gd<Material>,
    pub vertex_count: usize,
    pub index_count: usize,
}

/// Build a merged ArrayMesh + matching material from the parts of a single
/// `(transparency, cull_mode, cell)` bucket. Returns `None` if every part has
/// an unreadable mesh (defensive — the classifier already ensures one
/// surface per source).
pub fn build_merged_mesh(
    parts: &[MeshPart],
    cell: (i32, i32),
    transparency: i32,
    cull_mode: i32,
) -> Option<BuiltMesh> {
    if parts.is_empty() {
        return None;
    }

    let cell_origin = cell_center(cell.0, cell.1);
    let mut all_verts: Vec<Vector3> = Vec::new();
    let mut all_normals: Vec<Vector3> = Vec::new();
    let mut all_uvs: Vec<Vector2> = Vec::new();
    let mut all_colors: Vec<Color> = Vec::new();
    let mut all_indices: Vec<i32> = Vec::new();

    for part in parts {
        let Some(mesh) = part.source_mi.get_mesh() else {
            continue;
        };
        if mesh.get_surface_count() < 1 {
            continue;
        }

        let arrays = mesh.surface_get_arrays(0);
        if arrays.len() <= ArrayType::INDEX.ord() as usize {
            continue;
        }

        let verts_v = arrays.at(ArrayType::VERTEX.ord() as usize);
        let Ok(verts) = verts_v.try_to::<PackedVector3Array>() else {
            continue;
        };
        if verts.is_empty() {
            continue;
        }

        // Index buffer (may be empty for non-indexed meshes — we then
        // synthesize one mapping each vertex once, which still avoids the
        // GDScript prototype's per-index expansion).
        let idx_v = arrays.at(ArrayType::INDEX.ord() as usize);
        let source_indices: PackedInt32Array = idx_v
            .try_to::<PackedInt32Array>()
            .unwrap_or_else(|_| PackedInt32Array::new());

        let normals_v = arrays.at(ArrayType::NORMAL.ord() as usize);
        let source_normals: PackedVector3Array = normals_v
            .try_to::<PackedVector3Array>()
            .unwrap_or_else(|_| PackedVector3Array::new());
        let uvs_v = arrays.at(ArrayType::TEX_UV.ord() as usize);
        let source_uvs: PackedVector2Array = uvs_v
            .try_to::<PackedVector2Array>()
            .unwrap_or_else(|_| PackedVector2Array::new());

        let xform = part.world_transform;
        let basis_it = xform.basis.inverse().transposed();

        let base_vertex = all_verts.len() as i32;

        for i in 0..verts.len() {
            let v_local = verts.get(i).unwrap_or(Vector3::ZERO);
            let v_world = xform * v_local;
            all_verts.push(v_world - cell_origin);

            let n_local = source_normals.get(i).unwrap_or(Vector3::UP);
            all_normals.push((basis_it * n_local).normalized());

            let uv = source_uvs.get(i).unwrap_or(Vector2::ZERO);
            all_uvs.push(uv);

            all_colors.push(part.albedo_color);
        }

        if source_indices.is_empty() {
            // Non-indexed: emit identity index list so the merged surface is
            // always built with an INDEX array. Cheaper to remap once than
            // to keep two surface variants alive.
            for i in 0..(verts.len() as i32) {
                all_indices.push(base_vertex + i);
            }
        } else {
            for i in 0..source_indices.len() {
                all_indices.push(base_vertex + source_indices.get(i).unwrap_or(0));
            }
        }
    }

    if all_verts.is_empty() {
        return None;
    }

    let vertex_count = all_verts.len();
    let index_count = all_indices.len();

    let mut arrays = VarArray::new();
    arrays.resize(ArrayType::MAX.ord() as usize, &Variant::nil());
    arrays.set(
        ArrayType::VERTEX.ord() as usize,
        &packed_vector3(&all_verts).to_variant(),
    );
    arrays.set(
        ArrayType::NORMAL.ord() as usize,
        &packed_vector3(&all_normals).to_variant(),
    );
    arrays.set(
        ArrayType::TEX_UV.ord() as usize,
        &packed_vector2(&all_uvs).to_variant(),
    );
    arrays.set(
        ArrayType::COLOR.ord() as usize,
        &packed_color(&all_colors).to_variant(),
    );
    arrays.set(
        ArrayType::INDEX.ord() as usize,
        &packed_int32(&all_indices).to_variant(),
    );

    let mut mesh = ArrayMesh::new_gd();
    mesh.add_surface_from_arrays(PrimitiveType::TRIANGLES, &arrays);

    let mut material = StandardMaterial3D::new_gd();
    material.set_flag(
        godot::classes::base_material_3d::Flags::ALBEDO_FROM_VERTEX_COLOR,
        true,
    );
    material.set_transparency(transparency_from_i32(transparency));
    material.set_cull_mode(cull_mode_from_i32(cull_mode));

    Some(BuiltMesh {
        mesh,
        material: material.upcast::<Material>(),
        vertex_count,
        index_count,
    })
}

fn transparency_from_i32(v: i32) -> Transparency {
    match v {
        1 => Transparency::ALPHA,
        2 => Transparency::ALPHA_SCISSOR,
        3 => Transparency::ALPHA_HASH,
        4 => Transparency::ALPHA_DEPTH_PRE_PASS,
        _ => Transparency::DISABLED,
    }
}

fn cull_mode_from_i32(v: i32) -> CullMode {
    match v {
        1 => CullMode::FRONT,
        2 => CullMode::DISABLED,
        _ => CullMode::BACK,
    }
}

fn packed_vector3(src: &[Vector3]) -> PackedVector3Array {
    let mut a = PackedVector3Array::new();
    a.resize(src.len());
    a.as_mut_slice().copy_from_slice(src);
    a
}

fn packed_vector2(src: &[Vector2]) -> PackedVector2Array {
    let mut a = PackedVector2Array::new();
    a.resize(src.len());
    a.as_mut_slice().copy_from_slice(src);
    a
}

fn packed_int32(src: &[i32]) -> PackedInt32Array {
    let mut a = PackedInt32Array::new();
    a.resize(src.len());
    a.as_mut_slice().copy_from_slice(src);
    a
}

fn packed_color(src: &[Color]) -> PackedColorArray {
    let mut a = PackedColorArray::new();
    a.resize(src.len());
    a.as_mut_slice().copy_from_slice(src);
    a
}
