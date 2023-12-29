use godot::{
    builtin::math::FloatExt,
    prelude::{Node, Transform3D, Vector3},
};

use crate::{
    dcl::{
        components::{
            transform_and_parent::DclTransformAndParent, SceneComponentId, SceneEntityId,
        },
        crdt::{last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState},
    },
    scene_runner::{godot_dcl_scene::GodotDclScene, scene::Scene},
};

impl DclTransformAndParent {
    pub fn from_godot(godot_transform: &Transform3D, offset: Vector3) -> Self {
        let rotation = godot_transform.basis.orthonormalized().to_quat();
        let translation = godot_transform.origin - offset;
        let scale = godot_transform.basis.scale();

        Self {
            translation: godot::prelude::Vector3 {
                x: translation.x,
                y: translation.y,
                z: -translation.z,
            },
            rotation: godot::prelude::Quaternion {
                x: rotation.x,
                y: rotation.y,
                z: -rotation.z,
                w: -rotation.w,
            },
            scale,
            parent: SceneEntityId::ROOT,
        }
    }
}

pub fn update_transform_and_parent(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_lww_components = &scene.current_dirty.lww_components;
    let transform_component = crdt_state.get_transform();

    if let Some(dirty_transform) = dirty_lww_components.get(&SceneComponentId::TRANSFORM) {
        for entity in dirty_transform {
            let value = if let Some(entry) = transform_component.get(entity) {
                entry.value.clone()
            } else {
                None
            };
            let (godot_entity_node, mut node_3d) = godot_dcl_scene.ensure_node_3d(entity);

            let old_parent = godot_entity_node.desired_parent_3d;
            let mut transform = value.unwrap_or_default();
            if !transform.rotation.is_normalized() {
                if transform.rotation.length_squared() == 0.0 {
                    transform.rotation = godot::prelude::Quaternion::default();
                } else {
                    transform.rotation = transform.rotation.normalized();
                }
            }

            node_3d.set_transform(transform.to_godot_transform_3d_without_scaled());
            if transform.scale.x.is_zero_approx() {
                transform.scale.x = 0.00001;
            }
            if transform.scale.y.is_zero_approx() {
                transform.scale.y = 0.00001;
            }
            if transform.scale.z.is_zero_approx() {
                transform.scale.z = 0.00001;
            }
            node_3d.set_scale(transform.scale);

            godot_entity_node.desired_parent_3d = transform.parent;
            if godot_entity_node.desired_parent_3d != old_parent {
                godot_dcl_scene.unparented_entities_3d.insert(*entity);
                godot_dcl_scene.hierarchy_dirty_3d = true;
            }
        }
    }

    let root_node = godot_dcl_scene.root_node_3d.clone().upcast::<Node>();
    while godot_dcl_scene.hierarchy_dirty_3d {
        godot_dcl_scene.hierarchy_dirty_3d = false;

        let unparented = godot_dcl_scene
            .unparented_entities_3d
            .iter()
            .copied()
            .collect::<Vec<SceneEntityId>>();

        for entity in unparented {
            let desired_parent_3d =
                if let Some(node) = godot_dcl_scene.get_godot_entity_node(&entity) {
                    node.desired_parent_3d
                } else {
                    godot_dcl_scene.ensure_node_3d(&entity).0.desired_parent_3d
                };

            // cancel if the desired_parent_3d is the entity itself
            if desired_parent_3d == entity {
                continue;
            }

            // if parent doens't exist cause it's dead, we remap to the root entity
            if crdt_state.entities.is_dead(&desired_parent_3d) {
                let (current_godot_entity_node, mut current_node_3d) =
                    godot_dcl_scene.ensure_node_3d(&entity);

                current_node_3d
                    .reparent_ex(root_node.clone())
                    .keep_global_transform(false)
                    .done();
                current_godot_entity_node.computed_parent_3d = SceneEntityId::ROOT;

                godot_dcl_scene.ensure_node_3d(&entity).0.desired_parent_3d = SceneEntityId::ROOT;
                godot_dcl_scene.hierarchy_dirty_3d = true;
            } else {
                let has_cycle =
                    detect_entity_id_in_parent_chain(godot_dcl_scene, desired_parent_3d, entity);

                if !has_cycle {
                    let parent_node = godot_dcl_scene
                        .ensure_node_3d(&desired_parent_3d)
                        .1
                        .upcast::<Node>();

                    let (current_godot_entity_node, mut current_node_3d) =
                        godot_dcl_scene.ensure_node_3d(&entity);

                    current_node_3d
                        .reparent_ex(parent_node)
                        .keep_global_transform(false)
                        .done();
                    current_godot_entity_node.computed_parent_3d = desired_parent_3d;

                    godot_dcl_scene.hierarchy_dirty_3d = true;
                    godot_dcl_scene.unparented_entities_3d.remove(&entity);
                }
            }
        }
    }
}

fn detect_entity_id_in_parent_chain(
    godot_dcl_scene: &GodotDclScene,
    mut current_entity: SceneEntityId,
    search_entity: SceneEntityId,
) -> bool {
    while let Some(node) = godot_dcl_scene.get_godot_entity_node(&current_entity) {
        if current_entity == SceneEntityId::ROOT {
            return false;
        }

        if node.desired_parent_3d == search_entity {
            return true;
        }
        current_entity = node.desired_parent_3d;
    }

    false
}
