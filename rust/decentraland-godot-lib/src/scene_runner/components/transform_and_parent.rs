use godot::prelude::{Node, Share, Transform3D, Vector3};

use crate::{
    dcl::{
        components::{
            transform_and_parent::DclTransformAndParent, SceneComponentId, SceneEntityId,
        },
        crdt::{last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState},
        DirtyComponents,
    },
    scene_runner::godot_dcl_scene::GodotDclScene,
};

impl DclTransformAndParent {
    pub fn from_godot(godot_transform: &Transform3D, offset: Vector3) -> Self {
        let rotation = godot_transform.basis.orthonormalized().to_quat();
        let translation = godot_transform.origin - offset;
        let scale = godot_transform.basis.scale();

        Self {
            translation,
            rotation,
            scale,
            parent: SceneEntityId::ROOT,
        }
    }
}

pub fn update_transform_and_parent(
    godot_dcl_scene: &mut GodotDclScene,
    crdt_state: &mut SceneCrdtState,
    dirty_components: &DirtyComponents,
) {
    let transform_component = crdt_state.get_transform();

    if let Some(dirty_transform) = dirty_components.get(&SceneComponentId::TRANSFORM) {
        for entity in dirty_transform {
            let value = if let Some(entry) = transform_component.get(*entity) {
                entry.value.clone()
            } else {
                None
            };
            let node = godot_dcl_scene.ensure_node_mut(entity);

            let old_parent = node.desired_parent;
            let mut transform = value.unwrap_or_default();
            if !transform.rotation.is_normalized() {
                if transform.rotation.length_squared() == 0.0 {
                    transform.rotation = godot::prelude::Quaternion::default();
                } else {
                    transform.rotation = transform.rotation.normalized();
                }
            }

            // node.base.set_position(transform.translation);
            // node.base
            //     .set_rotation(transform.rotation.to_euler(godot::prelude::EulerOrder::XYZ));

            // TODO: the scale seted in the transform is local
            node.base.set_transform(transform.to_godot_transform_3d());
            node.base.set_scale(transform.scale);

            node.desired_parent = transform.parent;
            if node.desired_parent != old_parent {
                godot_dcl_scene.unparented_entities.insert(*entity);
                godot_dcl_scene.hierarchy_dirty = true;
            }
        }
    }

    let root_node = godot_dcl_scene.root_node.share().upcast::<Node>();
    while godot_dcl_scene.hierarchy_dirty {
        godot_dcl_scene.hierarchy_dirty = false;

        let unparented = godot_dcl_scene
            .unparented_entities
            .iter()
            .copied()
            .collect::<Vec<SceneEntityId>>();

        for entity in unparented {
            let desired_parent = godot_dcl_scene.get_node(&entity).unwrap().desired_parent;

            // cancel if the desired_parent is the entity itself
            if desired_parent == entity {
                continue;
            }

            // if parent doens't exist cause it's dead, we remap to the root entity
            if crdt_state.entities.is_dead(&desired_parent) {
                let current_node = godot_dcl_scene.ensure_node_mut(&entity);
                current_node.base.reparent(root_node.share(), false);
                current_node.computed_parent = SceneEntityId::ROOT;

                godot_dcl_scene.ensure_node_mut(&entity).desired_parent = SceneEntityId::ROOT;
                godot_dcl_scene.hierarchy_dirty = true;
            } else {
                let has_cycle =
                    detect_entity_id_in_parent_chain(godot_dcl_scene, desired_parent, entity);

                if !has_cycle {
                    let parent_node = godot_dcl_scene
                        .ensure_node_mut(&desired_parent)
                        .base
                        .share()
                        .upcast::<Node>();

                    let current_node = godot_dcl_scene.ensure_node_mut(&entity);
                    current_node.base.reparent(parent_node, false);
                    current_node.computed_parent = desired_parent;

                    godot_dcl_scene.hierarchy_dirty = true;
                    godot_dcl_scene.unparented_entities.remove(&entity);
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
    while let Some(node) = godot_dcl_scene.get_node(&current_entity) {
        if current_entity == SceneEntityId::ROOT {
            return false;
        }

        if node.desired_parent == search_entity {
            return true;
        }
        current_entity = node.desired_parent;
    }

    false
}
