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
        ui_text_tags::strip_tags_extract_first_color,
    },
    scene_runner::scene::Scene,
};
use godot::{
    engine::{
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

                // Process text: strip Unity tags and extract first color
                let (display_text, tag_color) =
                    if let Some(strip_result) = strip_tags_extract_first_color(&new_value.text) {
                        let color = strip_result.first_color.and_then(|c| parse_color(&c));
                        (strip_result.text, color)
                    } else {
                        (new_value.text.clone(), None)
                    };

                // Use tag color if found, otherwise use the default text_color
                let text_color = tag_color
                    .map(|c| Color::from_rgba(c.0, c.1, c.2, opacity))
                    .unwrap_or_else(|| {
                        new_value
                            .text_color
                            .map(|color| Color::from_rgba(color.r, color.g, color.b, opacity))
                            .unwrap_or(Color::from_rgba(1.0, 1.0, 1.0, opacity))
                    });

                let outline_color = new_value
                    .outline_color
                    .map(|color| Color::from_rgba(color.r, color.g, color.b, opacity))
                    .unwrap_or(Color::from_rgba(1.0, 1.0, 1.0, opacity));

                label_3d.set_text(GString::from(display_text));
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

/// Parses a color string (named color or hex) into RGB values (0.0-1.0)
fn parse_color(color: &str) -> Option<(f32, f32, f32)> {
    let color = color.trim().to_lowercase();

    // Named colors (common Unity/CSS colors)
    match color.as_str() {
        "red" => return Some((1.0, 0.0, 0.0)),
        "green" => return Some((0.0, 0.5, 0.0)),
        "blue" => return Some((0.0, 0.0, 1.0)),
        "white" => return Some((1.0, 1.0, 1.0)),
        "black" => return Some((0.0, 0.0, 0.0)),
        "yellow" => return Some((1.0, 1.0, 0.0)),
        "cyan" => return Some((0.0, 1.0, 1.0)),
        "magenta" => return Some((1.0, 0.0, 1.0)),
        "gray" | "grey" => return Some((0.5, 0.5, 0.5)),
        "orange" => return Some((1.0, 0.65, 0.0)),
        "purple" => return Some((0.5, 0.0, 0.5)),
        "pink" => return Some((1.0, 0.75, 0.8)),
        "brown" => return Some((0.65, 0.16, 0.16)),
        "lime" => return Some((0.0, 1.0, 0.0)),
        "navy" => return Some((0.0, 0.0, 0.5)),
        "teal" => return Some((0.0, 0.5, 0.5)),
        "olive" => return Some((0.5, 0.5, 0.0)),
        "maroon" => return Some((0.5, 0.0, 0.0)),
        "aqua" => return Some((0.0, 1.0, 1.0)),
        "silver" => return Some((0.75, 0.75, 0.75)),
        "fuchsia" => return Some((1.0, 0.0, 1.0)),
        _ => {}
    }

    // Hex color (#RGB, #RRGGBB, or #RRGGBBAA)
    if let Some(hex) = color.strip_prefix('#') {
        match hex.len() {
            3 => {
                // #RGB -> expand to #RRGGBB
                let r = u8::from_str_radix(&hex[0..1], 16).ok()?;
                let g = u8::from_str_radix(&hex[1..2], 16).ok()?;
                let b = u8::from_str_radix(&hex[2..3], 16).ok()?;
                return Some((
                    (r * 17) as f32 / 255.0,
                    (g * 17) as f32 / 255.0,
                    (b * 17) as f32 / 255.0,
                ));
            }
            6 | 8 => {
                // #RRGGBB or #RRGGBBAA (ignore alpha)
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                return Some((r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0));
            }
            _ => {}
        }
    }

    None
}
