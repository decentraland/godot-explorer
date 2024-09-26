use godot::{obj::NewAlloc, prelude::Gd};

use crate::{
    dcl::{
        components::SceneComponentId,
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    godot_classes::dcl_ui_input::DclUiInput,
    scene_runner::scene::Scene,
};

pub fn update_ui_input(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_lww_components = &scene.current_dirty.lww_components;
    let ui_input_component = SceneCrdtStateProtoComponents::get_ui_input(crdt_state);

    if let Some(dirty_ui_input) = dirty_lww_components.get(&SceneComponentId::UI_INPUT) {
        for entity in dirty_ui_input {
            let value = if let Some(entry) = ui_input_component.get(entity) {
                entry.value.clone()
            } else {
                None
            };

            let existing_ui_input = godot_dcl_scene
                .ensure_node_ui(entity)
                .base_ui
                .as_mut()
                .unwrap();

            if value.is_none() {
                if let Some(mut node) = existing_ui_input
                    .base_control
                    .get_node_or_null("input".into())
                {
                    node.queue_free();
                    existing_ui_input.base_control.remove_child(node);
                }
                existing_ui_input.text_size = None;
                continue;
            }

            let value = value.as_ref().unwrap();
            if let Some(node) = existing_ui_input
                .base_control
                .get_node_or_null("input".into())
            {
                let mut existing_ui_input_control = node.cast::<DclUiInput>();
                existing_ui_input_control.bind_mut().change_value(value);
                existing_ui_input.text_size = Some(existing_ui_input_control.get_size());
            } else {
                let mut node: Gd<DclUiInput> = DclUiInput::new_alloc();
                node.set_name("input".into());
                node.set_anchors_preset(godot::engine::control::LayoutPreset::FULL_RECT);

                node.bind_mut().set_dcl_entity_id(entity.as_i32());

                existing_ui_input
                    .base_control
                    .add_child(node.clone().upcast());
                existing_ui_input
                    .base_control
                    .move_child(node.clone().upcast(), 1);

                node.bind_mut().change_value(value);
                existing_ui_input.text_size = Some(node.get_size());

                node.bind_mut()
                    .set_ui_result(godot_dcl_scene.ui_results.clone());
            }
        }
    }
}
