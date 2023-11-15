use godot::prelude::Gd;

use crate::{
    dcl::{
        components::SceneComponentId,
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    godot_classes::dcl_ui_dropdown::DclUiDropdown,
    scene_runner::scene::Scene,
};

pub fn update_ui_dropdown(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_lww_components = &scene.current_dirty.lww_components;
    let ui_dropdown_component = SceneCrdtStateProtoComponents::get_ui_dropdown(crdt_state);

    if let Some(dirty_ui_dropdown) = dirty_lww_components.get(&SceneComponentId::UI_DROPDOWN) {
        for entity in dirty_ui_dropdown {
            let value = if let Some(entry) = ui_dropdown_component.get(entity) {
                entry.value.clone()
            } else {
                None
            };

            let existing_ui_dropdown = godot_dcl_scene
                .ensure_node_ui(entity)
                .base_ui
                .as_mut()
                .unwrap();

            if value.is_none() {
                if let Some(node) = existing_ui_dropdown
                    .base_control
                    .get_node("dropdown".into())
                {
                    existing_ui_dropdown.base_control.remove_child(node);
                }
                existing_ui_dropdown.has_text = false;
                continue;
            }
            existing_ui_dropdown.has_text = true;

            let value = value.as_ref().unwrap();
            let mut existing_ui_dropdown_control = if let Some(node) = existing_ui_dropdown
                .base_control
                .get_node_or_null("dropdown".into())
            {
                node.cast::<DclUiDropdown>()
            } else {
                let mut node: Gd<DclUiDropdown> = Gd::new_default();
                node.set_name("dropdown".into());
                node.set_anchors_preset(godot::engine::control::LayoutPreset::PRESET_FULL_RECT);
                node.bind_mut().set_dcl_entity_id(entity.as_i32());

                existing_ui_dropdown
                    .base_control
                    .add_child(node.clone().upcast());
                existing_ui_dropdown
                    .base_control
                    .move_child(node.clone().upcast(), 1);
                node.bind_mut()
                    .set_ui_result(godot_dcl_scene.ui_results.clone());
                node
            };

            existing_ui_dropdown_control.bind_mut().change_value(value);
        }
    }
}
