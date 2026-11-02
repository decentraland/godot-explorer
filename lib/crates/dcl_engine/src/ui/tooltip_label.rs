//! One interaction pill: the action's glyph or keycap and the scene's hover text. On a phone it is
//! tappable and presses the action.

use godot::classes::{
    IPanelContainer, Input, InputEvent, InputEventKey, InputEventMouseButton,
    InputEventScreenTouch, InputMap, Label, LabelSettings, MarginContainer, PanelContainer,
    StyleBox, StyleBoxFlat, Texture2D, TextureRect, TranslationServer,
};
use godot::global::HorizontalAlignment;
use godot::prelude::*;

use super::action_icons;
use super::host;
use super::touch_controls::{custom_icon_for_action, load_custom_icon};
use crate::engine::local_task::spawn_local;

/// The avatar "View profile" prompt, emitted as a KEY by the scene manager. The text label never
/// translates (it normally carries creator copy), so only this one marker is resolved here.
// i18n-keys: TOOLTIP_VIEW_PROFILE
pub const VIEW_PROFILE_KEY: &str = "TOOLTIP_VIEW_PROFILE";

/// The pill fill (#161518 at 70%); the pressed tint is tap feedback the design does not specify.
const BG_NORMAL: Color = Color::from_rgba(0.0863, 0.0824, 0.0941, 0.7);
const BG_PRESSED: Color = Color::from_rgba(0.2667, 0.2627, 0.2824, 0.7);
/// The shared HUD glyph tint; scene-replaced icons keep their own colors.
const ICON_COLOR: Color = Color::from_rgba(0.8745, 0.8157, 1.0, 1.0);
/// Invisible tap growth per side: 10 px by the design is under a millimetre on a phone once the
/// content scale applies, so it is widened to reach the ~48 dp minimum touch target.
const TAP_GROW_X: f32 = 24.0;
const TAP_GROW_Y: f32 = 24.0;
/// The widest the label may grow before it elides. Applied as a clamp: with an overrun behavior
/// set a Label's minimum width collapses to ~0, so a cap in `custom_minimum_size` would make every
/// pill that wide.
const LABEL_MAX_WIDTH: f32 = 313.0;
const ICON_LEFT_CLICK: &str = "uid://cljfaeb8np0ma";
const ANY: &str = "ia_any";

/// The pill's text width, capped.
pub fn label_width(text_width: f32) -> f32 {
    text_width.ceil().min(LABEL_MAX_WIDTH)
}

/// Whether `point` is inside the pill grown by the tap margins.
pub fn has_point_in(size: Vector2, point: Vector2, grow_y: f32) -> bool {
    Rect2::new(Vector2::ZERO, size)
        .grow_individual(TAP_GROW_X, grow_y, TAP_GROW_X, grow_y)
        .contains_point(point)
}

#[derive(GodotClass)]
#[class(base=PanelContainer, init)]
pub struct PointerTooltipPill {
    base: Base<PanelContainer>,
    /// Vertical tap growth: stacked pills clear each other by only a small gap, so the pointer
    /// tooltip zeroes it when more than one is on screen.
    #[init(val = TAP_GROW_Y)]
    pub tap_grow_y: f32,
    action_to_trigger: String,
    text_down: String,
    text_up: String,
    last_state_pressed: bool,
    stylebox: Option<Gd<StyleBoxFlat>>,
    /// The hash of the scene icon being loaded, so a stale fetch can bail when the pill is reused.
    custom_icon_hash: String,
    #[init(node = "%Label_Action")]
    label_action: OnReady<Gd<Label>>,
    #[init(node = "%TextureRect_ActionIcon")]
    action_icon: OnReady<Gd<TextureRect>>,
    #[init(node = "%Label_Text")]
    label_text: OnReady<Gd<Label>>,
    #[init(node = "%MarginContainer_Icons")]
    icons_margin: OnReady<Gd<MarginContainer>>,
}

#[godot_api]
impl IPanelContainer for PointerTooltipPill {
    fn ready(&mut self) {
        if let Some(panel) = self
            .base()
            .get_theme_stylebox("panel")
            .map(|s: Gd<StyleBox>| s.duplicate_resource())
            .and_then(|s| s.try_cast::<StyleBoxFlat>().ok())
        {
            self.base_mut().add_theme_stylebox_override("panel", &panel);
            self.stylebox = Some(panel);
        }
        self.set_bg(BG_NORMAL);
        if host::is_mobile() {
            let me = self.to_gd();
            self.base()
                .signals()
                .gui_input()
                .connect_other(&me, |this: &mut Self, event| this.on_gui_input(event));
        }
    }

    fn has_point(&self, point: Vector2) -> bool {
        has_point_in(self.base().get_size(), point, self.tap_grow_y)
    }

    fn physics_process(&mut self, _delta: f64) {
        if self.action_to_trigger == ANY {
            return;
        }
        // React to live icon changes (a scene reassigning a glyph) while the pill stays on screen.
        if !self.action_to_trigger.is_empty() {
            let desired = custom_icon_for_action(&self.action_to_trigger)
                .map(|c| c.hash)
                .unwrap_or_default();
            if desired != self.custom_icon_hash {
                let (down, up, action) = (
                    self.text_down.clone(),
                    self.text_up.clone(),
                    self.action_to_trigger.clone(),
                );
                self.set_tooltip_data(
                    down.as_str().into(),
                    up.as_str().into(),
                    action.as_str().into(),
                );
            }
        }
        let pressed = Input::singleton().is_action_pressed(&self.action_to_trigger);
        if self.last_state_pressed != pressed {
            self.set_bg(if pressed { BG_PRESSED } else { BG_NORMAL });
            self.icons_margin
                .add_theme_constant_override("margin_top", if pressed { 2 } else { 0 });
            let text = if pressed {
                &self.text_up
            } else {
                &self.text_down
            }
            .clone();
            self.set_label_text(&text);
            self.last_state_pressed = pressed;
        }
    }
}

#[godot_api]
impl PointerTooltipPill {
    #[func]
    pub fn set_tooltip_data(
        &mut self,
        text_pet_down: GString,
        text_pet_up: GString,
        action: GString,
    ) {
        let (down, up) = (text_pet_down.to_string(), text_pet_up.to_string());
        self.text_down = resolve(if down.is_empty() { &up } else { &down });
        self.text_up = resolve(if up.is_empty() { &down } else { &up });
        let action = action.to_string().to_lowercase();
        self.custom_icon_hash.clear();

        if host::is_mobile() && action_icons::look(&action).is_some() {
            self.show_mobile_action(&action);
        } else if action == ANY {
            self.show_keyboard("Any");
            self.action_to_trigger = action.clone();
            let text = self.text_down.clone();
            self.set_label_text(&text);
        } else if !self.show_desktop_action(&action) {
            return;
        }
        // A scene may replace this action's icon: swap the default glyph for it so the hint
        // matches the on-screen button. The default stays until the texture loads, if ever.
        if self.action_to_trigger == action {
            self.apply_custom_icon_override(&action);
        }
    }
}

impl PointerTooltipPill {
    fn set_bg(&mut self, color: Color) {
        if let Some(style) = self.stylebox.as_mut() {
            style.set_bg_color(color);
        }
    }

    /// The same table the joypad skins its buttons from, so the hint always matches the button.
    fn show_mobile_action(&mut self, action: &str) {
        let Some(look) = action_icons::look(action) else {
            return;
        };
        let label = TranslationServer::singleton()
            .translate(look.label_key)
            .to_string();
        match action_icons::tooltip_icon_for(&look) {
            Some(glyph) => self.show_keyboard_icon(&glyph, ICON_COLOR),
            None => self.show_keyboard(look.keycap),
        }
        self.action_to_trigger = action.to_string();
        if self.text_down.is_empty() {
            self.text_down = label.clone();
        }
        if self.text_up.is_empty() {
            self.text_up = label;
        }
        let text = self.text_down.clone();
        self.set_label_text(&text);
    }

    /// Shows what the InputMap binds to the action; false (and hides) when it is unknown.
    fn show_desktop_action(&mut self, action: &str) -> bool {
        let mut input_map = InputMap::singleton();
        if !input_map.has_action(action) {
            self.fail_action(action);
            return false;
        }
        enum Key {
            Letter(String),
            Icon(Gd<Texture2D>),
        }
        let pointer =
            || action_icons::texture(action_icons::look("ia_pointer").map_or("", |l| l.normal));
        let key = match input_map.action_get_events(action).iter_shared().next() {
            Some(event) => {
                if let Ok(key) = event.clone().try_cast::<InputEventKey>() {
                    char::from_u32(key.get_unicode())
                        .map(|c| Key::Letter(c.to_uppercase().to_string()))
                } else if let Ok(button) = event.try_cast::<InputEventMouseButton>() {
                    if button.get_button_index() == godot::global::MouseButton::LEFT {
                        let icon = if host::is_mobile() {
                            pointer()
                        } else {
                            action_icons::texture(ICON_LEFT_CLICK)
                        };
                        icon.map(Key::Icon)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            None => pointer().map(Key::Icon),
        };
        match key {
            Some(Key::Letter(letter)) => self.show_keyboard(&letter),
            Some(Key::Icon(icon)) => self.show_keyboard_icon(&icon, ICON_COLOR),
            None => {
                self.fail_action(action);
                return false;
            }
        }
        self.action_to_trigger = action.to_string();
        let text = self.text_down.clone();
        self.set_label_text(&text);
        true
    }

    fn fail_action(&mut self, action: &str) {
        self.base_mut().hide();
        self.action_to_trigger.clear();
        godot_error!("Action doesn't exist {action}");
    }

    fn apply_custom_icon_override(&mut self, action: &str) {
        let Some(custom) = custom_icon_for_action(action) else {
            return;
        };
        self.custom_icon_hash = custom.hash.clone();
        let me = self.to_gd();
        spawn_local(async move {
            let Some(texture) = load_custom_icon(&custom).await else {
                return;
            };
            if !me.is_instance_valid() {
                return;
            }
            let mut me = me;
            let mut pill = me.bind_mut();
            // Bail if the pill was reused for another action while the icon loaded.
            if pill.custom_icon_hash == custom.hash {
                pill.show_keyboard_icon(&texture, Color::WHITE);
            }
        });
    }

    fn show_keyboard(&mut self, text: &str) {
        self.base_mut().show();
        self.label_action.show();
        self.action_icon.hide();
        self.label_action.set_text(text);
    }

    /// Built-in glyphs take `ICON_COLOR`; a scene's own icon arrives with its colors and is shown
    /// untinted, as the on-screen button does.
    fn show_keyboard_icon(&mut self, icon: &Gd<Texture2D>, tint: Color) {
        self.base_mut().show();
        self.action_icon.show();
        self.label_action.hide();
        self.action_icon.set_self_modulate(tint);
        self.action_icon.set_texture(icon);
    }

    fn set_label_text(&mut self, value: &str) {
        self.label_text.set_text(value);
        let settings: Option<Gd<LabelSettings>> = self.label_text.get_label_settings();
        let font = settings
            .as_ref()
            .and_then(|s| s.get_font())
            .or_else(|| self.label_text.get_theme_font("font"));
        let size = settings
            .as_ref()
            .map(|s| s.get_font_size())
            .unwrap_or_else(|| self.label_text.get_theme_font_size("font_size"));
        if let Some(font) = font {
            let width = font
                .get_string_size_ex(value)
                .alignment(HorizontalAlignment::LEFT)
                .width(-1.0)
                .font_size(size)
                .done()
                .x;
            self.label_text
                .set_custom_minimum_size(Vector2::new(label_width(width), 0.0));
        }
    }

    fn on_gui_input(&mut self, event: Gd<InputEvent>) {
        let Ok(touch) = event.try_cast::<InputEventScreenTouch>() else {
            return;
        };
        // `ia_any` is virtual, with no InputMap entry: pressing it would log engine errors.
        if self.action_to_trigger.is_empty() || self.action_to_trigger == ANY {
            return;
        }
        let mut input = Input::singleton();
        if touch.is_pressed() {
            input.action_press(&self.action_to_trigger);
        } else {
            input.action_release(&self.action_to_trigger);
        }
    }
}

/// Resolves only our own prompt; anything else is creator copy and passes through untouched.
fn resolve(value: &str) -> String {
    if value == VIEW_PROFILE_KEY {
        TranslationServer::singleton().translate(value).to_string()
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_cap_at_the_max_width() {
        assert_eq!(label_width(100.2), 101.0);
        assert_eq!(label_width(900.0), LABEL_MAX_WIDTH);
    }

    #[test]
    fn the_tap_area_grows_past_the_pill() {
        let size = Vector2::new(130.0, 60.0);
        assert!(has_point_in(size, Vector2::new(-20.0, 30.0), TAP_GROW_Y));
        assert!(!has_point_in(size, Vector2::new(-30.0, 30.0), TAP_GROW_Y));
        // Stacked pills drop the vertical growth.
        assert!(!has_point_in(size, Vector2::new(10.0, 70.0), 0.0));
        assert!(has_point_in(size, Vector2::new(10.0, 70.0), TAP_GROW_Y));
    }

    #[test]
    fn creator_copy_passes_through() {
        assert_eq!(resolve("Take a seat"), "Take a seat");
    }
}
