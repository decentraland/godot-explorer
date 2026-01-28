use godot::{
    classes::{control::LayoutPreset, text_server::JustificationFlag, ILabel, Label},
    global::{HorizontalAlignment, VerticalAlignment},
    prelude::*,
};

use crate::dcl::components::proto_components::sdk::components::{
    common::{Font, TextAlignMode},
    PbUiText, TextWrap,
};

#[derive(GodotClass)]
#[class(base=Label)]
pub struct DclUiText {
    base: Base<Label>,

    current_font: Font,
}

#[godot_api]
impl ILabel for DclUiText {
    fn init(base: Base<Label>) -> Self {
        Self {
            base,
            current_font: Font::FSansSerif,
        }
    }

    fn ready(&mut self) {
        let new_font_resource = self.current_font.get_font_resource();
        self.base_mut()
            .add_theme_font_override("font", &new_font_resource);
    }
}

#[godot_api]
impl DclUiText {
    pub fn change_value(&mut self, new_value: &PbUiText) {
        self.base_mut()
            .add_theme_font_size_override("font_size", new_value.font_size.unwrap_or(10));
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

        self.base_mut()
            .add_theme_color_override("font_color", font_color);
        self.base_mut()
            .add_theme_constant_override("line_spacing", 0);

        let text_align = new_value
            .text_align
            .map(TextAlignMode::from_i32)
            .unwrap_or(Some(TextAlignMode::TamMiddleCenter))
            .unwrap();

        let (hor_align, vert_align, _) = match text_align {
            TextAlignMode::TamTopLeft => (
                HorizontalAlignment::LEFT,
                VerticalAlignment::TOP,
                LayoutPreset::TOP_LEFT,
            ),
            TextAlignMode::TamTopCenter => (
                HorizontalAlignment::CENTER,
                VerticalAlignment::TOP,
                LayoutPreset::CENTER_TOP,
            ),
            TextAlignMode::TamTopRight => (
                HorizontalAlignment::RIGHT,
                VerticalAlignment::TOP,
                LayoutPreset::TOP_RIGHT,
            ),
            TextAlignMode::TamMiddleLeft => (
                HorizontalAlignment::LEFT,
                VerticalAlignment::CENTER,
                LayoutPreset::CENTER_LEFT,
            ),
            TextAlignMode::TamMiddleCenter => (
                HorizontalAlignment::CENTER,
                VerticalAlignment::CENTER,
                LayoutPreset::CENTER,
            ),
            TextAlignMode::TamMiddleRight => (
                HorizontalAlignment::RIGHT,
                VerticalAlignment::CENTER,
                LayoutPreset::CENTER_RIGHT,
            ),
            TextAlignMode::TamBottomLeft => (
                HorizontalAlignment::LEFT,
                VerticalAlignment::BOTTOM,
                LayoutPreset::BOTTOM_LEFT,
            ),
            TextAlignMode::TamBottomCenter => (
                HorizontalAlignment::CENTER,
                VerticalAlignment::BOTTOM,
                LayoutPreset::CENTER_BOTTOM,
            ),
            TextAlignMode::TamBottomRight => (
                HorizontalAlignment::RIGHT,
                VerticalAlignment::BOTTOM,
                LayoutPreset::BOTTOM_RIGHT,
            ),
        };

        self.base_mut().set_vertical_alignment(vert_align);
        self.base_mut().set_horizontal_alignment(hor_align);
        self.base_mut()
            .set_text(&clone_removing_tags(new_value.value.as_str()).to_godot());
        self.base_mut()
            .set_justification_flags(JustificationFlag::NONE);

        if new_value.font() != self.current_font {
            self.current_font = new_value.font();
            let new_font_resource = self.current_font.get_font_resource();
            self.base_mut()
                .add_theme_font_override("font", &new_font_resource);
        }

        if new_value.text_wrap_compat() == TextWrap::TwWrap {
            self.base_mut()
                .set_autowrap_mode(godot::classes::text_server::AutowrapMode::WORD_SMART);
        } else {
            self.base_mut()
                .set_autowrap_mode(godot::classes::text_server::AutowrapMode::OFF);
        }
    }
}

// temporary fix for removing <b>, </b>, <i>, </i> tags until is supportid
// this is a clone() with avoiding .replace
fn clone_removing_tags(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut skip = false;

    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '<' {
            if let Some(&next_char) = chars.peek() {
                if next_char == 'b'
                    || next_char == 'i'
                    || (next_char == '/' && chars.nth(1) == Some('b'))
                    || (next_char == '/' && chars.nth(1) == Some('i'))
                {
                    // Skip until closing '>'
                    skip = true;
                }
            }
        }

        if c == '>' && skip {
            skip = false;
            continue;
        }

        if !skip {
            result.push(c);
        }
    }

    result
}
