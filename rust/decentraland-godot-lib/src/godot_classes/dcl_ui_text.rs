use godot::{
    engine::{
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

impl Font {
    pub fn get_font_path(self) -> &'static str {
        match self {
            Font::FSansSerif => "res://assets/themes/fonts/noto/NotoSans-Regular.ttf",
            Font::FSerif => "res://assets/themes/fonts/noto/NotoSerif-Regular.ttf",
            Font::FMonospace => "res://assets/themes/fonts/noto/NotoSansMono-Regular.ttf",
        }
    }
}

#[godot_api]
impl NodeVirtual for DclUiText {
    fn init(base: Base<Label>) -> Self {
        Self {
            base,
            current_font: Font::FSansSerif,
        }
    }

    fn ready(&mut self) {
        let font_resource = load(self.current_font.get_font_path());
        self.base
            .add_theme_font_override("font".into(), font_resource);
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
        self.base
            .add_theme_color_override("font_color".into(), font_color);

        let text_align = new_value
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

        self.base.set_vertical_alignment(vert_align);
        self.base.set_horizontal_alignment(hor_align);
        self.base.set_text(new_value.value.clone().into());

        if new_value.font() != self.current_font {
            self.current_font = new_value.font();
            let font_resource = load(self.current_font.get_font_path());
            self.base
                .add_theme_font_override("font".into(), font_resource);
        }
    }
}
