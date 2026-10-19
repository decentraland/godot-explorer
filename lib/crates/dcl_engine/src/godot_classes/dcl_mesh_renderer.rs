//! The `MeshRenderer` component's mesh: a box, plane, cylinder or sphere of unit size, with the
//! SDK's UVs.

use std::f32::consts::PI;
use std::sync::OnceLock;

use godot::classes::mesh::{ArrayType, PrimitiveType};
use godot::classes::{ArrayMesh, IMeshInstance3D, MeshInstance3D, SphereMesh};
use godot::prelude::*;

/// Vertex data of a primitive, before it becomes Godot arrays.
#[derive(Clone, Default)]
pub struct PrimitiveArrays {
    pub vertices: Vec<Vector3>,
    pub normals: Vec<Vector3>,
    pub uvs: Vec<Vector2>,
    pub indices: Vec<i32>,
}

/// A plane facing +Z with a back face, UVs matching Unity's layout with Y inverted for Godot.
pub fn build_plane() -> PrimitiveArrays {
    let h = 0.5;
    PrimitiveArrays {
        vertices: vec![
            Vector3::new(-h, -h, 0.0),
            Vector3::new(-h, h, 0.0),
            Vector3::new(h, h, 0.0),
            Vector3::new(h, -h, 0.0),
            Vector3::new(h, -h, 0.0),
            Vector3::new(h, h, 0.0),
            Vector3::new(-h, h, 0.0),
            Vector3::new(-h, -h, 0.0),
        ],
        normals: [Vector3::new(0.0, 0.0, 1.0); 4]
            .into_iter()
            .chain([Vector3::new(0.0, 0.0, -1.0); 4])
            .collect(),
        uvs: vec![
            Vector2::new(0.0, 1.0),
            Vector2::new(0.0, 0.0),
            Vector2::new(1.0, 0.0),
            Vector2::new(1.0, 1.0),
            Vector2::new(1.0, 1.0),
            Vector2::new(1.0, 0.0),
            Vector2::new(0.0, 0.0),
            Vector2::new(0.0, 1.0),
        ],
        indices: vec![0, 1, 2, 2, 3, 0, 4, 5, 6, 6, 7, 4],
    }
}

/// A unit cube: four vertices per face (top, bottom, left, right, front, back), UVs per face as
/// Unity's, `1 - U` for Godot.
pub fn build_cube() -> PrimitiveArrays {
    let d = 1.0_f32;
    let right = Vector3::RIGHT * d;
    let back = Vector3::BACK * d;
    let down = Vector3::DOWN * d;
    let mut vertices = Vec::with_capacity(24);
    let mut face = |start: Vector3, a: Vector3, b: Vector3| {
        vertices.extend([start, start + a, start + a + b, start + b]);
    };
    face(Vector3::new(-d / 2.0, d / 2.0, -d / 2.0), right, back);
    face(Vector3::new(-d / 2.0, -d / 2.0, -d / 2.0), right, back);
    face(Vector3::new(-d / 2.0, d / 2.0, -d / 2.0), back, down);
    face(Vector3::new(d / 2.0, d / 2.0, -d / 2.0), back, down);
    face(Vector3::new(-d / 2.0, d / 2.0, -d / 2.0), right, down);
    face(Vector3::new(-d / 2.0, d / 2.0, d / 2.0), right, down);

    // Per face, in vertex order.
    let up_flip = [(0.0, 1.0), (0.0, 0.0), (1.0, 0.0), (1.0, 1.0)];
    let down_flip = [(0.0, 0.0), (0.0, 1.0), (1.0, 1.0), (1.0, 0.0)];
    let front = [(1.0, 0.0), (0.0, 0.0), (0.0, 1.0), (1.0, 1.0)];
    let back_face = [(1.0, 1.0), (0.0, 1.0), (0.0, 0.0), (1.0, 0.0)];
    let uvs = [up_flip, down_flip, up_flip, down_flip, front, back_face]
        .iter()
        .flatten()
        .map(|(u, v)| Vector2::new(*u, *v))
        .collect();

    let normals = [
        Vector3::UP,
        Vector3::DOWN,
        Vector3::LEFT,
        Vector3::RIGHT,
        Vector3::FORWARD,
        Vector3::BACK,
    ]
    .iter()
    .flat_map(|n| [*n; 4])
    .collect();

    let mut indices = Vec::with_capacity(36);
    for (face_index, reversed) in [false, true, false, true, true, false]
        .into_iter()
        .enumerate()
    {
        let base = face_index as i32 * 4;
        if reversed {
            indices.extend([base, base + 2, base + 1, base, base + 3, base + 2]);
        } else {
            indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }
    PrimitiveArrays {
        vertices,
        normals,
        uvs,
        indices,
    }
}

const CYLINDER_SEGMENTS: usize = 50;

/// A cylinder (or a cone when a radius is zero) of height one centered at the origin: the side
/// with a doubled seam vertex, then the top and bottom discs.
pub fn build_cylinder(radius_top: f32, radius_bottom: f32) -> PrimitiveArrays {
    let n = CYLINDER_SEGMENTS;
    let plus_one = n + 1;
    let length = 1.0_f32;
    let offset = Vector3::new(0.0, -0.5, 0.0);
    let total = 2 * plus_one + (n + 1) + (n + 1);
    let mut vertices = vec![Vector3::ZERO; total];
    let mut normals = vec![Vector3::ZERO; total];
    let mut uvs = vec![Vector2::ZERO; total];

    let slope = ((radius_bottom - radius_top) / length).atan();
    let slope_sin = -slope.sin();
    let slope_cos = slope.cos();

    for i in 0..n {
        let angle = 2.0 * PI * i as f32 / n as f32;
        let (angle_sin, angle_cos) = (-angle.sin(), angle.cos());
        let half = 2.0 * PI * (i as f32 + 0.5) / n as f32;
        let (half_sin, half_cos) = (-half.sin(), half.cos());

        vertices[i] = Vector3::new(radius_top * angle_cos, length, radius_top * angle_sin) + offset;
        vertices[i + plus_one] =
            Vector3::new(radius_bottom * angle_cos, 0.0, radius_bottom * angle_sin) + offset;

        normals[i] = if radius_top == 0.0 {
            Vector3::new(half_cos * slope_cos, -slope_sin, half_sin * slope_cos)
        } else {
            Vector3::new(angle_cos * slope_cos, -slope_sin, angle_sin * slope_cos)
        };
        normals[i + plus_one] = if radius_bottom == 0.0 {
            Vector3::new(half_cos * slope_cos, -slope_sin, half_sin * slope_cos)
        } else {
            Vector3::new(angle_cos * slope_cos, -slope_sin, angle_sin * slope_cos)
        };
        uvs[i] = Vector2::new(1.0 - i as f32 / n as f32, 1.0);
        uvs[i + plus_one] = Vector2::new(1.0 - i as f32 / n as f32, 0.0);
    }
    // The seam closes the side on its first vertex.
    vertices[n] = vertices[0];
    vertices[n + plus_one] = vertices[plus_one];
    uvs[n] = Vector2::new(0.0, 1.0);
    uvs[n + plus_one] = Vector2::new(0.0, 0.0);
    normals[n] = normals[0];
    normals[n + plus_one] = normals[plus_one];

    let top_start = 2 * plus_one;
    let top_end = top_start + n;
    for i in 0..n {
        let angle = 2.0 * PI * i as f32 / n as f32;
        let (angle_sin, angle_cos) = (-angle.sin(), angle.cos());
        vertices[top_start + i] =
            Vector3::new(radius_top * angle_cos, length, radius_top * angle_sin) + offset;
        normals[top_start + i] = Vector3::UP;
        uvs[top_start + i] = Vector2::new(angle_cos / 2.0 + 0.5, angle_sin / 2.0 + 0.5);
    }
    vertices[top_end] = Vector3::new(0.0, length, 0.0) + offset;
    normals[top_end] = Vector3::UP;
    uvs[top_end] = Vector2::new(0.5, 0.5);

    let bottom_start = top_start + n + 1;
    let bottom_end = bottom_start + n;
    for i in 0..n {
        let angle = 2.0 * PI * i as f32 / n as f32;
        let (angle_sin, angle_cos) = (-angle.sin(), angle.cos());
        vertices[bottom_start + i] =
            Vector3::new(radius_bottom * angle_cos, 0.0, radius_bottom * angle_sin) + offset;
        normals[bottom_start + i] = Vector3::DOWN;
        uvs[bottom_start + i] = Vector2::new(angle_cos / 2.0 + 0.5, angle_sin / 2.0 + 0.5);
    }
    vertices[bottom_end] = offset;
    normals[bottom_end] = Vector3::DOWN;
    uvs[bottom_end] = Vector2::new(0.5, 0.5);

    let mut indices: Vec<i32> = Vec::new();
    let (n_i, plus_one_i) = (n as i32, plus_one as i32);
    if radius_top == 0.0 {
        for i in 0..n_i {
            indices.extend([i + plus_one_i, i, i + 1 + plus_one_i]);
        }
    } else if radius_bottom == 0.0 {
        for i in 0..n_i {
            indices.extend([i, i + 1, i + plus_one_i]);
        }
    } else {
        for i in 0..n_i {
            let ip1 = i + 1;
            indices.extend([
                i,
                ip1,
                i + plus_one_i,
                ip1 + plus_one_i,
                i + plus_one_i,
                ip1,
            ]);
        }
    }
    for i in 0..n {
        let mut next = top_start + i + 1;
        if next == top_end {
            next = top_start;
        }
        indices.extend([next as i32, (top_start + i) as i32, top_end as i32]);
    }
    for i in 0..n {
        let mut next = bottom_start + i + 1;
        if next == bottom_end {
            next = bottom_start;
        }
        indices.extend([bottom_end as i32, (bottom_start + i) as i32, next as i32]);
    }
    PrimitiveArrays {
        vertices,
        normals,
        uvs,
        indices,
    }
}

fn cached_plane() -> &'static PrimitiveArrays {
    static PLANE: OnceLock<PrimitiveArrays> = OnceLock::new();
    PLANE.get_or_init(build_plane)
}

fn cached_cube() -> &'static PrimitiveArrays {
    static CUBE: OnceLock<PrimitiveArrays> = OnceLock::new();
    CUBE.get_or_init(build_cube)
}

/// Replaces the first `min(uvs / 2, 8)` UVs with the SDK's; `flip` maps the SDK's V.
fn with_sdk_uvs(
    base: &PrimitiveArrays,
    sdk_uvs: &[f32],
    flip: impl Fn(f32) -> f32,
) -> PrimitiveArrays {
    let mut arrays = base.clone();
    let count = (sdk_uvs.len() / 2).min(8);
    for i in 0..count {
        arrays.uvs[i] = Vector2::new(sdk_uvs[i * 2], flip(sdk_uvs[i * 2 + 1]));
    }
    arrays
}

/// The box's arrays for the SDK UVs (V negated, as Unity's layout needs).
pub fn box_arrays(sdk_uvs: &[f32]) -> PrimitiveArrays {
    with_sdk_uvs(cached_cube(), sdk_uvs, |v| -v)
}

/// The plane's arrays for the SDK UVs (V inverted).
pub fn plane_arrays(sdk_uvs: &[f32]) -> PrimitiveArrays {
    with_sdk_uvs(cached_plane(), sdk_uvs, |v| 1.0 - v)
}

fn to_godot_arrays(arrays: &PrimitiveArrays) -> VarArray {
    let mut result = VarArray::new();
    result.resize(ArrayType::MAX.ord() as usize, &Variant::nil());
    let packed_vertices: PackedVector3Array = arrays.vertices.iter().copied().collect();
    let packed_normals: PackedVector3Array = arrays.normals.iter().copied().collect();
    let packed_uvs: PackedVector2Array = arrays.uvs.iter().copied().collect();
    let packed_indices: PackedInt32Array = arrays.indices.iter().copied().collect();
    result.set(
        ArrayType::VERTEX.ord() as usize,
        &packed_vertices.to_variant(),
    );
    result.set(
        ArrayType::NORMAL.ord() as usize,
        &packed_normals.to_variant(),
    );
    result.set(ArrayType::TEX_UV.ord() as usize, &packed_uvs.to_variant());
    result.set(
        ArrayType::INDEX.ord() as usize,
        &packed_indices.to_variant(),
    );
    result
}

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
enum MeshRendererPrimitive {
    #[default]
    None,
    Box,
    Cylinder,
    Sphere,
    Plane,
}

#[derive(GodotClass)]
#[class(init, base=MeshInstance3D)]
pub struct DclMeshRenderer {
    current: MeshRendererPrimitive,
    base: Base<MeshInstance3D>,
}

#[godot_api]
impl IMeshInstance3D for DclMeshRenderer {}

impl DclMeshRenderer {
    /// Keeps the `ArrayMesh` while the primitive is the same, clearing its surface.
    fn array_mesh_for(&mut self, primitive: MeshRendererPrimitive) -> Gd<ArrayMesh> {
        if self.current != primitive || self.base().get_mesh().is_none() {
            self.current = primitive;
            let mesh = ArrayMesh::new_gd();
            self.base_mut().set_mesh(&mesh);
            return mesh;
        }
        match self
            .base()
            .get_mesh()
            .and_then(|m| m.try_cast::<ArrayMesh>().ok())
        {
            Some(mut mesh) => {
                mesh.clear_surfaces();
                mesh
            }
            None => {
                let mesh = ArrayMesh::new_gd();
                self.base_mut().set_mesh(&mesh);
                mesh
            }
        }
    }

    fn add_surface(mesh: &mut Gd<ArrayMesh>, arrays: &PrimitiveArrays) {
        mesh.add_surface_from_arrays(PrimitiveType::TRIANGLES, &to_godot_arrays(arrays));
    }

    pub fn set_box(&mut self, sdk_uvs: &[f32]) {
        let mut mesh = self.array_mesh_for(MeshRendererPrimitive::Box);
        Self::add_surface(&mut mesh, &box_arrays(sdk_uvs));
    }

    pub fn set_plane(&mut self, sdk_uvs: &[f32]) {
        let mut mesh = self.array_mesh_for(MeshRendererPrimitive::Plane);
        Self::add_surface(&mut mesh, &plane_arrays(sdk_uvs));
    }

    pub fn set_cylinder(&mut self, top_radius: f32, bottom_radius: f32) {
        let mut mesh = self.array_mesh_for(MeshRendererPrimitive::Cylinder);
        Self::add_surface(&mut mesh, &build_cylinder(top_radius, bottom_radius));
    }

    pub fn set_sphere(&mut self) {
        if self.current != MeshRendererPrimitive::Sphere {
            self.current = MeshRendererPrimitive::Sphere;
            self.base_mut().set_mesh(&SphereMesh::new_gd());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn indices_in_range(arrays: &PrimitiveArrays) -> bool {
        arrays
            .indices
            .iter()
            .all(|i| *i >= 0 && (*i as usize) < arrays.vertices.len())
    }

    fn consistent(arrays: &PrimitiveArrays) -> bool {
        arrays.vertices.len() == arrays.normals.len() && arrays.vertices.len() == arrays.uvs.len()
    }

    #[test]
    fn plane_has_two_sided_quads() {
        let plane = build_plane();
        assert_eq!(plane.vertices.len(), 8);
        assert_eq!(plane.indices.len(), 12);
        assert!(consistent(&plane) && indices_in_range(&plane));
    }

    #[test]
    fn cube_has_four_vertices_per_face() {
        let cube = build_cube();
        assert_eq!(cube.vertices.len(), 24);
        assert_eq!(cube.indices.len(), 36);
        assert!(consistent(&cube) && indices_in_range(&cube));
        // Unit size, centered.
        for v in &cube.vertices {
            assert!(v.x.abs() <= 0.5 + 1e-6 && v.y.abs() <= 0.5 + 1e-6 && v.z.abs() <= 0.5 + 1e-6);
        }
    }

    #[test]
    fn cylinder_arrays_are_consistent_for_cylinder_and_cones() {
        for (top, bottom) in [(0.5, 0.5), (0.0, 0.5), (0.5, 0.0), (0.25, 1.0)] {
            let cylinder = build_cylinder(top, bottom);
            assert!(consistent(&cylinder), "top {top} bottom {bottom}");
            assert!(indices_in_range(&cylinder), "top {top} bottom {bottom}");
            assert_eq!(cylinder.indices.len() % 3, 0);
        }
        assert_eq!(build_cylinder(0.5, 0.5).vertices.len(), 2 * 51 + 51 + 51);
    }

    #[test]
    fn cylinder_normals_are_unit_length() {
        for normal in build_cylinder(0.5, 0.5).normals {
            assert!((normal.length() - 1.0).abs() < 1e-4);
        }
    }

    #[test]
    fn sdk_uvs_replace_the_first_vertices_and_flip_v() {
        let uvs = [0.25, 0.75, 0.5, 0.5];
        let plane = plane_arrays(&uvs);
        assert_eq!(plane.uvs[0], Vector2::new(0.25, 0.25));
        assert_eq!(plane.uvs[1], Vector2::new(0.5, 0.5));
        assert_eq!(plane.uvs[2], build_plane().uvs[2]);
        let cube = box_arrays(&uvs);
        assert_eq!(cube.uvs[0], Vector2::new(0.25, -0.75));
    }

    #[test]
    fn more_than_eight_uvs_are_ignored() {
        let many: Vec<f32> = (0..40).map(|i| i as f32).collect();
        let cube = box_arrays(&many);
        assert_eq!(cube.uvs.len(), 24);
        assert_eq!(cube.uvs[8], build_cube().uvs[8]);
    }
}
