use crate::{
    dcl::{
        components::{
            proto_components::sdk::components::{pb_mesh_collider, PbMeshCollider},
            SceneComponentId,
        },
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    scene_runner::scene::Scene,
};
use godot::{
    classes::{
        mesh::PrimitiveType,
        physics_server_3d::BodyMode,
        ArrayMesh, BoxShape3D, CollisionShape3D, CylinderShape3D, PhysicsServer3D, Shape3D,
        SphereShape3D, StaticBody3D,
    },
    obj::Singleton,
    prelude::*,
};
use num_traits::Zero;

fn build_cylinder_arrays(radius_top: f32, radius_bottom: f32) -> VarArray {
    let mut uvs_array = PackedVector2Array::new();
    let mut vertices_array = PackedVector3Array::new();
    let mut normals_array = PackedVector3Array::new();
    let mut triangles_array = PackedInt32Array::new();
    let num_vertices = 10;
    let length = 1.0;
    let offset_pos = Vector3::new(0.0, -0.5, 0.0);
    let num_vertices_plus_one = num_vertices + 1;

    vertices_array.resize(2 * num_vertices_plus_one + (num_vertices + 1) + (num_vertices + 1));
    normals_array.resize(2 * num_vertices_plus_one + (num_vertices + 1) + (num_vertices + 1));
    uvs_array.resize(2 * num_vertices_plus_one + (num_vertices + 1) + (num_vertices + 1));

    let uvs = uvs_array.as_mut_slice();
    let vertices = vertices_array.as_mut_slice();
    let normals = normals_array.as_mut_slice();

    let slope = ((radius_bottom - radius_top) / length).atan();
    let slope_sin = -slope.sin();
    let slope_cos = slope.sin();

    for i in 0..num_vertices {
        let angle = 2.0 * std::f32::consts::PI * i as f32 / num_vertices as f32;
        let angle_sin = -angle.sin();
        let angle_cos = angle.cos();
        let angle_half = 2.0 * std::f32::consts::PI * (i as f32 + 0.5) / num_vertices as f32;
        let angle_half_sin = -angle_half.sin();
        let angle_half_cos = angle_half.cos();

        vertices[i] =
            Vector3::new(radius_top * angle_cos, length, radius_top * angle_sin) + offset_pos;
        vertices[i + num_vertices_plus_one] =
            Vector3::new(radius_bottom * angle_cos, 0.0, radius_bottom * angle_sin) + offset_pos;

        if radius_top.is_zero() {
            normals[i] = Vector3::new(
                angle_half_cos * slope_cos,
                -slope_sin,
                angle_half_sin * slope_cos,
            );
        } else {
            normals[i] = Vector3::new(angle_cos * slope_cos, -slope_sin, angle_sin * slope_cos);
        }

        if radius_bottom.is_zero() {
            normals[i + num_vertices_plus_one] = Vector3::new(
                angle_half_cos * slope_cos,
                -slope_sin,
                angle_half_sin * slope_cos,
            );
        } else {
            normals[i + num_vertices_plus_one] =
                Vector3::new(angle_cos * slope_cos, -slope_sin, angle_sin * slope_cos);
        }

        uvs[i] = Vector2::new(1.0 - 1.0 * i as f32 / num_vertices as f32, 1.0);
        uvs[i + num_vertices_plus_one] =
            Vector2::new(1.0 - 1.0 * i as f32 / num_vertices as f32, 0.0);
    }

    vertices[num_vertices] = vertices[0];
    vertices[num_vertices + num_vertices_plus_one] = vertices[num_vertices_plus_one];
    uvs[num_vertices] = Vector2::new(1.0 - 1.0 * num_vertices as f32 / num_vertices as f32, 1.0);
    uvs[num_vertices + num_vertices_plus_one] =
        Vector2::new(1.0 - 1.0 * num_vertices as f32 / num_vertices as f32, 0.0);
    normals[num_vertices] = normals[0];
    normals[num_vertices + num_vertices_plus_one] = normals[num_vertices_plus_one];

    let cover_top_index_start = 2 * num_vertices_plus_one;
    let cover_top_index_end = 2 * num_vertices_plus_one + num_vertices;
    for i in 0..num_vertices {
        let angle = 2.0 * std::f32::consts::PI * i as f32 / num_vertices as f32;
        let angle_sin = -angle.sin();
        let angle_cos = angle.cos();

        vertices[cover_top_index_start + i] =
            Vector3::new(radius_top * angle_cos, length, radius_top * angle_sin) + offset_pos;
        normals[cover_top_index_start + i] = Vector3::new(0.0, 1.0, 0.0);
        uvs[cover_top_index_start + i] = Vector2::new(angle_cos / 2.0 + 0.5, angle_sin / 2.0 + 0.5);
    }

    vertices[cover_top_index_start + num_vertices] = Vector3::new(0.0, length, 0.0) + offset_pos;
    normals[cover_top_index_start + num_vertices] = Vector3::new(0.0, 1.0, 0.0);
    uvs[cover_top_index_start + num_vertices] = Vector2::new(0.5, 0.5);

    let cover_bottom_index_start = cover_top_index_start + num_vertices + 1;
    let cover_bottom_index_end = cover_bottom_index_start + num_vertices;
    for i in 0..num_vertices {
        let angle = 2.0 * std::f32::consts::PI * i as f32 / num_vertices as f32;
        let angle_sin = -angle.sin();
        let angle_cos = angle.cos();

        vertices[cover_bottom_index_start + i] =
            Vector3::new(radius_bottom * angle_cos, 0.0, radius_bottom * angle_sin) + offset_pos;
        normals[cover_bottom_index_start + i] = Vector3::new(0.0, -1.0, 0.0);
        uvs[cover_bottom_index_start + i] =
            Vector2::new(angle_cos / 2.0 + 0.5, angle_sin / 2.0 + 0.5);
    }

    vertices[cover_bottom_index_start + num_vertices] = Vector3::new(0.0, 0.0, 0.0) + offset_pos;
    normals[cover_bottom_index_start + num_vertices] = Vector3::new(0.0, -1.0, 0.0);
    uvs[cover_bottom_index_start + num_vertices] = Vector2::new(0.5, 0.5);

    if radius_top.is_zero() || radius_bottom.is_zero() {
        triangles_array.resize(num_vertices_plus_one * 3 + num_vertices * 3 + num_vertices * 3);
    } else {
        triangles_array.resize(num_vertices_plus_one * 6 + num_vertices * 3 + num_vertices * 3);
    }
    let triangles = triangles_array.as_mut_slice();

    let mut cnt = 0;
    if radius_top.is_zero() {
        for i in 0..num_vertices {
            triangles[cnt] = (i + num_vertices_plus_one) as i32;
            cnt += 1;
            triangles[cnt] = (i) as i32;
            cnt += 1;
            triangles[cnt] = (i + 1 + num_vertices_plus_one) as i32;
            cnt += 1;
        }
    } else if radius_bottom.is_zero() {
        for i in 0..num_vertices {
            triangles[cnt] = (i) as i32;
            cnt += 1;
            triangles[cnt] = (i + 1) as i32;
            cnt += 1;
            triangles[cnt] = (i + num_vertices_plus_one) as i32;
            cnt += 1;
        }
    } else {
        for i in 0..num_vertices {
            let ip1 = i + 1;
            triangles[cnt] = (i) as i32;
            cnt += 1;
            triangles[cnt] = (ip1) as i32;
            cnt += 1;
            triangles[cnt] = (i + num_vertices_plus_one) as i32;
            cnt += 1;

            triangles[cnt] = (ip1 + num_vertices_plus_one) as i32;
            cnt += 1;
            triangles[cnt] = (i + num_vertices_plus_one) as i32;
            cnt += 1;
            triangles[cnt] = (ip1) as i32;
            cnt += 1;
        }
    }

    for i in 0..num_vertices {
        let mut next = cover_top_index_start + i + 1;

        if next == cover_top_index_end {
            next = cover_top_index_start
        }

        triangles[cnt] = (next) as i32;
        cnt += 1;
        triangles[cnt] = (cover_top_index_start + i) as i32;
        cnt += 1;
        triangles[cnt] = (cover_top_index_end) as i32;
        cnt += 1;
    }

    for i in 0..num_vertices {
        let mut next = cover_bottom_index_start + i + 1;
        if next == cover_bottom_index_end {
            next = cover_bottom_index_start;
        }

        triangles[cnt] = (cover_bottom_index_end) as i32;
        cnt += 1;
        triangles[cnt] = (cover_bottom_index_start + i) as i32;
        cnt += 1;
        triangles[cnt] = (next) as i32;
        cnt += 1;
    }

    let mut ret = VarArray::new();
    ret.resize(13, &VarArray::new().to_variant());
    ret.set(0, &vertices_array.to_variant());
    ret.set(1, &normals_array.to_variant());
    ret.set(4, &uvs_array.to_variant());
    ret.set(12, &triangles_array.to_variant());
    ret
}

pub fn create_or_update_mesh(
    static_body_3d: &mut Gd<StaticBody3D>,
    mesh_collider: &PbMeshCollider,
) {
    if static_body_3d.get_child_count() == 0 {
        return;
    }

    let mut collision_shape = if let Some(maybe_shape) = static_body_3d.get_child(0) {
        if let Ok(shape) = maybe_shape.try_cast::<CollisionShape3D>() {
            shape
        } else {
            return; // TODO: error
        }
    } else {
        return; // TODO: error
    };

    let current_shape = collision_shape.get_shape();
    let collision_mask = mesh_collider.collision_mask.unwrap_or(3); // (default CL_POINTER | CL_PHYSICS)

    let godot_shape = match mesh_collider.mesh.as_ref() {
        Some(mesh) => match mesh {
            pb_mesh_collider::Mesh::Box(_box_mesh) => {
                let box_shape = match current_shape {
                    Some(current_shape) => current_shape
                        .try_cast::<BoxShape3D>()
                        .unwrap_or(BoxShape3D::new_gd()),
                    None => BoxShape3D::new_gd(),
                };
                box_shape.upcast::<Shape3D>()
            }
            pb_mesh_collider::Mesh::Sphere(_sphere_mesh) => {
                let sphere_mesh = match current_shape {
                    Some(current_shape) => current_shape
                        .try_cast::<SphereShape3D>()
                        .unwrap_or(SphereShape3D::new_gd()),
                    None => SphereShape3D::new_gd(),
                };
                sphere_mesh.upcast::<Shape3D>()
            }
            pb_mesh_collider::Mesh::Cylinder(cylinder_mesh_value) => {
                let mut array_mesh = ArrayMesh::new_gd();
                let arrays = build_cylinder_arrays(
                    cylinder_mesh_value.radius_top.unwrap_or(0.5),
                    cylinder_mesh_value.radius_bottom.unwrap_or(0.5),
                );
                array_mesh.add_surface_from_arrays(PrimitiveType::TRIANGLES, &arrays);
                if let Some(new_shape) = array_mesh.create_trimesh_shape() {
                    new_shape.upcast::<Shape3D>()
                } else {
                    let mut cylinder_shape = match current_shape {
                        Some(current_shape) => current_shape
                            .try_cast::<CylinderShape3D>()
                            .unwrap_or(CylinderShape3D::new_gd()),
                        None => CylinderShape3D::new_gd(),
                    };
                    // TODO: top and bottom radius
                    let radius = (cylinder_mesh_value.radius_top.unwrap_or(0.5)
                        + cylinder_mesh_value.radius_bottom.unwrap_or(0.5))
                        * 0.5;
                    cylinder_shape.set_radius(radius);
                    cylinder_shape.set_height(1.0);
                    cylinder_shape.upcast::<Shape3D>()
                }
            }
            pb_mesh_collider::Mesh::Plane(_plane_mesh) => {
                let mut box_shape = match current_shape {
                    Some(current_shape) => current_shape
                        .try_cast::<BoxShape3D>()
                        .unwrap_or(BoxShape3D::new_gd()),
                    None => BoxShape3D::new_gd(),
                };
                box_shape.set_size(godot::prelude::Vector3::new(1.0, 1.0, 0.0));
                box_shape.upcast::<Shape3D>()
            }
            pb_mesh_collider::Mesh::Gltf(_) => {
                // TODO: Implement Gltf
                todo!("Implement Gltf Mesh Collider");
            }
        },
        _ => {
            let box_shape = match current_shape {
                Some(current_shape) => current_shape
                    .try_cast::<BoxShape3D>()
                    .unwrap_or(BoxShape3D::new_gd()),
                None => BoxShape3D::new_gd(),
            };
            box_shape.upcast::<Shape3D>()
        }
    };

    collision_shape.set_shape(&godot_shape);
    static_body_3d.set_collision_layer(collision_mask);
    static_body_3d.set_collision_mask(0);
}

pub fn update_mesh_collider(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_lww_components = &scene.current_dirty.lww_components;
    if let Some(mesh_collider_dirty) = dirty_lww_components.get(&SceneComponentId::MESH_COLLIDER) {
        let mesh_collider_component = SceneCrdtStateProtoComponents::get_mesh_collider(crdt_state);

        for entity in mesh_collider_dirty {
            let new_value = mesh_collider_component.get(entity);
            if new_value.is_none() {
                continue;
            }

            let new_value = new_value.unwrap();
            let (_godot_entity_node, mut node_3d) = godot_dcl_scene.ensure_node_3d(entity);

            let new_value = new_value.value.clone();
            let existing = node_3d.try_get_node_as::<StaticBody3D>("MeshCollider");

            if new_value.is_none() {
                if let Some(mut mesh_collider_node) = existing {
                    mesh_collider_node.queue_free();
                    node_3d.remove_child(&mesh_collider_node.upcast::<Node>());
                }
            } else if let Some(new_value) = new_value {
                let (mut static_body_3d, add_to_base) = match existing {
                    Some(static_body_3d) => (static_body_3d, false),
                    None => {
                        let mut body = StaticBody3D::new_alloc();

                        body.add_child(&CollisionShape3D::new_alloc().upcast::<Node>());

                        (body, true)
                    }
                };

                create_or_update_mesh(&mut static_body_3d, &new_value);

                if add_to_base {
                    static_body_3d.set_name("MeshCollider");
                    static_body_3d.set_meta("dcl_entity_id", &(entity.as_i32()).to_variant());
                    static_body_3d.set_meta("dcl_scene_id", &(scene.scene_id.0).to_variant());

                    // If entity has an active tween, set collider to KINEMATIC
                    if scene.kinematic_entities.contains(entity) {
                        let rid = static_body_3d.get_rid();
                        PhysicsServer3D::singleton()
                            .body_set_mode(rid, BodyMode::KINEMATIC);
                    }

                    node_3d.add_child(&static_body_3d.upcast::<Node>());
                }
            }
        }
    }
}
