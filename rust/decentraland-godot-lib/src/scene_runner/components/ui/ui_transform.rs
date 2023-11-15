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

use super::style::UiTransform;

pub fn update_ui_transform(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_lww_components = &scene.current_dirty.lww_components;
    let ui_transform_component = SceneCrdtStateProtoComponents::get_ui_transform(crdt_state);

    if let Some(dirty_transform) = dirty_lww_components.get(&SceneComponentId::UI_TRANSFORM) {
        for entity in dirty_transform {
            let ui_transform = if let Some(entry) = ui_transform_component.get(entity) {
                entry.value.as_ref()
            } else {
                None
            };

            if let Some(pb_ui_transform) = ui_transform {
                let node = godot_dcl_scene
                    .ensure_node_ui(entity)
                    .base_ui
                    .as_mut()
                    .unwrap();

                node.ui_transform = UiTransform::from(pb_ui_transform);
                node.base_control.set_clip_contents(
                    node.ui_transform.overflow == crate::dcl::components::proto_components::sdk::components::YgOverflow::YgoHidden,
                );
                node.base_control
                    .bind_mut()
                    .set_pointer_filter(node.ui_transform.pointer_filter_mode);
            }
        }
    }
}
