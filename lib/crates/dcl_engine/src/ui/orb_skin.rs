//! The orb behind a circular touch button: a pre-baked texture drawn behind the glyph and swapped
//! with the button's state (default, pressed, hold, disabled).

use godot::classes::control::MouseFilter;
use godot::classes::texture_rect::{ExpandMode, StretchMode};
use godot::classes::{Button, ITextureRect, Texture2D, TextureRect};
use godot::prelude::*;

const DIR: &str = "res://src/ui/components/atoms/buttons/button_touch_action/";
/// The orb PNGs are 600x600 with a 512x512 opaque circle: the rect is drawn larger than the
/// button so the visible circle fills it and the transparent padding spills outside.
const ORB_OVERSCAN: f32 = 600.0 / 512.0;
const ICON_LIGHT: Color = Color::from_rgba(0.9098039, 0.8509804, 1.0, 1.0);
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
/// The glyph stays light in every state (the theme flips it dark for the old white pressed fill).
const ICON_COLOR_SLOTS: [&str; 4] = [
    "icon_pressed_color",
    "icon_hover_pressed_color",
    "font_pressed_color",
    "font_hover_pressed_color",
];
const ALL_ICON_COLOR_SLOTS: [&str; 6] = [
    "icon_normal_color",
    "icon_hover_color",
    "icon_pressed_color",
    "icon_hover_pressed_color",
    "icon_focus_color",
    "icon_disabled_color",
];

#[derive(GodotClass)]
#[class(base=TextureRect, init)]
pub struct OrbSkin {
    base: Base<TextureRect>,
    #[export]
    default_texture: Option<Gd<Texture2D>>,
    #[export]
    pressed_texture: Option<Gd<Texture2D>>,
    #[export]
    hold_texture: Option<Gd<Texture2D>>,
    #[export]
    disabled_texture: Option<Gd<Texture2D>>,
    #[export]
    normal_icon: Option<Gd<Texture2D>>,
    #[export]
    pressed_icon: Option<Gd<Texture2D>>,
    #[export]
    hold_icon: Option<Gd<Texture2D>>,
    /// Tints the button's glyph by state; off keeps the theme's glyph colors.
    #[export]
    recolor_icon_by_state: bool,
    #[export]
    #[init(val = Color::from_html("DFD0FF").unwrap_or(Color::WHITE))]
    default_icon_color: Color,
    #[export]
    #[init(val = Color::from_html("DF9CFF").unwrap_or(Color::WHITE))]
    pressed_icon_color: Color,
    held: bool,
}

fn orb_texture(file: &str) -> Option<Gd<Texture2D>> {
    godot::tools::try_load::<Texture2D>(&format!("{DIR}{file}")).ok()
}

#[godot_api]
impl ITextureRect for OrbSkin {
    fn ready(&mut self) {
        self.default_texture
            .get_or_insert_with(|| orb_texture("orb_default.png").unwrap_or_default());
        self.pressed_texture = self
            .pressed_texture
            .take()
            .or_else(|| orb_texture("orb_pressed.png"));
        self.hold_texture = self
            .hold_texture
            .take()
            .or_else(|| orb_texture("orb-hold.png"));
        self.disabled_texture = self
            .disabled_texture
            .take()
            .or_else(|| orb_texture("orb_disabled.png"));

        self.base_mut().set_draw_behind_parent(true);
        self.base_mut().set_mouse_filter(MouseFilter::IGNORE);
        self.base_mut().set_custom_minimum_size(Vector2::ZERO);
        self.base_mut().set_expand_mode(ExpandMode::IGNORE_SIZE);
        self.base_mut()
            .set_stretch_mode(StretchMode::KEEP_ASPECT_CENTERED);

        if let Some(mut button) = self.button() {
            self.apply_style_overrides(&mut button);
            self.fit();
            let me = self.to_gd();
            button
                .signals()
                .resized()
                .connect_other(&me, |this: &mut Self| this.fit());
            button
                .signals()
                .button_down()
                .connect_other(&me, |this: &mut Self| this.refresh());
            // `button_up` fires before the button lowers its pressed state, so settle next idle.
            button
                .signals()
                .button_up()
                .connect_other(&me, |this: &mut Self| {
                    this.base_mut().call_deferred("refresh", &[]);
                });
            button
                .signals()
                .toggled()
                .connect_other(&me, |this: &mut Self, _on| this.refresh());
        }
        self.refresh();
    }
}

#[godot_api]
impl OrbSkin {
    /// The external latch of the hold look (kept through press and release until cleared).
    #[func]
    pub fn set_hold(&mut self, on: bool) {
        if self.held != on {
            self.held = on;
            self.refresh();
        }
    }

    /// Sets the per-state glyphs; the button shows the one matching its state.
    #[func]
    pub fn set_icons(
        &mut self,
        normal: Option<Gd<Texture2D>>,
        pressed: Option<Gd<Texture2D>>,
        hold: Option<Gd<Texture2D>>,
    ) {
        self.normal_icon = normal;
        self.pressed_icon = pressed;
        self.hold_icon = hold;
        if self.base().is_node_ready() {
            self.refresh();
        }
    }

    #[func]
    fn refresh(&mut self) {
        let button = self.button();
        let mut texture = self.default_texture.clone();
        let mut icon = self.normal_icon.clone();
        let mut pressed_state = false;
        if button.as_ref().is_some_and(|b| b.is_disabled()) {
            if self.disabled_texture.is_some() {
                texture = self.disabled_texture.clone();
            }
        } else if self.held {
            pressed_state = true;
            if self.hold_texture.is_some() {
                texture = self.hold_texture.clone();
            }
            if self.hold_icon.is_some() {
                icon = self.hold_icon.clone();
            }
        } else if button.as_ref().is_some_and(|b| b.is_pressed()) {
            pressed_state = true;
            if self.pressed_texture.is_some() {
                texture = self.pressed_texture.clone();
            }
            if self.pressed_icon.is_some() {
                icon = self.pressed_icon.clone();
            }
        }
        self.base_mut().set_texture(texture.as_ref());
        let Some(mut button) = button else {
            return;
        };
        if self.normal_icon.is_some() || self.pressed_icon.is_some() || self.hold_icon.is_some() {
            button.set_button_icon(icon.as_ref());
        }
        if self.recolor_icon_by_state {
            let color = if pressed_state {
                self.pressed_icon_color
            } else {
                self.default_icon_color
            };
            for slot in ALL_ICON_COLOR_SLOTS {
                button.add_theme_color_override(slot, color);
            }
        }
    }
}

impl OrbSkin {
    fn button(&self) -> Option<Gd<Button>> {
        self.base().get_parent()?.try_cast::<Button>().ok()
    }

    /// Blanks the theme's flat fills so only the orb shows.
    fn apply_style_overrides(&self, button: &mut Gd<Button>) {
        for slot in STYLE_SLOTS {
            button.add_theme_stylebox_override(slot, &godot::classes::StyleBoxEmpty::new_gd());
        }
        if !self.recolor_icon_by_state {
            for slot in ICON_COLOR_SLOTS {
                button.add_theme_color_override(slot, ICON_LIGHT);
            }
        }
    }

    /// Fills the parent: anchors do not resolve reliably under a non-container Button.
    fn fit(&mut self) {
        let Some(button) = self.button() else {
            return;
        };
        let size = orb_rect(button.get_size());
        self.base_mut().set_size(size.size);
        self.base_mut().set_position(size.position);
    }
}

/// The orb's rect inside a button of `button_size`: overscanned and centered.
pub fn orb_rect(button_size: Vector2) -> Rect2 {
    let orb = button_size * ORB_OVERSCAN;
    Rect2::new((button_size - orb) * 0.5, orb)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_orb_overflows_the_button_evenly() {
        let rect = orb_rect(Vector2::new(80.0, 80.0));
        assert!((rect.size.x - 93.75).abs() < 1e-4);
        assert!((rect.position.x + 6.875).abs() < 1e-4);
        assert_eq!(rect.position.x, rect.position.y);
    }
}
