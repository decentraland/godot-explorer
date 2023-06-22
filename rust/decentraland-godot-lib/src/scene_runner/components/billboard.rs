use crate::{
    dcl::crdt::{SceneCrdtState, SceneCrdtStateProtoComponents},
    scene_runner::scene_manager::Scene,
};
use godot::prelude::*;

pub fn update_billboard(
    scene: &mut Scene,
    crdt_state: &mut SceneCrdtState,
    camera_global_transform: &Transform3D,
) {
    let billboard_component = SceneCrdtStateProtoComponents::get_billboard(crdt_state);
    let camera_position = camera_global_transform.origin;

    for (entity, entry) in billboard_component.values.iter() {
        if let Some(_billboard) = entry.value.as_ref() {
            let node = scene.godot_dcl_scene.ensure_node_mut(entity);
            let original_scale = node.base.get_scale();
            let origin = node.base.get_global_position();
            let direction = node.base.get_global_position() - camera_position;

            let basis = Basis::new_looking_at(direction, Vector3::UP);
            node.base
                .set_global_transform(Transform3D { basis, origin });

            node.base.set_scale(original_scale);

            // TODO: implement billboard mode
        }
    }
}
