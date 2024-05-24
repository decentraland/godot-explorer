use godot::{
    engine::{
        control::{LayoutPreset, LayoutPresetMode},
        global::{HorizontalAlignment, VerticalAlignment},
        Label,
    },
    prelude::*,
};

use crate::dcl::components::proto_components::sdk::components::{
    common::{Font, TextAlignMode},
    PbUiText,
};

#[derive(GodotClass)]
#[class(base=Label)]
pub struct DclUiText {
    #[base]
    base: Base<Label>,

    current_font: Font,
}

#[godot_api]
impl INode for DclUiText {
    fn init(base: Base<Label>) -> Self {
        Self {
            base,
            current_font: Font::FSansSerif,
        }
    }

    fn ready(&mut self) {
        self.base
            .add_theme_font_override("font".into(), self.current_font.get_font_resource());
    }
}

#[godot_api]
impl DclUiText {
    pub fn change_value(&mut self, new_value: &PbUiText) {
        self.base
            .add_theme_font_size_override("font_size".into(), new_value.font_size.unwrap_or(10));
        let font_color = new_value
            .color
            .as_ref()
            .map(|v| godot::prelude::Color {
                r: v.r,
                g: v.g,
                b: v.b,
                a: v.a,
            })
            .unwrap_or(godot::prelude::Color::WHITE);
        let outline_font_color = new_value
            .outline_color
            .as_ref()
            .map(|v| godot::prelude::Color {
                r: v.r,
                g: v.g,
                b: v.b,
                a: v.a,
            })
            .unwrap_or(godot::prelude::Color::BLACK);
        let outline_width = new_value.outline_width.unwrap_or(0.0) as i32;

        self.base
            .add_theme_color_override("font_color".into(), font_color);
        self.base
            .add_theme_color_override("font_outline_color".into(), outline_font_color);
        self.base
            .add_theme_constant_override("outline_size".into(), outline_width);

        let text_align = new_value
            .text_align
            .map(TextAlignMode::from_i32)
            .unwrap_or(Some(TextAlignMode::TamMiddleCenter))
            .unwrap();

        let (hor_align, vert_align, anchor) = match text_align {
            TextAlignMode::TamTopLeft => (
                HorizontalAlignment::HORIZONTAL_ALIGNMENT_LEFT,
                VerticalAlignment::VERTICAL_ALIGNMENT_TOP,
                LayoutPreset::PRESET_TOP_LEFT,
            ),
            TextAlignMode::TamTopCenter => (
                HorizontalAlignment::HORIZONTAL_ALIGNMENT_CENTER,
                VerticalAlignment::VERTICAL_ALIGNMENT_TOP,
                LayoutPreset::PRESET_CENTER_TOP,
            ),
            TextAlignMode::TamTopRight => (
                HorizontalAlignment::HORIZONTAL_ALIGNMENT_RIGHT,
                VerticalAlignment::VERTICAL_ALIGNMENT_TOP,
                LayoutPreset::PRESET_TOP_RIGHT,
            ),
            TextAlignMode::TamMiddleLeft => (
                HorizontalAlignment::HORIZONTAL_ALIGNMENT_LEFT,
                VerticalAlignment::VERTICAL_ALIGNMENT_CENTER,
                LayoutPreset::PRESET_CENTER_LEFT,
            ),
            TextAlignMode::TamMiddleCenter => (
                HorizontalAlignment::HORIZONTAL_ALIGNMENT_CENTER,
                VerticalAlignment::VERTICAL_ALIGNMENT_CENTER,
                LayoutPreset::PRESET_CENTER,
            ),
            TextAlignMode::TamMiddleRight => (
                HorizontalAlignment::HORIZONTAL_ALIGNMENT_RIGHT,
                VerticalAlignment::VERTICAL_ALIGNMENT_CENTER,
                LayoutPreset::PRESET_CENTER_RIGHT,
            ),
            TextAlignMode::TamBottomLeft => (
                HorizontalAlignment::HORIZONTAL_ALIGNMENT_LEFT,
                VerticalAlignment::VERTICAL_ALIGNMENT_BOTTOM,
                LayoutPreset::PRESET_BOTTOM_LEFT,
            ),
            TextAlignMode::TamBottomCenter => (
                HorizontalAlignment::HORIZONTAL_ALIGNMENT_CENTER,
                VerticalAlignment::VERTICAL_ALIGNMENT_BOTTOM,
                LayoutPreset::PRESET_CENTER_BOTTOM,
            ),
            TextAlignMode::TamBottomRight => (
                HorizontalAlignment::HORIZONTAL_ALIGNMENT_RIGHT,
                VerticalAlignment::VERTICAL_ALIGNMENT_BOTTOM,
                LayoutPreset::PRESET_BOTTOM_RIGHT,
            ),
        };

        self.base.set_vertical_alignment(vert_align);
        self.base.set_horizontal_alignment(hor_align);
        self.base.set_text(new_value.value.clone().into());

        if new_value.font() != self.current_font {
            self.current_font = new_value.font();
            self.base
                .add_theme_font_override("font".into(), self.current_font.get_font_resource());
        }

        if new_value.text_wrapping() {
            self.base
                .set_autowrap_mode(godot::engine::text_server::AutowrapMode::AUTOWRAP_WORD_SMART);
        } else {
            self.base
                .set_autowrap_mode(godot::engine::text_server::AutowrapMode::AUTOWRAP_OFF);
        }
        self.base
            .set_anchors_and_offsets_preset_ex(anchor)
            .resize_mode(LayoutPresetMode::PRESET_MODE_KEEP_SIZE)
            .done();
    }
}
