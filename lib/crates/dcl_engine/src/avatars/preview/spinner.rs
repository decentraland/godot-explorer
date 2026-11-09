//! The ring the preview shows over the avatar while a new look is dressed: the design system's
//! `Loader/Spinner`, drawn here because the Godot surface sits above every RN view in preview mode.

use std::f32::consts::TAU;

use godot::builtin::Side;
use godot::classes::control::{LayoutPreset, MouseFilter};
use godot::classes::{Control, IControl};
use godot::prelude::*;

/// 75 dp, the design system's default, in the preview's viewport units.
const SIZE: f32 = 150.0;
const TURNS_PER_SECOND: f32 = 1.0;
const TRACK: Color = Color::from_rgba(0.0, 0.0, 0.0, 0.4);
const RUBY: Color = Color::from_rgba(1.0, 0.176, 0.333, 1.0);

#[derive(GodotClass)]
#[class(base=Control, init)]
pub struct PreviewSpinner {
    base: Base<Control>,
    turn: f32,
}

#[godot_api]
impl IControl for PreviewSpinner {
    fn process(&mut self, delta: f64) {
        self.turn = (self.turn + delta as f32 * TAU * TURNS_PER_SECOND) % TAU;
        self.base_mut().queue_redraw();
    }

    fn draw(&mut self) {
        let size = self.base().get_size().x;
        let (center, radius, width) = (
            Vector2::splat(size * 0.5),
            size * 53.0 / 150.0,
            size * 16.0 / 150.0,
        );
        let start = 135.0_f32.to_radians() + self.turn;
        let mut base = self.base_mut();
        base.draw_arc_ex(center, radius, 0.0, TAU, 96, TRACK)
            .width(width)
            .antialiased(true)
            .done();
        base.draw_arc_ex(center, radius, start, start + TAU * 0.442, 48, RUBY)
            .width(width)
            .antialiased(true)
            .done();
    }
}

impl PreviewSpinner {
    /// A spinner of its fixed size, centred in its parent.
    pub fn centred() -> Gd<Self> {
        let mut spinner = Self::new_alloc();
        spinner.set_anchors_preset(LayoutPreset::CENTER);
        spinner.set_offset(Side::LEFT, -SIZE * 0.5);
        spinner.set_offset(Side::TOP, -SIZE * 0.5);
        spinner.set_offset(Side::RIGHT, SIZE * 0.5);
        spinner.set_offset(Side::BOTTOM, SIZE * 0.5);
        spinner.set_mouse_filter(MouseFilter::IGNORE);
        spinner.set_process(true);
        spinner
    }
}
