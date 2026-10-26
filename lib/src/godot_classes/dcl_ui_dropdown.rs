use std::{cell::RefCell, cmp::Ordering, rc::Rc};

use godot::{
    classes::{IOptionButton, OptionButton, StyleBoxEmpty},
    prelude::*,
};

use crate::{
    content::content_mapping::ContentMappingAndUrlRef,
    dcl::components::{
        proto_components::{
            sdk::components::{PbUiDropdown, PbUiDropdownResult},
            WrapToGodot,
        },
        SceneEntityId,
    },
    godot_classes::font::{FontStyle, SceneFont},
    scene_runner::components::ui::scene_ui::UiResults,
};

#[derive(GodotClass)]
#[class(base=OptionButton)]
pub struct DclUiDropdown {
    base: Base<OptionButton>,

    scene_font: SceneFont,

    #[export]
    dcl_entity_id: SceneEntityId,

    ui_result: Option<Rc<RefCell<UiResults>>>,
}

#[godot_api]
impl IOptionButton for DclUiDropdown {
    fn init(base: Base<OptionButton>) -> Self {
        Self {
            base,
            scene_font: SceneFont::default(),
            ui_result: None,
            dcl_entity_id: SceneEntityId::ROOT,
        }
    }

    fn ready(&mut self) {
        let style_box_empty: Gd<godot::classes::StyleBox> = StyleBoxEmpty::new_gd().upcast();
        self.apply_font();
        self.base_mut()
            .add_theme_stylebox_override("normal", &style_box_empty.clone());
        self.base_mut()
            .add_theme_stylebox_override("focus", &style_box_empty.clone());
        self.base_mut()
            .add_theme_stylebox_override("hover", &style_box_empty.clone());
        self.base_mut()
            .add_theme_stylebox_override("pressed", &style_box_empty.clone());

        let callable_on_item_selected = self.base().callable("on_item_selected");
        self.base_mut()
            .clone()
            .connect("item_selected", &callable_on_item_selected);
    }
}

#[godot_api]
impl DclUiDropdown {
    #[func]
    pub fn on_item_selected(&mut self, index: i32) {
        let Some(ui_result) = self.ui_result.as_ref() else {
            return;
        };

        let mut ui_result = ui_result.borrow_mut();
        let value = ui_result
            .dropdown_results
            .entry(self.dcl_entity_id)
            .or_insert(PbUiDropdownResult::default());
        value.value = index;
    }

    #[func]
    fn apply_font(&mut self) {
        let font = self.scene_font.get(FontStyle::Regular);
        self.base_mut().add_theme_font_override("font", &font);
        // The options list is a separate PopupMenu with its own theme.
        if let Some(mut popup) = self.base().get_popup() {
            popup.add_theme_font_override("font", &font);
        }
    }

    pub fn change_value(
        &mut self,
        new_value: &PbUiDropdown,
        content_mapping: &ContentMappingAndUrlRef,
    ) {
        let current_item_count = self.base().get_item_count();
        match current_item_count.cmp(&(new_value.options.len() as i32)) {
            Ordering::Greater => {
                for i in new_value.options.len() as i32..current_item_count {
                    self.base_mut().remove_item(i);
                }
            }
            Ordering::Less => {
                for _ in current_item_count..new_value.options.len() as i32 {
                    self.base_mut().add_item("");
                }
            }
            _ => {}
        }
        let current_item_count = new_value.options.len();
        for i in 0..current_item_count {
            self.base_mut()
                .set_item_text(i as i32, &new_value.options[i].to_godot());
        }

        let current_selected_index = if current_item_count > 0 {
            self.base().get_selected()
        } else {
            -1
        };
        if current_selected_index == -1 {
            if !new_value.accept_empty && current_item_count > 0 {
                self.base_mut().select(new_value.selected_index());
            }
            if let Some(label) = new_value.empty_label.as_ref() {
                self.base_mut().set_text(&label.to_godot());
            }
        } else if let Some(new_selected_index) = new_value.selected_index.as_ref() {
            self.base_mut().select(*new_selected_index);
        }

        let font_color = new_value
            .color
            .to_godot_or_else(godot::prelude::Color::WHITE);

        self.base_mut()
            .add_theme_color_override("font_color", font_color);

        let font_size = new_value.font_size.unwrap_or(10);
        self.base_mut()
            .add_theme_font_size_override("font_size", font_size);
        if let Some(mut popup) = self.base().get_popup() {
            popup.add_theme_font_size_override("font_size", font_size);
        }

        self.base_mut().set_disabled(new_value.disabled);

        // let (hor_text_align, _) = new_value
        //     .text_align
        //     .map(TextAlignMode::from_i32)
        //     .unwrap_or(Some(TextAlignMode::TamMiddleCenter))
        //     .unwrap()
        //     .to_godot();

        // self.base.set_align(hor_text_align);

        if self.scene_font.update(
            new_value.font(),
            new_value.font_src.as_deref(),
            content_mapping,
        ) {
            self.apply_font();
            // No relayout: the size comes from UiTransform, not the text.
            self.scene_font
                .on_loaded(&self.base().callable("apply_font"));
        }
    }

    pub fn set_ui_result(&mut self, ui_result: Rc<RefCell<UiResults>>) {
        self.ui_result = Some(ui_result);
    }
}
