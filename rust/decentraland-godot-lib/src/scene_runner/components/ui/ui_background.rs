use godot::prelude::Gd;

use crate::{
    dcl::{
        components::SceneComponentId,
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    godot_classes::dcl_ui_background::DclUiBackground,
    scene_runner::scene::Scene,
};

pub fn update_ui_background(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_lww_components = &scene.current_dirty.lww_components;
    let ui_background_component = SceneCrdtStateProtoComponents::get_ui_background(crdt_state);

    if let Some(dirty_ui_background) = dirty_lww_components.get(&SceneComponentId::UI_BACKGROUND) {
        for entity in dirty_ui_background {
            let value = if let Some(entry) = ui_background_component.get(*entity) {
                entry.value.clone()
            } else {
                None
            };

            let existing_ui_background = godot_dcl_scene
                .ensure_node_ui(entity)
                .base_ui
                .as_mut()
                .unwrap();

            if value.is_none() {
                if let Some(node) = existing_ui_background.base_control.get_node("bkg".into()) {
                    existing_ui_background.base_control.remove_child(node);
                }

                continue;
            }

            let value = value.as_ref().unwrap();

            let mut existing_ui_background_control = if let Some(node) = existing_ui_background
                .base_control
                .get_node_or_null("bkg".into())
            {
                node.cast::<DclUiBackground>()
            } else {
                // let mut node = Gd::ne<DclUiBackground>::new_alloc();
                let mut node: Gd<DclUiBackground> = Gd::new_default();
                node.set_name("bkg".into());
                node.set_anchors_preset(godot::engine::control::LayoutPreset::PRESET_FULL_RECT);

                existing_ui_background
                    .base_control
                    .add_child(node.clone().upcast());
                existing_ui_background
                    .base_control
                    .move_child(node.clone().upcast(), 0);
                node
            };

            existing_ui_background_control
                .bind_mut()
                .change_value(value.clone(), &scene.content_mapping);
        }
    }
}
