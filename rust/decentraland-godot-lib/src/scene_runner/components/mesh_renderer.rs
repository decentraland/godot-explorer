use crate::{
    dcl::{
        components::{proto_components::sdk::components::pb_mesh_renderer, SceneComponentId},
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
        DirtyComponents,
    },
    scene_runner::{godot_dcl_scene::GodotDclScene, scene_manager::Scene},
};
use godot::{
    engine::{node::InternalMode, BoxMesh, CylinderMesh, MeshInstance3D, SphereMesh},
    prelude::*,
};

pub fn update_mesh_renderer(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let mut godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_components = &scene.current_dirty.components;
    if let Some(mesh_renderer_dirty) = dirty_components.get(&SceneComponentId::MESH_RENDERER) {
        let mesh_renderer_component = SceneCrdtStateProtoComponents::get_mesh_renderer(crdt_state);

        for entity in mesh_renderer_dirty {
            let new_value = mesh_renderer_component.get(*entity);
            if new_value.is_none() {
                continue;
            }

            let new_value = new_value.unwrap();
            let node = godot_dcl_scene.ensure_node_mut(entity);

            let new_value = new_value.value.clone();
            let existing = node
                .base
                .try_get_node_as::<MeshInstance3D>(NodePath::from("MeshRenderer"));

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
                                // TODO: using box mesh for now, the planeshape is one side faced
                                let mut new_plane_mesh = BoxMesh::new();
                                new_plane_mesh
                                    .set_size(godot::prelude::Vector3::new(1.0, 1.0, 0.0));
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
                    node.base.add_child(
                        new_mesh_instance_3d.share().upcast(),
                        false,
                        InternalMode::INTERNAL_MODE_DISABLED,
                    );

                    // scene.objs.push(new_mesh_instance_3d.share().upcast());
                }
            }
        }
    }
}
