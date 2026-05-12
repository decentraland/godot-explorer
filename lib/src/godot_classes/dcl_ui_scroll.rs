use godot::{
    classes::{Control, IScrollContainer, ScrollContainer},
    obj::NewAlloc,
    prelude::*,
};

use crate::dcl::components::proto_components::sdk::components::ShowScrollBar;

#[derive(GodotClass)]
#[class(base=ScrollContainer)]
pub struct DclUiScroll {
    base: Base<ScrollContainer>,
    scroll_content: Gd<Control>,
}

#[godot_api]
impl IScrollContainer for DclUiScroll {
    fn init(base: Base<ScrollContainer>) -> Self {
        let mut content = Control::new_alloc();
        content.set_name("scroll_content");
        Self {
            base,
            scroll_content: content,
        }
    }

    fn ready(&mut self) {
        let content = self.scroll_content.clone();
        self.base_mut().add_child(&content.upcast::<Node>());
    }
}

#[godot_api]
impl DclUiScroll {
    pub fn content_node(&self) -> Gd<Control> {
        self.scroll_content.clone()
    }

    pub fn set_scroll_visible(&mut self, scroll_visible: ShowScrollBar) {
        use godot::classes::scroll_container::ScrollMode;
        let (h_mode, v_mode) = match scroll_visible {
            ShowScrollBar::SsbBoth => (ScrollMode::AUTO, ScrollMode::AUTO),
            ShowScrollBar::SsbOnlyHorizontal => (ScrollMode::AUTO, ScrollMode::SHOW_NEVER),
            ShowScrollBar::SsbOnlyVertical => (ScrollMode::SHOW_NEVER, ScrollMode::AUTO),
            ShowScrollBar::SsbHidden => (ScrollMode::SHOW_NEVER, ScrollMode::SHOW_NEVER),
        };
        self.base_mut().set_horizontal_scroll_mode(h_mode);
        self.base_mut().set_vertical_scroll_mode(v_mode);
    }

    pub fn set_scroll_position(&mut self, x: f32, y: f32) {
        self.base_mut().set_h_scroll(x as i32);
        self.base_mut().set_v_scroll(y as i32);
    }

    pub fn update_content_size(&mut self, width: f32, height: f32) {
        self.scroll_content
            .set_custom_minimum_size(Vector2::new(width, height));
    }
}
