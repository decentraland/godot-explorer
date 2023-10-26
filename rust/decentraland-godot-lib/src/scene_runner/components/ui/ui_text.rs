use godot::engine::{
    global::{HorizontalAlignment, VerticalAlignment},
    Label,
};

use crate::{
    dcl::{
        components::{proto_components::sdk::components::common::TextAlignMode, SceneComponentId},
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    scene_runner::scene::Scene,
};

pub fn update_ui_text(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_lww_components = &scene.current_dirty.lww_components;
    let ui_text_component = SceneCrdtStateProtoComponents::get_ui_text(crdt_state);

    if let Some(dirty_ui_text) = dirty_lww_components.get(&SceneComponentId::UI_TEXT) {
        for entity in dirty_ui_text {
            let value = if let Some(entry) = ui_text_component.get(*entity) {
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
                if let Some(node) = existing_ui_text.base_control.get_node("text".into()) {
                    existing_ui_text.base_control.remove_child(node);
                }

                existing_ui_text.has_text = false;
                continue;
            }

            existing_ui_text.has_text = true;
            let value = value.as_ref().unwrap();

            let mut existing_ui_text_control = if let Some(node) = existing_ui_text
                .base_control
                .get_node_or_null("text".into())
            {
                node.cast::<Label>()
            } else {
                let mut node = Label::new_alloc();
                node.set_name("text".into());
                node.set_anchors_preset(godot::engine::control::LayoutPreset::PRESET_FULL_RECT);

                existing_ui_text
                    .base_control
                    .add_child(node.clone().upcast());
                existing_ui_text
                    .base_control
                    .move_child(node.clone().upcast(), 1);
                node
            };
            existing_ui_text_control
                .add_theme_font_size_override("font_size".into(), value.font_size.unwrap_or(10));
            let font_color = value
                .color
                .as_ref()
                .map(|v| godot::prelude::Color {
                    r: v.r,
                    g: v.g,
                    b: v.b,
                    a: v.a,
                })
                .unwrap_or(godot::prelude::Color::WHITE);
            existing_ui_text_control.add_theme_color_override("font_color".into(), font_color);

            let text_align = value
                .text_align
                .map(TextAlignMode::from_i32)
                .unwrap_or(Some(TextAlignMode::TamMiddleCenter))
                .unwrap();

            let (hor_align, vert_align) = match text_align {
                TextAlignMode::TamTopLeft => (
                    HorizontalAlignment::HORIZONTAL_ALIGNMENT_LEFT,
                    VerticalAlignment::VERTICAL_ALIGNMENT_TOP,
                ),
                TextAlignMode::TamTopCenter => (
                    HorizontalAlignment::HORIZONTAL_ALIGNMENT_CENTER,
                    VerticalAlignment::VERTICAL_ALIGNMENT_TOP,
                ),
                TextAlignMode::TamTopRight => (
                    HorizontalAlignment::HORIZONTAL_ALIGNMENT_RIGHT,
                    VerticalAlignment::VERTICAL_ALIGNMENT_TOP,
                ),
                TextAlignMode::TamMiddleLeft => (
                    HorizontalAlignment::HORIZONTAL_ALIGNMENT_LEFT,
                    VerticalAlignment::VERTICAL_ALIGNMENT_CENTER,
                ),
                TextAlignMode::TamMiddleCenter => (
                    HorizontalAlignment::HORIZONTAL_ALIGNMENT_CENTER,
                    VerticalAlignment::VERTICAL_ALIGNMENT_CENTER,
                ),
                TextAlignMode::TamMiddleRight => (
                    HorizontalAlignment::HORIZONTAL_ALIGNMENT_RIGHT,
                    VerticalAlignment::VERTICAL_ALIGNMENT_CENTER,
                ),
                TextAlignMode::TamBottomLeft => (
                    HorizontalAlignment::HORIZONTAL_ALIGNMENT_LEFT,
                    VerticalAlignment::VERTICAL_ALIGNMENT_BOTTOM,
                ),
                TextAlignMode::TamBottomCenter => (
                    HorizontalAlignment::HORIZONTAL_ALIGNMENT_CENTER,
                    VerticalAlignment::VERTICAL_ALIGNMENT_BOTTOM,
                ),
                TextAlignMode::TamBottomRight => (
                    HorizontalAlignment::HORIZONTAL_ALIGNMENT_RIGHT,
                    VerticalAlignment::VERTICAL_ALIGNMENT_BOTTOM,
                ),
            };

            existing_ui_text_control.set_vertical_alignment(vert_align);
            existing_ui_text_control.set_horizontal_alignment(hor_align);
            existing_ui_text_control.set_text(value.value.clone().into());
        }
    }
}
