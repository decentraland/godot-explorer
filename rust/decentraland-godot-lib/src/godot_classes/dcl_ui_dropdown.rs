use std::{cell::RefCell, cmp::Ordering, rc::Rc};

use godot::{
    engine::{OptionButton, StyleBoxEmpty},
    prelude::*,
};

use crate::{
    dcl::components::{
        proto_components::{
            sdk::components::{common::Font, PbUiDropdown, PbUiDropdownResult},
            WrapToGodot,
        },
        SceneEntityId,
    },
    scene_runner::components::ui::scene_ui::UiResults,
};

#[derive(GodotClass)]
#[class(base=OptionButton)]
pub struct DclUiDropdown {
    #[base]
    base: Base<OptionButton>,

    current_font: Font,

    #[export]
    dcl_entity_id: SceneEntityId,

    ui_result: Option<Rc<RefCell<UiResults>>>,
}

#[godot_api]
impl INode for DclUiDropdown {
    fn init(base: Base<OptionButton>) -> Self {
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
            .add_theme_stylebox_override("hover".into(), style_box_empty.clone());
        self.base
            .add_theme_stylebox_override("pressed".into(), style_box_empty.clone());

        self.base.clone().connect(
            "item_selected".into(),
            self.base.callable("on_item_selected"),
        );
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

    pub fn change_value(&mut self, new_value: &PbUiDropdown) {
        let current_item_count = self.base.get_item_count();
        match current_item_count.cmp(&(new_value.options.len() as i32)) {
            Ordering::Greater => {
                for i in new_value.options.len() as i32..current_item_count {
                    self.base.remove_item(i);
                }
            }
            Ordering::Less => {
                for _ in current_item_count..new_value.options.len() as i32 {
                    self.base.add_item("".into());
                }
            }
            _ => {}
        }
        let current_item_count = new_value.options.len();
        for i in 0..current_item_count {
            self.base
                .set_item_text(i as i32, new_value.options[i].clone().into());
        }

        let current_selected_index = if current_item_count > 0 {
            self.base.get_selected()
        } else {
            -1
        };
        if current_selected_index == -1 {
            if !new_value.accept_empty && current_item_count > 0 {
                self.base.select(new_value.selected_index());
            }
            if let Some(label) = new_value.empty_label.as_ref() {
                self.base.set_text(label.into());
            }
        } else if let Some(new_selected_index) = new_value.selected_index.as_ref() {
            self.base.select(*new_selected_index);
        }

        let font_color = new_value
            .color
            .to_godot_or_else(godot::prelude::Color::WHITE);

        self.base
            .add_theme_color_override("font_color".into(), font_color);

        self.base
            .add_theme_font_size_override("font_size".into(), new_value.font_size.unwrap_or(10));

        self.base.set_disabled(new_value.disabled);

        // let (hor_text_align, _) = new_value
        //     .text_align
        //     .map(TextAlignMode::from_i32)
        //     .unwrap_or(Some(TextAlignMode::TamMiddleCenter))
        //     .unwrap()
        //     .to_godot();

        // self.base.set_align(hor_text_align);

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
