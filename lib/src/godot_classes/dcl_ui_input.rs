use std::{cell::RefCell, rc::Rc};

use godot::{
    engine::{
        global::{HorizontalAlignment, VerticalAlignment},
        LineEdit, StyleBoxEmpty,
    },
    prelude::*,
};

use crate::{
    dcl::components::{
        proto_components::{
            sdk::components::{
                common::{Font, TextAlignMode},
                PbUiInput, PbUiInputResult,
            },
            WrapToGodot,
        },
        SceneEntityId,
    },
    scene_runner::components::ui::scene_ui::UiResults,
};

#[derive(GodotClass)]
#[class(base=LineEdit)]
pub struct DclUiInput {
    #[base]
    base: Base<LineEdit>,

    current_font: Font,

    #[export]
    dcl_entity_id: SceneEntityId,

    ui_result: Option<Rc<RefCell<UiResults>>>,
}

#[godot_api]
impl INode for DclUiInput {
    fn init(base: Base<LineEdit>) -> Self {
        Self {
            base,
            current_font: Font::FSansSerif,
            ui_result: None,
            dcl_entity_id: SceneEntityId::ROOT,
        }
    }

    fn ready(&mut self) {
        let style_box_empty: Gd<godot::engine::StyleBox> = StyleBoxEmpty::new().upcast();
        self.base
            .add_theme_font_override("font".into(), self.current_font.get_font_resource());
        self.base
            .add_theme_stylebox_override("normal".into(), style_box_empty.clone());
        self.base
            .add_theme_stylebox_override("focus".into(), style_box_empty.clone());
        self.base
            .add_theme_stylebox_override("read_only".into(), style_box_empty.clone());

        self.base
            .clone()
            .connect("text_changed".into(), self.base.callable("on_text_changed"));
        self.base.clone().connect(
            "text_submitted".into(),
            self.base.callable("on_text_submitted"),
        );
    }
}

#[godot_api]
impl DclUiInput {
    #[func]
    pub fn on_text_changed(&mut self, new_text: GString) {
        let Some(ui_result) = self.ui_result.as_ref() else {
            return;
        };

        let mut ui_result = ui_result.borrow_mut();
        let value = ui_result
            .input_results
            .entry(self.dcl_entity_id)
            .or_insert(PbUiInputResult::default());
        value.value = new_text.to_string();
        value.is_submit = Some(false);
    }

    #[func]
    pub fn on_text_submitted(&mut self, new_text: GString) {
        let Some(ui_result) = self.ui_result.as_ref() else {
            return;
        };

        let mut ui_result = ui_result.borrow_mut();
        let value = ui_result
            .input_results
            .entry(self.dcl_entity_id)
            .or_insert(PbUiInputResult::default());
        value.value = new_text.to_string();
        value.is_submit = Some(true);
    }

    pub fn change_value(&mut self, new_value: &PbUiInput) {
        self.base
            .set_placeholder(new_value.placeholder.clone().into());

        let font_placeholder_color = new_value
            .placeholder_color
            .to_godot_or_else(godot::prelude::Color::from_rgba(0.3, 0.3, 0.3, 1.0));

        self.base
            .add_theme_color_override("font_placeholder_color".into(), font_placeholder_color);

        let font_color = new_value
            .color
            .to_godot_or_else(godot::prelude::Color::WHITE);

        self.base
            .add_theme_color_override("font_color".into(), font_color);

        self.base
            .add_theme_font_size_override("font_size".into(), new_value.font_size.unwrap_or(10));

        self.base.set_editable(!new_value.disabled);

        if let Some(text_value) = new_value.value.as_ref() {
            self.base.set_text(text_value.clone().into());
        }

        let text_align = new_value
            .text_align
            .map(TextAlignMode::from_i32)
            .unwrap_or(Some(TextAlignMode::TamMiddleCenter))
            .unwrap();

        let (hor_align, _vert_align) = match text_align {
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

        self.base.set_horizontal_alignment(hor_align);

        if new_value.font() != self.current_font {
            self.current_font = new_value.font();
            self.base
                .add_theme_font_override("font".into(), self.current_font.get_font_resource());
        }
    }

    pub fn set_ui_result(&mut self, ui_result: Rc<RefCell<UiResults>>) {
        self.ui_result = Some(ui_result);
    }
}
