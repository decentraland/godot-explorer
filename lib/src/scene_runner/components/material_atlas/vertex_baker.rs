//! Bake the atlas layer index into a Mesh's `CUSTOM0` vertex attribute.

use godot::classes::mesh::{ArrayType, PrimitiveType};
use godot::classes::{ArrayMesh, Mesh};
use godot::prelude::*;

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

    let layer_f = layer as f32;
    let mut custom0 = PackedFloat32Array::new();
    custom0.resize(verts.len() * 4);
    let custom0_slice = custom0.as_mut_slice();
    for i in 0..verts.len() {
        let base = i * 4;
        custom0_slice[base] = layer_f;
        custom0_slice[base + 1] = 0.0;
        custom0_slice[base + 2] = 0.0;
        custom0_slice[base + 3] = 0.0;
    }

    let mut new_arrays = arrays.clone();
    new_arrays.set(ArrayType::CUSTOM0.ord() as usize, &custom0.to_variant());

    let mut out = ArrayMesh::new_gd();
    // Pass empty blend shapes + LOD list, no flags. Godot infers CUSTOM0
    // format from the array shape (4 floats per vertex → RGBA_FLOAT).
    out.add_surface_from_arrays(PrimitiveType::TRIANGLES, &new_arrays);
    Some(out)
}
