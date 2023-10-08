use crate::{scene_runner::scene::Scene, dcl::crdt::{SceneCrdtState, SceneCrdtStateProtoComponents}};

pub fn update_visibility(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let visibility_component = SceneCrdtStateProtoComponents::get_visibility_component(crdt_state);

    for (entity, entry) in visibility_component.values.iter() {
        if let Some(_visibility) = entry.value.as_ref() {
            let node = scene.godot_dcl_scene.ensure_node_mut(entity);
            if _visibility.visible() {
                node.base.show();
            } else {
                node.base.hide();
            }
        }
    }
}