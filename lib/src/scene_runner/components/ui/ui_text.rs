use godot::{obj::UserClass, prelude::Gd};

use crate::{
    dcl::{
        components::SceneComponentId,
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    godot_classes::dcl_ui_text::DclUiText,
    scene_runner::scene::Scene,
};

pub fn update_ui_text(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_lww_components = &scene.current_dirty.lww_components;
    let ui_text_component = SceneCrdtStateProtoComponents::get_ui_text(crdt_state);

    if let Some(dirty_ui_text) = dirty_lww_components.get(&SceneComponentId::UI_TEXT) {
        for entity in dirty_ui_text {
            let value = if let Some(entry) = ui_text_component.get(entity) {
                entry.value.clone()
            } else {
                None
            };

            let existing_ui_text = godot_dcl_scene
                .ensure_node_ui(entity)
                .base_ui
                .as_mut()
                .unwrap();

            if value.is_none() {
                if let Some(mut node) = existing_ui_text.base_control.get_node("text".into()) {
                    node.queue_free();
                    existing_ui_text.base_control.remove_child(node);
                }
                existing_ui_text.text_size = None;
                continue;
            }

            let value = value.as_ref().unwrap();
            let mut existing_ui_text_control = if let Some(node) = existing_ui_text
                .base_control
                .get_node_or_null("text".into())
            {
                node.cast::<DclUiText>()
            } else {
                let mut node: Gd<DclUiText> = DclUiText::alloc_gd();
                node.set_name("text".into());

                existing_ui_text
                    .base_control
                    .add_child(node.clone().upcast());
                existing_ui_text
                    .base_control
                    .move_child(node.clone().upcast(), 1);
                node
            };

            existing_ui_text_control.bind_mut().change_value(value);
            existing_ui_text.text_size = Some(existing_ui_text_control.get_minimum_size());
        }
    }
}
