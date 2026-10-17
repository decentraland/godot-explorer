use std::collections::HashMap;

use godot::classes::multi_mesh::TransformFormat;
use godot::classes::{
    mesh::{ArrayType, PrimitiveType},
    ArrayMesh, Material, MultiMesh, MultiMeshInstance3D,
};
use godot::prelude::*;

const FLOOR_SIZE: f32 = 16.0;
const FLOOR_MATERIAL: &str = "res://assets/empty-scenes/base_floor_material.tres";

/// One quad per parcel of every loaded scene, drawn as a single multimesh.
#[derive(GodotClass)]
#[class(init, base=Node3D)]
pub struct BaseFloorManager {
    base: Base<Node3D>,
    floor: Option<Gd<MultiMeshInstance3D>>,
    scene_parcels: HashMap<String, Vec<Vector2i>>,
}

#[godot_api]
impl INode3D for BaseFloorManager {
    fn ready(&mut self) {
        let mut multi_mesh = MultiMesh::new_gd();
        multi_mesh.set_transform_format(TransformFormat::TRANSFORM_3D);
        multi_mesh.set_mesh(&Self::floor_quad());
        multi_mesh.set_instance_count(0);
        let mut floor = MultiMeshInstance3D::new_alloc();
        floor.set_name("BaseFloorMultimesh");
        floor.set_multimesh(&multi_mesh);
        self.base_mut().add_child(&floor);
        self.floor = Some(floor);
    }
}

impl BaseFloorManager {
    fn floor_quad() -> Gd<ArrayMesh> {
        let half = FLOOR_SIZE / 2.0;
        let mut arrays = VarArray::new();
        arrays.resize(ArrayType::MAX.ord() as usize, &Variant::nil());
        arrays.set(
            ArrayType::VERTEX.ord() as usize,
            &PackedVector3Array::from(&[
                Vector3::new(-half, 0.0, -half),
                Vector3::new(half, 0.0, -half),
                Vector3::new(half, 0.0, half),
                Vector3::new(-half, 0.0, half),
            ])
            .to_variant(),
        );
        arrays.set(
            ArrayType::TEX_UV.ord() as usize,
            &PackedVector2Array::from(&[
                Vector2::new(0.0, 0.0),
                Vector2::new(1.0, 0.0),
                Vector2::new(1.0, 1.0),
                Vector2::new(0.0, 1.0),
            ])
            .to_variant(),
        );
        arrays.set(
            ArrayType::NORMAL.ord() as usize,
            &PackedVector3Array::from(&[Vector3::UP; 4]).to_variant(),
        );
        arrays.set(
            ArrayType::INDEX.ord() as usize,
            &PackedInt32Array::from(&[0, 1, 2, 0, 2, 3]).to_variant(),
        );
        let mut mesh = ArrayMesh::new_gd();
        mesh.add_surface_from_arrays(PrimitiveType::TRIANGLES, &arrays);
        if let Ok(material) = try_load::<Material>(FLOOR_MATERIAL) {
            mesh.surface_set_material(0, &material);
        }
        mesh
    }

    pub fn add_scene_floors(&mut self, scene_id: &str, parcels: &[Vector2i]) {
        if self.floor.is_none() || self.scene_parcels.contains_key(scene_id) {
            return;
        }
        self.scene_parcels
            .insert(scene_id.to_string(), parcels.to_vec());
        self.rebuild();
    }

    pub fn remove_scene_floors(&mut self, scene_id: &str) {
        if self.scene_parcels.remove(scene_id).is_some() {
            self.rebuild();
        }
    }

    pub fn clear_all_floors(&mut self) {
        self.scene_parcels.clear();
        self.rebuild();
    }

    fn rebuild(&mut self) {
        let Some(floor) = &mut self.floor else {
            return;
        };
        let Some(mut multi_mesh) = floor.get_multimesh() else {
            return;
        };
        let total: usize = self.scene_parcels.values().map(Vec::len).sum();
        multi_mesh.set_instance_count(total as i32);
        for (index, parcel) in self.scene_parcels.values().flatten().enumerate() {
            let x = parcel.x as f32 * FLOOR_SIZE + FLOOR_SIZE / 2.0;
            let z = -parcel.y as f32 * FLOOR_SIZE - FLOOR_SIZE / 2.0;
            multi_mesh.set_instance_transform(
                index as i32,
                Transform3D::new(Basis::IDENTITY, Vector3::new(x, -0.05, z)),
            );
        }
        floor.set_visible(total > 0);
    }
}
