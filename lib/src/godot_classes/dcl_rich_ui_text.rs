use godot::{
    classes::{control::LayoutPreset, IRichTextLabel, RichTextLabel},
    global::{HorizontalAlignment, VerticalAlignment},
    prelude::*,
};

use crate::dcl::components::proto_components::sdk::components::{
    common::{Font, TextAlignMode},
    PbUiText, TextWrap,
};

#[derive(GodotClass)]
#[class(base=RichTextLabel)]
pub struct DclRichUiText {
    base: Base<RichTextLabel>,

    current_font: Font,
}

#[godot_api]
impl IRichTextLabel for DclRichUiText {
    fn init(base: Base<RichTextLabel>) -> Self {
        Self {
            base,
            current_font: Font::FSansSerif,
        }
    }

    fn ready(&mut self) {
        let font = self.current_font;

        self.base_mut()
            .add_theme_font_override("normal_font", &font.get_font_resource());
        self.base_mut()
            .add_theme_font_override("bold_font", &font.get_font_bold_resource());
        self.base_mut()
            .add_theme_font_override("italics_font", &font.get_font_italic_resource());
        self.base_mut()
            .add_theme_font_override("bold_italics_font", &font.get_font_bold_italic_resource());
        self.base_mut().set_use_bbcode(true);

        // Configure to behave like Label in terms of layout:
        // - Disable scrolling (Label doesn't have scroll)
        self.base_mut().set_scroll_active(false);
        self.base_mut().set_scroll_follow(false);

        // - Disable text selection (Label doesn't support selection)
        self.base_mut().set_selection_enabled(false);

        // - Enable fit_content for auto-sizing similar to Label
        //   This makes RichTextLabel auto-size to its content like Label does
        self.base_mut().set_fit_content(true);

        // - Disable clip_contents to allow theme effects (outline/shadow) to extend beyond bounds
        //   This matches Label behavior which doesn't clip theme effects
        self.base_mut().set_clip_contents(false);
    }
}

#[godot_api]
impl DclRichUiText {
    pub fn change_value(&mut self, new_value: &PbUiText, converted_text: &str) {
        self.base_mut()
            .add_theme_font_size_override("normal_font_size", new_value.font_size.unwrap_or(10));
        self.base_mut()
            .add_theme_font_size_override("bold_font_size", new_value.font_size.unwrap_or(10));
        self.base_mut()
            .add_theme_font_size_override("italics_font_size", new_value.font_size.unwrap_or(10));
        self.base_mut().add_theme_font_size_override(
            "bold_italics_font_size",
            new_value.font_size.unwrap_or(10),
        );

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
            .add_theme_color_override("default_color", font_color);

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

        // RichTextLabel supports both horizontal and vertical alignment
        // Use call() method to ensure compatibility across Godot versions
        self.base_mut().call(
            "set_horizontal_alignment",
            &[godot::prelude::Variant::from(hor_align.ord())],
        );
        self.base_mut().call(
            "set_vertical_alignment",
            &[godot::prelude::Variant::from(vert_align.ord())],
        );
        self.base_mut().call(
            "set_justification_flags",
            &[godot::prelude::Variant::from(0)],
        );

        // Set justification flags to NONE (no text justification, just alignment)
        // self.base_mut().set_justification_flags(
        //     godot::classes::text_server::JustificationFlag::JUSTIFICATION_NONE,
        // );

        // Set the BBCode text with converted Unity tags to Godot BBCode
        self.base_mut().set_text(converted_text);

        if new_value.font() != self.current_font {
            self.current_font = new_value.font();
            let new_font_resource = self.current_font.get_font_resource();
            self.base_mut()
                .add_theme_font_override("normal_font", &new_font_resource.clone());
            self.base_mut()
                .add_theme_font_override("bold_font", &new_font_resource.clone());
            self.base_mut()
                .add_theme_font_override("italics_font", &new_font_resource.clone());
            self.base_mut()
                .add_theme_font_override("bold_italics_font", &new_font_resource);
        }

        if new_value.text_wrap_compat() == TextWrap::TwWrap {
            self.base_mut()
                .set_autowrap_mode(godot::classes::text_server::AutowrapMode::WORD_SMART);
        } else {
            self.base_mut()
                .set_autowrap_mode(godot::classes::text_server::AutowrapMode::OFF);
        }

        // Note: RichTextLabel has some differences from Label:
        // - Uses bbcode/text property instead of text
        // - Doesn't support vertical_alignment directly
        // - Alignment for RichTextLabel may need BBCode tags like [center], [right]
        // For vertical alignment, we rely on the Control's anchors/layout
    }
}
