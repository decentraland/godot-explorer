use super::godot_dcl_scene::GodotDclScene;
use crate::dcl::{
    components::{proto_components::sdk::components::pb_mesh_renderer, SceneComponentId},
    crdt::{last_write_wins::LastWriteWinsComponentOperation, SceneCrdtStateProtoComponents},
    DirtyComponents, DirtyEntities,
};
use godot::{
    engine::{node::InternalMode, BoxMesh, CylinderMesh, MeshInstance3D, PlaneMesh, SphereMesh},
    prelude::*,
};

pub fn update_scene(
    _dt: f64,
    scene: &mut GodotDclScene,
    _dirty_entities: DirtyEntities,
    dirty_components: DirtyComponents,
) {
    scene.waiting_for_updates = false;

    let crdt = scene.dcl_scene.scene_crdt.clone();
    let crdt_state = crdt.lock().unwrap();
    let transform_component = crdt_state.get_transform();

    if let Some(transform_dirty) = dirty_components.get(&SceneComponentId::TRANSFORM) {
        for entity in transform_dirty {
            let value = transform_component.get(*entity);
            let mut node = scene.ensure_node(entity);
            if let Some(entry) = value {
                if let Some(transform) = entry.value.clone() {
                    node.set_rotation(transform.rotation.to_euler(EulerOrder::XYZ));
                    node.set_position(transform.translation);
                    node.set_scale(transform.scale);
                }
            }
        }
    }

    if let Some(mesh_renderer_dirty) = dirty_components.get(&SceneComponentId::MESH_RENDERER) {
        let mesh_renderer_component = SceneCrdtStateProtoComponents::get_mesh_renderer(&crdt_state);

        for entity in mesh_renderer_dirty {
            let new_value = mesh_renderer_component.get(*entity);
            if new_value.is_none() {
                continue;
            }

            let new_value = new_value.unwrap();
            let mut node = scene.ensure_node(entity);

            let new_value = new_value.value.clone();
            let existing = node.try_get_node_as::<MeshInstance3D>(NodePath::from("MeshRenderer"));

            if new_value.is_none() {
                if existing.is_some() {
                    // remove
                }
            } else if let Some(new_value) = new_value {
                if let Some(_existing) = existing {
                    // update
                } else {
                    // create
                    let mut new_mesh_instance_3d = MeshInstance3D::new_alloc();

                    match new_value.mesh {
                        Some(mesh) => match mesh {
                            pb_mesh_renderer::Mesh::Box(_box_mesh) => {
                                let new_box_mesh = BoxMesh::new();
                                new_mesh_instance_3d.set_mesh(new_box_mesh.upcast());

                                // update the material (and with uvs)
                            }
                            pb_mesh_renderer::Mesh::Sphere(_sphere_mesh) => {
                                let new_sphere_mesh = SphereMesh::new();
                                new_mesh_instance_3d.set_mesh(new_sphere_mesh.upcast());

                                // update the material
                            }
                            pb_mesh_renderer::Mesh::Cylinder(cylinder_mesh) => {
                                let mut new_cylinder_mesh = CylinderMesh::new();
                                new_cylinder_mesh
                                    .set_top_radius(cylinder_mesh.radius_top.unwrap_or(0.5) as f64);
                                new_cylinder_mesh.set_bottom_radius(
                                    cylinder_mesh.radius_bottom.unwrap_or(0.5) as f64,
                                );
                                new_cylinder_mesh.set_height(1.0);
                                new_mesh_instance_3d.set_mesh(new_cylinder_mesh.upcast());

                                // update the material
                            }
                            pb_mesh_renderer::Mesh::Plane(_plane_mesh) => {
                                let new_plane_mesh = PlaneMesh::new();
                                new_mesh_instance_3d.set_mesh(new_plane_mesh.upcast());

                                // update the material (and with uvs)
                            }
                        },
                        _ => {
                            let new_box_mesh = BoxMesh::new();
                            new_mesh_instance_3d.set_mesh(new_box_mesh.upcast());
                        }
                    }

                    new_mesh_instance_3d.set_name(GodotString::from("MeshRenderer"));
                    node.add_child(
                        new_mesh_instance_3d.share().upcast(),
                        false,
                        InternalMode::INTERNAL_MODE_DISABLED,
                    );

                    // scene.objs.push(new_mesh_instance_3d.share().upcast());
                }
            }
        }
    }

    drop(crdt_state)
}
