use std::{cell::RefCell, rc::Rc};

use godot::{
    classes::{ILineEdit, LineEdit, StyleBoxEmpty},
    global::{HorizontalAlignment, VerticalAlignment},
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
    base: Base<LineEdit>,

    current_font: Font,

    #[export]
    dcl_entity_id: SceneEntityId,

    ui_result: Option<Rc<RefCell<UiResults>>>,
}

#[godot_api]
impl ILineEdit for DclUiInput {
    fn init(base: Base<LineEdit>) -> Self {
        Self {
            base,
            current_font: Font::FSansSerif,
            ui_result: None,
            dcl_entity_id: SceneEntityId::ROOT,
        }
    }

    fn ready(&mut self) {
        let style_box_empty: Gd<godot::classes::StyleBox> = StyleBoxEmpty::new_gd().upcast();
        let new_font_resource = self.current_font.get_font_resource();
        self.base_mut()
            .add_theme_font_override("font", &new_font_resource);
        self.base_mut()
            .add_theme_stylebox_override("normal", &style_box_empty);
        self.base_mut()
            .add_theme_stylebox_override("focus", &style_box_empty);
        self.base_mut()
            .add_theme_stylebox_override("read_only", &style_box_empty);

        let callable_on_text_changed = self.base().callable("on_text_changed");
        let callable_on_text_submitted = self.base().callable("on_text_submitted");
        self.base_mut()
            .connect("text_changed", &callable_on_text_changed);
        self.base_mut()
            .connect("text_submitted", &callable_on_text_submitted);
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
        self.base_mut().set_placeholder(&new_value.placeholder);

        let font_placeholder_color = new_value
            .placeholder_color
            .to_godot_or_else(godot::prelude::Color::from_rgba(0.3, 0.3, 0.3, 1.0));

        self.base_mut()
            .add_theme_color_override("font_placeholder_color", font_placeholder_color);

        let font_color = new_value
            .color
            .to_godot_or_else(godot::prelude::Color::WHITE);

        self.base_mut()
            .add_theme_color_override("font_color", font_color);

        self.base_mut()
            .add_theme_font_size_override("font_size", new_value.font_size.unwrap_or(10));

        self.base_mut().set_editable(!new_value.disabled);

        if let Some(text_value) = new_value.value.as_ref() {
            self.base_mut().set_text(text_value);
        }

        let text_align = new_value
            .text_align
            .map(TextAlignMode::from_i32)
            .unwrap_or(Some(TextAlignMode::TamMiddleCenter))
            .unwrap();

        let (hor_align, _vert_align) = match text_align {
            TextAlignMode::TamTopLeft => (HorizontalAlignment::LEFT, VerticalAlignment::TOP),
            TextAlignMode::TamTopCenter => (HorizontalAlignment::CENTER, VerticalAlignment::TOP),
            TextAlignMode::TamTopRight => (HorizontalAlignment::RIGHT, VerticalAlignment::TOP),
            TextAlignMode::TamMiddleLeft => (HorizontalAlignment::LEFT, VerticalAlignment::CENTER),
            TextAlignMode::TamMiddleCenter => {
                (HorizontalAlignment::CENTER, VerticalAlignment::CENTER)
            }
            TextAlignMode::TamMiddleRight => {
                (HorizontalAlignment::RIGHT, VerticalAlignment::CENTER)
            }
            TextAlignMode::TamBottomLeft => (HorizontalAlignment::LEFT, VerticalAlignment::BOTTOM),
            TextAlignMode::TamBottomCenter => {
                (HorizontalAlignment::CENTER, VerticalAlignment::BOTTOM)
            }
            TextAlignMode::TamBottomRight => {
                (HorizontalAlignment::RIGHT, VerticalAlignment::BOTTOM)
            }
        };

        self.base_mut().set_horizontal_alignment(hor_align);

        if new_value.font() != self.current_font {
            self.current_font = new_value.font();
            let new_font_resource = self.current_font.get_font_resource();
            self.base_mut()
                .add_theme_font_override("font", &new_font_resource);
        }
    }

    pub fn set_ui_result(&mut self, ui_result: Rc<RefCell<UiResults>>) {
        self.ui_result = Some(ui_result);
    }
}
