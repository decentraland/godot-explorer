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
    scene_runner::scene_manager::Scene,
};
use godot::{
    engine::{
        node::InternalMode, AnimatableBody3D, BoxShape3D, CollisionShape3D, CylinderShape3D,
        SphereShape3D,
    },
    prelude::*,
};

pub fn create_or_update_mesh(
    animatable_body_3d: &mut Gd<AnimatableBody3D>,
    mesh_collider: &PbMeshCollider,
) {
    let mut collision_shape = if let Some(maybe_shape) = animatable_body_3d.get_child(0, false) {
        if let Some(shape) = maybe_shape.try_cast::<CollisionShape3D>() {
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
                        .unwrap_or(BoxShape3D::new()),
                    None => BoxShape3D::new(),
                };
                box_shape.upcast()
            }
            pb_mesh_collider::Mesh::Sphere(_sphere_mesh) => {
                let sphere_mesh = match current_shape {
                    Some(current_shape) => current_shape
                        .try_cast::<SphereShape3D>()
                        .unwrap_or(SphereShape3D::new()),
                    None => SphereShape3D::new(),
                };
                sphere_mesh.upcast()
            }
            pb_mesh_collider::Mesh::Cylinder(cylinder_mesh_value) => {
                let mut cylinder_shape = match current_shape {
                    Some(current_shape) => current_shape
                        .try_cast::<CylinderShape3D>()
                        .unwrap_or(CylinderShape3D::new()),
                    None => CylinderShape3D::new(),
                };
                // TODO: top and bottom radius
                let radius = (cylinder_mesh_value.radius_top.unwrap_or(0.5)
                    + cylinder_mesh_value.radius_bottom.unwrap_or(0.5))
                    * 0.5;
                cylinder_shape.set_radius(radius as f64);
                cylinder_shape.set_height(1.0);
                cylinder_shape.upcast()
            }
            pb_mesh_collider::Mesh::Plane(_plane_mesh) => {
                let mut box_shape = match current_shape {
                    Some(current_shape) => current_shape
                        .try_cast::<BoxShape3D>()
                        .unwrap_or(BoxShape3D::new()),
                    None => BoxShape3D::new(),
                };
                box_shape.set_size(godot::prelude::Vector3::new(1.0, 1.0, 0.0));
                box_shape.upcast()
            }
        },
        _ => {
            let box_shape = match current_shape {
                Some(current_shape) => current_shape
                    .try_cast::<BoxShape3D>()
                    .unwrap_or(BoxShape3D::new()),
                None => BoxShape3D::new(),
            };
            box_shape.upcast()
        }
    };

    collision_shape.set_shape(godot_shape);
    animatable_body_3d.set_collision_layer(collision_mask as i64);
}

pub fn update_mesh_collider(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_lww_components = &scene.current_dirty.lww_components;
    if let Some(mesh_collider_dirty) = dirty_lww_components.get(&SceneComponentId::MESH_COLLIDER) {
        let mesh_collider_component = SceneCrdtStateProtoComponents::get_mesh_collider(crdt_state);

        for entity in mesh_collider_dirty {
            let new_value = mesh_collider_component.get(*entity);
            if new_value.is_none() {
                continue;
            }

            let new_value = new_value.unwrap();
            let node = godot_dcl_scene.ensure_node_mut(entity);

            let new_value = new_value.value.clone();
            let existing = node
                .base
                .try_get_node_as::<AnimatableBody3D>(NodePath::from("MeshCollider"));

            if new_value.is_none() {
                if let Some(mesh_collider_node) = existing {
                    node.base.remove_child(mesh_collider_node.upcast());
                }
            } else if let Some(new_value) = new_value {
                let (mut animatable_body_3d, add_to_base) = match existing {
                    Some(animatable_body_3d) => (animatable_body_3d, false),
                    None => {
                        let mut body = AnimatableBody3D::new_alloc();

                        body.set("sync_to_physics".into(), Variant::from(false));
                        body.add_child(
                            CollisionShape3D::new_alloc().upcast(),
                            false,
                            InternalMode::INTERNAL_MODE_DISABLED,
                        );

                        (body, true)
                    }
                };

                create_or_update_mesh(&mut animatable_body_3d, &new_value);

                if add_to_base {
                    animatable_body_3d.set_name(GodotString::from("MeshCollider"));
                    animatable_body_3d.set_meta(
                        "dcl_entity_id".into(),
                        (entity.as_usize() as i32).to_variant(),
                    );
                    animatable_body_3d.set_meta(
                        "dcl_scene_id".into(),
                        (scene.scene_id.0 as i32).to_variant(),
                    );

                    node.base.add_child(
                        animatable_body_3d.upcast(),
                        false,
                        InternalMode::INTERNAL_MODE_DISABLED,
                    );
                }
            }
        }
    }
}
