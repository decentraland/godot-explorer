use crate::{
    dcl::{
        components::{SceneComponentId, SceneEntityId},
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    scene_runner::scene::Scene,
};

pub fn update_ui_transform(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_lww_components = &scene.current_dirty.lww_components;
    let ui_transform_component = SceneCrdtStateProtoComponents::get_ui_transform(crdt_state);

    if let Some(dirty_transform) = dirty_lww_components.get(&SceneComponentId::UI_TRANSFORM) {
        for entity in dirty_transform {
            let new_parent = if let Some(entry) = ui_transform_component.get(*entity) {
                SceneEntityId::from_i32(entry.value.as_ref().unwrap().parent)
            } else {
                SceneEntityId::ROOT
            };

            let node = godot_dcl_scene.ensure_node_ui(entity);
            node.parent_ui = new_parent;
        }
    }
}
