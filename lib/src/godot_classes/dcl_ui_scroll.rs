use godot::{
    classes::{control::MouseFilter, Control, IScrollContainer, ScrollContainer},
    obj::NewAlloc,
    prelude::*,
};

use crate::scene_runner::components::ui::style::SCROLLBAR_GUTTER_PX;

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
        // Godot's default mouse_filter for plain Control is STOP, which
        // would capture touches landing on empty content and prevent them
        // from reaching the ScrollContainer's native drag-gesture detector.
        // Container subclasses (VBoxContainer, etc.) default to PASS for
        // this reason; we mirror that with IGNORE since scroll_content has
        // no behavior of its own beyond holding children.
        content.set_mouse_filter(MouseFilter::IGNORE);
        Self {
            base,
            scroll_content: content,
        }
    }

    fn ready(&mut self) {
        let content = self.scroll_content.clone();
        self.base_mut().add_child(&content.upcast::<Node>());
        // Drag threshold (in pixels) before a touch on a child Control is
        // re-routed to the ScrollContainer as a scroll gesture. Below this,
        // the child keeps the event (taps fire normally). Above it, Godot
        // transfers focus and the ScrollContainer starts scrolling.
        // 20 px is a starting feel — App UI uses 100 on dense settings
        // panels, but Scene UI rarely packs that many tap targets.
        self.base_mut().set_deadzone(20);

        // Force the visible scrollbar widget width to match the layout
        // gutter reserved in `style.rs`. Without this, Godot's default
        // theme renders a ~16 px scrollbar inside our 24 px gutter, leaving
        // a visible gap between the content edge and the scrollbar.
        let v_bar_opt = self.base_mut().get_v_scroll_bar();
        let h_bar_opt = self.base_mut().get_h_scroll_bar();
        if let Some(mut v_bar) = v_bar_opt.clone() {
            let mut size = v_bar.get_custom_minimum_size();
            size.x = SCROLLBAR_GUTTER_PX;
            v_bar.set_custom_minimum_size(size);
        }
        if let Some(mut h_bar) = h_bar_opt.clone() {
            let mut size = h_bar.get_custom_minimum_size();
            size.y = SCROLLBAR_GUTTER_PX;
            h_bar.set_custom_minimum_size(size);
        }

        // The project theme (`assets/themes/theme.tres`) defines different
        // StyleBoxFlat resources for HScrollBar and VScrollBar, so the two
        // axes render with mismatched colors out of the box. Copy V's
        // styles onto H so Scene UI scrollbars look consistent across axes
        // without touching the global theme (which app UI also inherits).
        if let (Some(v_bar), Some(mut h_bar)) = (v_bar_opt, h_bar_opt) {
            for name in ["grabber", "grabber_highlight", "grabber_pressed", "scroll"] {
                let style_name = StringName::from(name);
                if let Some(sb) = v_bar.get_theme_stylebox(&style_name) {
                    h_bar.add_theme_stylebox_override(&style_name, &sb);
                }
            }
        }
    }
}

#[godot_api]
impl DclUiScroll {
    pub fn content_node(&self) -> Gd<Control> {
        self.scroll_content.clone()
    }

    pub fn update_content_size(&mut self, width: f32, height: f32) {
        self.scroll_content
            .set_custom_minimum_size(Vector2::new(width, height));
    }
}
