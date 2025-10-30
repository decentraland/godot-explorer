use crate::{
    dcl::{
        components::{
            proto_components::sdk::components::common::{Font, TextAlignMode},
            SceneComponentId,
        },
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    scene_runner::scene::Scene,
};
use godot::{
    classes::{
        global::{HorizontalAlignment, VerticalAlignment},
        label_3d::AlphaCutMode,
        text_server::AutowrapMode,
        Label3D,
    },
    prelude::*,
};

pub fn update_text_shape(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_lww_components = &scene.current_dirty.lww_components;
    if let Some(text_shape_dirty) = dirty_lww_components.get(&SceneComponentId::TEXT_SHAPE) {
        let text_shape_component = SceneCrdtStateProtoComponents::get_text_shape(crdt_state);

        for entity in text_shape_dirty {
            let new_value = text_shape_component.get(entity);
            if new_value.is_none() {
                continue;
            }

            let new_value = new_value.unwrap();
            let (_godot_entity_node, mut node_3d) = godot_dcl_scene.ensure_node_3d(entity);

            let new_value = new_value.value.clone();
            let existing = node_3d.try_get_node_as::<Label3D>(NodePath::from("TextShape"));

            if new_value.is_none() {
                if let Some(mut text_shape_node) = existing {
                    text_shape_node.queue_free();
                    node_3d.remove_child(text_shape_node.upcast());
                }
            } else if let Some(new_value) = new_value {
                let (mut label_3d, add_to_base) = match existing {
                    Some(label_3d) => (label_3d, false),
                    None => (Label3D::new_alloc(), true),
                };

                let text_align = TextAlignMode::from_i32(
                    new_value
                        .text_align
                        .unwrap_or(TextAlignMode::TamMiddleCenter as i32),
                )
                .unwrap_or(TextAlignMode::TamMiddleCenter);
                let opacity = new_value
                    .text_color
                    .as_ref()
                    .map(|color| color.a)
                    .unwrap_or(1.0);

                let text_color = new_value
                    .text_color
                    .map(|color| Color::from_rgba(color.r, color.g, color.b, opacity))
                    .unwrap_or(Color::from_rgba(1.0, 1.0, 1.0, opacity));

                let outline_color = new_value
                    .outline_color
                    .map(|color| Color::from_rgba(color.r, color.g, color.b, opacity))
                    .unwrap_or(Color::from_rgba(1.0, 1.0, 1.0, opacity));

                label_3d.set_text(GString::from(new_value.text));
                label_3d.set_modulate(text_color);

                let font_size = (22.0 * new_value.font_size.unwrap_or(3.0)).max(1.0);
                let outline_size = font_size * new_value.outline_width.unwrap_or(0.0);
                label_3d.set_font_size(font_size as i32);
                label_3d.set_outline_size(outline_size as i32);
                label_3d.set_outline_modulate(outline_color);
                label_3d.set_alpha_cut_mode(AlphaCutMode::DISCARD);

                let text_wrapping = new_value.text_wrapping.unwrap_or_default();

                let (width_meter, height_meter) = if text_wrapping {
                    (
                        new_value.width.unwrap_or(0.0),
                        new_value.height.unwrap_or(0.0),
                    )
                } else {
                    (0.0, 0.0)
                };

                if text_wrapping {
                    label_3d.set_autowrap_mode(AutowrapMode::WORD_SMART);
                    label_3d.set_width(200.0 * new_value.width.unwrap_or(16.0));
                } else {
                    label_3d.set_autowrap_mode(AutowrapMode::OFF);
                    label_3d.set_width(200.0 * new_value.width.unwrap_or(16.0));
                }

                let new_font = match new_value.font {
                    Some(0) => Font::FSansSerif,
                    Some(1) => Font::FSerif,
                    Some(2) => Font::FMonospace,
                    _ => Font::FSansSerif,
                };

                let (v_align, y_pos) = match text_align {
                    TextAlignMode::TamMiddleLeft
                    | TextAlignMode::TamMiddleRight
                    | TextAlignMode::TamMiddleCenter => (VerticalAlignment::CENTER, 0.0),
                    TextAlignMode::TamTopLeft
                    | TextAlignMode::TamTopRight
                    | TextAlignMode::TamTopCenter => (VerticalAlignment::TOP, 0.5),
                    TextAlignMode::TamBottomLeft
                    | TextAlignMode::TamBottomRight
                    | TextAlignMode::TamBottomCenter => (VerticalAlignment::BOTTOM, -0.5),
                };

                let (h_align, x_pos) = match text_align {
                    TextAlignMode::TamMiddleLeft
                    | TextAlignMode::TamTopLeft
                    | TextAlignMode::TamBottomLeft => (HorizontalAlignment::LEFT, -0.5),
                    TextAlignMode::TamMiddleRight
                    | TextAlignMode::TamTopRight
                    | TextAlignMode::TamBottomRight => (HorizontalAlignment::RIGHT, 0.5),
                    TextAlignMode::TamMiddleCenter
                    | TextAlignMode::TamTopCenter
                    | TextAlignMode::TamBottomCenter => (HorizontalAlignment::CENTER, 0.0),
                };

                label_3d.set_position(Vector3::new(width_meter * x_pos, height_meter * y_pos, 0.0));
                label_3d.set_vertical_alignment(v_align);
                label_3d.set_horizontal_alignment(h_align);

                if add_to_base {
                    label_3d.set_name(GString::from("TextShape"));
                    label_3d.set_font(new_font.get_font_resource());
                    node_3d.add_child(label_3d.upcast());
                }

                // TODO: missing properties
                // - padding (left/right/top/bottom)
                // - shadow (offsetX/offsetY/blur/color) (on Unity: it's actually an overlay)
                // - line_spacing (on Unity: it doesn't work)
                // - line_count (on Unity: it truncates instead of setting up the wrapping)
            }
        }
    }
}
