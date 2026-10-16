//! A standalone HUD action button with three baked backgrounds (normal, pressed, selected)
//! swapped by state, and a glyph that swaps with it.

use godot::builtin::Side;
use godot::classes::{Button, IButton, StyleBoxEmpty, Texture2D, TextureRect};
use godot::global::HorizontalAlignment;
use godot::prelude::*;

/// The backgrounds are a 512 px disc centered on a 600 px canvas (the glow lives in the margin),
/// so a disc of D px draws its texture at D * 600/512.
const DISC_TO_CANVAS: f32 = 600.0 / 512.0;
const STYLE_SLOTS: [&str; 11] = [
    "normal",
    "normal_mirrored",
    "hover",
    "hover_mirrored",
    "pressed",
    "pressed_mirrored",
    "hover_pressed",
    "hover_pressed_mirrored",
    "disabled",
    "disabled_mirrored",
    "focus",
];

#[derive(GodotConvert, Var, Export, Clone, Copy, PartialEq, Debug)]
#[godot(via = i64)]
pub enum HudButtonSize {
    Small = 0,
    Medium = 1,
    Large = 2,
}

/// `(disc diameter = footprint, glyph size)` in pixels, per preset.
pub fn size_spec(size: HudButtonSize) -> (f32, i32) {
    match size {
        HudButtonSize::Small => (66.0, 46),
        HudButtonSize::Medium => (80.0, 48),
        HudButtonSize::Large => (96.0, 67),
    }
}

/// The background's offsets from the button center for a disc of `disc` px.
pub fn background_half_extent(disc: f32) -> f32 {
    disc * DISC_TO_CANVAS * 0.5
}

#[derive(GodotClass)]
#[class(base=Button, init)]
pub struct HudButton {
    base: Base<Button>,
    #[export]
    normal_texture: Option<Gd<Texture2D>>,
    #[export]
    pressed_texture: Option<Gd<Texture2D>>,
    #[export]
    selected_texture: Option<Gd<Texture2D>>,
    /// The glyph at rest.
    #[export]
    icon_normal: Option<Gd<Texture2D>>,
    /// The glyph while the finger is down or the toggle is latched; none keeps the normal one.
    #[export]
    icon_pressed: Option<Gd<Texture2D>>,
    #[export]
    #[init(val = HudButtonSize::Small)]
    size_preset: HudButtonSize,
    pressing: bool,
    #[init(node = "Background")]
    background: OnReady<Gd<TextureRect>>,
}

#[godot_api]
impl IButton for HudButton {
    fn ready(&mut self) {
        for slot in STYLE_SLOTS {
            self.base_mut()
                .add_theme_stylebox_override(slot, &StyleBoxEmpty::new_gd());
        }
        self.base_mut().set_expand_icon(true);
        self.base_mut()
            .set_icon_alignment(HorizontalAlignment::CENTER);
        let me = self.to_gd();
        self.base()
            .signals()
            .button_down()
            .connect_other(&me, |this: &mut Self| {
                this.pressing = true;
                this.refresh();
            });
        // `button_up` fires before toggle mode flips the pressed state, so settle next idle.
        self.base()
            .signals()
            .button_up()
            .connect_other(&me, |this: &mut Self| {
                this.pressing = false;
                this.base_mut().call_deferred("refresh", &[]);
            });
        self.base()
            .signals()
            .toggled()
            .connect_other(&me, |this: &mut Self, _on| this.refresh());
        self.apply_size();
        self.refresh();
    }
}

#[godot_api]
impl HudButton {
    #[func]
    fn refresh(&mut self) {
        let pressed = self.base().is_pressed();
        let mut texture = self.normal_texture.clone();
        if self.pressing && self.pressed_texture.is_some() {
            texture = self.pressed_texture.clone();
        } else if pressed && self.selected_texture.is_some() {
            texture = self.selected_texture.clone();
        }
        self.background.set_texture(texture.as_ref());
        let mut glyph = self.icon_normal.clone();
        if (self.pressing || pressed) && self.icon_pressed.is_some() {
            glyph = self.icon_pressed.clone();
        }
        self.base_mut().set_button_icon(glyph.as_ref());
    }
}

impl HudButton {
    /// The footprint is the disc, the glyph is capped, and the background is sized to the full
    /// texture and centered so the disc lands at exactly its size with the glow overflowing.
    fn apply_size(&mut self) {
        let (disc, glyph) = size_spec(self.size_preset);
        self.base_mut()
            .set_custom_minimum_size(Vector2::new(disc, disc));
        self.base_mut()
            .add_theme_constant_override("icon_max_width", glyph);
        let half = background_half_extent(disc);
        self.background.set_offset(Side::LEFT, -half);
        self.background.set_offset(Side::TOP, -half);
        self.background.set_offset(Side::RIGHT, half);
        self.background.set_offset(Side::BOTTOM, half);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_background_overflows_the_disc_by_the_glow() {
        let half = background_half_extent(66.0);
        assert!((half - 38.671875).abs() < 1e-4);
    }

    #[test]
    fn presets_grow() {
        assert!(size_spec(HudButtonSize::Small).0 < size_spec(HudButtonSize::Large).0);
    }
}
