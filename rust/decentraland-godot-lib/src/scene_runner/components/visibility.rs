use crate::{
    dcl::{
        components::SceneComponentId,
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    scene_runner::scene::Scene,
};

pub fn update_visibility(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_lww_components = &scene.current_dirty.lww_components;
    let visibility_component = SceneCrdtStateProtoComponents::get_visibility_component(crdt_state);

    let Some(visibility_dirty) = dirty_lww_components.get(&SceneComponentId::VISIBILITY_COMPONENT)
    else {
        return;
    };

    for entity in visibility_dirty {
        let new_value = visibility_component.get(*entity);

        let Some(new_value) = new_value else {
            continue;
        };

        let Some(new_value) = new_value.value.clone() else {
            continue;
        };

        let (_godot_entity_node, mut node_3d) = godot_dcl_scene.ensure_node_3d(entity);
        if new_value.visible() {
            node_3d.show();
        } else {
            node_3d.hide();
        }
    }
}
