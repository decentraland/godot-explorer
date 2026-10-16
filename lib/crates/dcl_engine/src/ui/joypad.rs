//! The action buttons on the right: one priority stack of `ia_*` actions fills fixed screen
//! slots from the top down. The first visible action is the big central button, the next ones sit
//! on an arc around it, and past that a "+" opens a column with the rest.

use std::collections::{HashMap, HashSet};

use godot::classes::{
    Button, Control, IControl, InputEvent, InputEventScreenTouch, Texture2D, VBoxContainer,
};
use godot::prelude::*;

use super::action_icons::{self, IC_JUMP_NORMAL, IC_JUMP_PRESSED};
use super::joypad_arc::JoypadArc;
use super::orb_skin::OrbSkin;
use super::touch_action_button::TouchActionButton;
use super::touch_controls::{custom_icon_for_action, load_custom_icon, CustomIcon};
use crate::engine::local_task::spawn_local;
use crate::fetcher::globals;
use crate::godot_classes::dcl_global::DclGlobal;
use crate::player::body::{Player, GLIDE_CLOSED, JUMP_ACTION_GLIDE_TOGGLE, JUMP_ACTION_JUMP};

const IC_DJUMP_NORMAL: &str = "uid://df6fla5fsgl2s";
const IC_DJUMP_PRESSED: &str = "uid://dmosa0apyje0c";
const IC_GLIDE_NORMAL: &str = "uid://baojgdd2swsg1";
const IC_GLIDE_PRESSED: &str = "uid://c52srwv13315";
const IC_GLIDE_HOLD: &str = "uid://c8jb31i47s7jy";
const IC_PLUS_NORMAL: &str = "uid://b14xrn24tfgpr";
const IC_PLUS_PRESSED: &str = "uid://npyym5xkix66";

#[derive(Clone, Copy, PartialEq)]
enum JumpIcon {
    Single,
    Double,
    Glider,
}

/// The combo column's glyph size: the design is 50x50 on the 72 px disc.
const COMBO_ICON_MAX_WIDTH: i32 = 50;
const BIG_ICON_MAX_WIDTH: i32 = 100;
const SMALL_ICON_MAX_WIDTH: i32 = 56;
const JUMP: &str = "ia_jump";

/// All on-screen buttons form one ordered list (issue #2518); hiding an action cascades the rest up.
pub const PRIORITY_ORDER: [&str; 8] = [
    JUMP,
    "ia_pointer",
    "ia_primary",
    "ia_secondary",
    "ia_action_3",
    "ia_action_4",
    "ia_action_5",
    "ia_action_6",
];

/// The priority stack minus hidden actions, with `main_action` (when visible and not jump)
/// promoted to the front. Hide wins: a hidden main action is not resurrected.
pub fn visible_actions(hidden: &HashSet<String>, main_action: &str) -> Vec<&'static str> {
    let mut visible: Vec<&'static str> = PRIORITY_ORDER
        .iter()
        .copied()
        .filter(|a| !hidden.contains(*a))
        .collect();
    if !main_action.is_empty() && main_action != JUMP {
        if let Some(index) = visible.iter().position(|a| *a == main_action) {
            if index > 0 {
                let promoted = visible.remove(index);
                visible.insert(0, promoted);
            }
        }
    }
    visible
}

/// `(show_plus, arc_capacity)` for `n` visible actions. The arc has four slots: without the "+"
/// all hold real buttons (five shown directly); a sixth button brings the "+", which takes the
/// fourth slot.
pub fn slot_plan(n: usize) -> (bool, usize) {
    let show_plus = n > 5;
    (show_plus, if show_plus { 3 } else { 4 })
}

fn icon(uid: &str) -> Option<Gd<Texture2D>> {
    action_icons::texture(uid)
}

#[derive(GodotClass)]
#[class(base=Control, init)]
pub struct Joypad {
    base: Base<Control>,
    combo_opened: bool,
    current_icon: Option<JumpIcon>,
    /// True while the glider provides lift, latching the button's hold look.
    gliding: bool,
    tc_active_applied: bool,
    last_tc_hash: u32,
    jump_icon_overridden: bool,
    jump_slot_is_big: bool,
    #[init(node = "%Button_Combo")]
    button_combo: OnReady<Gd<Button>>,
    #[init(node = "Button_Press")]
    button_press: OnReady<Gd<TouchActionButton>>,
    #[init(node = "Button_Press/Control")]
    arc: OnReady<Gd<JoypadArc>>,
    #[init(node = "%ComboColumn")]
    combo_column: OnReady<Gd<VBoxContainer>>,
    #[init(node = "%Button_Combo1")]
    combo1: OnReady<Gd<TouchActionButton>>,
    #[init(node = "%Button_Combo2")]
    combo2: OnReady<Gd<TouchActionButton>>,
    #[init(node = "%Button_Combo3")]
    combo3: OnReady<Gd<TouchActionButton>>,
    #[init(node = "%Button_Combo4")]
    combo4: OnReady<Gd<TouchActionButton>>,
    #[init(node = "Button_Press/Control/Button_Interact")]
    pointer_btn: OnReady<Gd<TouchActionButton>>,
    #[init(node = "Button_Press/Control/Button_Primary")]
    primary_btn: OnReady<Gd<TouchActionButton>>,
    #[init(node = "Button_Press/Control/Button_Secondary")]
    secondary_btn: OnReady<Gd<TouchActionButton>>,
    #[init(node = "Button_Press/Control/Button_Quaternary")]
    quaternary_btn: OnReady<Gd<TouchActionButton>>,
}

#[godot_api]
impl IControl for Joypad {
    fn ready(&mut self) {
        // The scene keeps a centered disc to align the buttons against while editing.
        if let Some(mut guide) = self.base().get_node_or_null("%TextureRect_ToAlign") {
            guide.set("visible", &false.to_variant());
        }
        let me = self.to_gd();
        for button in self.combo_buttons() {
            button
                .signals()
                .touch_action_changed()
                .connect_other(&me, |this: &mut Self, pressed| {
                    this.on_combo_action_changed(pressed)
                });
        }
        // The "+" toggles from raw touch so a second finger opens it too (the emulated mouse
        // only covers the primary one).
        self.button_combo
            .set_button_mask(godot::global::MouseButtonMask::default());
        self.button_combo
            .signals()
            .gui_input()
            .connect_other(&me, |this: &mut Self, event| this.on_combo_gui_input(event));
        self.button_combo
            .signals()
            .toggled()
            .connect_other(&me, |this: &mut Self, on| this.on_combo_toggled(on));
        // The "+" is a raw toggle, not a rendered action: skin it once. Open, it latches the hold
        // look like the glider.
        let mut plus = self.button_combo.clone();
        set_slot_icons(
            &mut plus,
            icon(IC_PLUS_NORMAL),
            icon(IC_PLUS_PRESSED),
            icon(IC_PLUS_PRESSED),
            false,
        );
        mark_attenuated(&self.to_gd().upcast::<Node>());
        self.apply_jump_icon(JumpIcon::Double);
        self.layout(&HashSet::new(), &HashMap::new(), "");
    }

    fn process(&mut self, _delta: f64) {
        self.update_jump_icon();
        self.apply_touch_controls();
    }

    fn on_notification(&mut self, what: godot::classes::notify::ControlNotification) {
        // Collapse the "+" menu whenever the joypad hides, so it never returns open.
        if what == godot::classes::notify::ControlNotification::VISIBILITY_CHANGED
            && !self.base().is_visible_in_tree()
        {
            self.close_combo();
        }
    }
}

impl Joypad {
    fn combo_buttons(&self) -> [Gd<TouchActionButton>; 4] {
        [
            self.combo1.clone(),
            self.combo2.clone(),
            self.combo3.clone(),
            self.combo4.clone(),
        ]
    }

    fn arc_slots(&self) -> [Gd<TouchActionButton>; 4] {
        [
            self.pointer_btn.clone(),
            self.primary_btn.clone(),
            self.secondary_btn.clone(),
            self.quaternary_btn.clone(),
        ]
    }

    fn on_combo_gui_input(&mut self, event: Gd<InputEvent>) {
        if let Ok(touch) = event.try_cast::<InputEventScreenTouch>() {
            if touch.is_pressed() {
                // Flipping the pressed state emits `toggled`, which opens or closes the menu.
                let pressed = self.button_combo.is_pressed();
                // Deferred: `toggled` would re-enter this joypad while it is bound.
                self.button_combo
                    .call_deferred("set_pressed", &[(!pressed).to_variant()]);
                self.button_combo.accept_event();
            }
        }
    }

    fn on_combo_toggled(&mut self, on: bool) {
        self.combo_opened = on;
        self.set_combo_hold(on);
        self.combo_column.set_visible(on);
        // Re-place on open: a layout run mid scene transition can leave the column's geometry stale.
        if on {
            self.position_combo_column();
        }
    }

    fn on_combo_action_changed(&mut self, pressed: bool) {
        if !pressed && self.combo_opened {
            // Clear the toggle before emitting `toggled(false)`: the handler refreshes the "+"
            // orb, which reads the pressed state.
            self.button_combo.set_pressed_no_signal(false);
            self.button_combo
                .call_deferred("emit_signal", &["toggled".to_variant(), false.to_variant()]);
        }
    }

    fn set_combo_hold(&mut self, on: bool) {
        if let Some(mut orb) = orb_of(&self.button_combo) {
            orb.bind_mut().set_hold(on);
        }
    }

    fn close_combo(&mut self) {
        if !self.combo_opened {
            return;
        }
        self.combo_opened = false;
        self.button_combo.set_pressed_no_signal(false);
        self.set_combo_hold(false);
        self.combo_column.set_visible(false);
    }

    /// Sizes the column to its visible buttons and pins it centered above the "+". The height is
    /// summed from the buttons, not read from the container's cached minimum, which lags a frame
    /// behind visibility changes (returning from a scene that hid the combo actions left the
    /// column mis-sized and floating far above the "+").
    fn position_combo_column(&mut self) {
        let visible: Vec<Gd<TouchActionButton>> = self
            .combo_buttons()
            .into_iter()
            .filter(|b| b.is_visible())
            .collect();
        if visible.is_empty() {
            return;
        }
        let separation = self.combo_column.get_theme_constant("separation") as f32;
        let mut width = 0.0_f32;
        let mut height = separation * (visible.len() - 1) as f32;
        for button in &visible {
            let min = button.get_combined_minimum_size();
            width = width.max(min.x);
            height += min.y;
        }
        self.combo_column
            .set_anchors_preset(godot::classes::control::LayoutPreset::TOP_LEFT);
        self.combo_column.set_size(Vector2::new(width, height));
        let x = (self.button_combo.get_size().x - width) * 0.5;
        self.combo_column
            .set_position(Vector2::new(x, -height - separation));
    }

    // Jump icon.

    fn update_jump_icon(&mut self) {
        if !self.jump_slot_is_big || self.jump_icon_overridden {
            if self.gliding {
                self.gliding = false;
                self.button_press.bind_mut().set_hold(false);
            }
            return;
        }
        let Some(player) = globals::scene_runner()
            .bind()
            .get_player_body_node()
            .and_then(|n| n.try_cast::<Player>().ok())
        else {
            return;
        };
        let (glide_disabled, double_jump_disabled) = {
            let global = DclGlobal::singleton();
            let global = global.bind();
            (global.is_glide_disabled(), global.is_double_jump_disabled())
        };
        let base = if double_jump_disabled {
            JumpIcon::Single
        } else {
            JumpIcon::Double
        };
        let current = self.current_icon.unwrap_or(JumpIcon::Double);
        let (jump_action, glide_state, grounded) = {
            let p = player.bind();
            let grounded = player.is_on_floor() || player.get_position().y <= 0.0;
            (p.get_jump_action(), p.get_glide_state(), grounded)
        };
        let want = match jump_action {
            JUMP_ACTION_JUMP => base,
            // Never flip to the glider where the scene disables it, even for the tick before the
            // force-close lands.
            JUMP_ACTION_GLIDE_TOGGLE if glide_disabled => base,
            JUMP_ACTION_GLIDE_TOGGLE => JumpIcon::Glider,
            // The glider stays through GLIDE_CLOSING to avoid a flicker before the cooldown,
            // unless the scene just disabled glide; otherwise the base icon is re-synced as the
            // scene-level flags change under us.
            _ if current == JumpIcon::Glider && glide_disabled => base,
            _ if current != JumpIcon::Glider => base,
            _ => current,
        };
        if Some(want) != self.current_icon {
            self.apply_jump_icon(want);
        }
        // The inverted look shows only while the glider provides lift in an allowed scene, so the
        // force-close does not flash the pressed state.
        let glide_active = glide_state != GLIDE_CLOSED && !grounded && !glide_disabled;
        if glide_active != self.gliding {
            self.gliding = glide_active;
            self.button_press.bind_mut().set_hold(glide_active);
        }
    }

    fn apply_jump_icon(&mut self, which: JumpIcon) {
        self.current_icon = Some(which);
        let mut press = self.button_press.clone().upcast::<Button>();
        match which {
            JumpIcon::Glider => set_slot_icons(
                &mut press,
                icon(IC_GLIDE_NORMAL),
                icon(IC_GLIDE_PRESSED),
                icon(IC_GLIDE_HOLD),
                true,
            ),
            JumpIcon::Single => set_slot_icons(
                &mut press,
                icon(IC_JUMP_NORMAL),
                icon(IC_JUMP_PRESSED),
                None,
                true,
            ),
            JumpIcon::Double => set_slot_icons(
                &mut press,
                icon(IC_DJUMP_NORMAL),
                icon(IC_DJUMP_PRESSED),
                None,
                true,
            ),
        }
    }

    // Touch controls.

    /// Applies PBTouchScreenControls: without the component the default stack; with it, the stack
    /// is recomputed from the `hide` denylist and the `main_action` override.
    fn apply_touch_controls(&mut self) {
        let (active, inputs, main_action) = {
            let global = DclGlobal::singleton();
            let global = global.bind();
            (
                global.touch_controls_active,
                global.touch_controls_inputs.clone(),
                global.touch_controls_main_action.to_string(),
            )
        };
        let state_hash = {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            (active, inputs.to_variant().hash_u32(), &main_action).hash(&mut hasher);
            hasher.finish() as u32
        };
        if state_hash == self.last_tc_hash {
            return;
        }
        self.last_tc_hash = state_hash;
        if !active {
            if self.tc_active_applied {
                self.tc_active_applied = false;
                self.layout(&HashSet::new(), &HashMap::new(), "");
            }
            return;
        }
        self.tc_active_applied = true;

        let mut hidden = HashSet::new();
        let mut icons = HashMap::new();
        for entry in inputs.iter_shared() {
            let Ok(entry) = entry.try_to::<VarDictionary>() else {
                continue;
            };
            let action = entry
                .get("action")
                .and_then(|v| v.try_to::<GString>().ok())
                .map(|s| s.to_string())
                .unwrap_or_default();
            if entry
                .get("hide")
                .and_then(|v| v.try_to::<bool>().ok())
                .unwrap_or(false)
            {
                hidden.insert(action.clone());
            }
            if let Some(custom) = custom_icon_for_action(&action) {
                icons.insert(action, custom);
            }
        }
        self.layout(&hidden, &icons, &main_action);
    }

    fn layout(
        &mut self,
        hidden: &HashSet<String>,
        icons: &HashMap<String, CustomIcon>,
        main_action: &str,
    ) {
        let visible = visible_actions(hidden, main_action);
        self.assign_slots(&visible, icons);
    }

    /// Index 0 is the big central button; the next up to three or four the arc; with a "+" the
    /// rest (index 4 on) go to the overflow column.
    fn assign_slots(&mut self, visible: &[&'static str], icons: &HashMap<String, CustomIcon>) {
        let n = visible.len();
        let (show_plus, arc_cap) = slot_plan(n);

        let mut big = self.button_press.clone();
        big.set_visible(n > 0);
        if n > 0 {
            self.render_action_on(&mut big, visible[0], icons.get(visible[0]), true);
        }

        let arc_actions: Vec<&str> = visible.iter().skip(1).take(arc_cap).copied().collect();
        for (i, mut slot) in self.arc_slots().into_iter().enumerate() {
            match arc_actions.get(i) {
                Some(action) => {
                    slot.set_visible(true);
                    self.render_action_on(&mut slot, action, icons.get(*action), false);
                }
                None => slot.set_visible(false),
            }
        }
        self.button_combo.set_visible(show_plus);

        // Satellites first and the "+" last, so it lands topmost.
        let mut arc_nodes = VarArray::new();
        for slot in self.arc_slots().iter().take(arc_actions.len()) {
            arc_nodes.push(&slot.to_variant());
        }
        if show_plus {
            arc_nodes.push(&self.button_combo.to_variant());
        }
        self.arc.bind_mut().arrange(arc_nodes);

        let overflow: Vec<&str> = if show_plus {
            visible.iter().skip(4).copied().collect()
        } else {
            Vec::new()
        };
        for (i, mut slot) in self.combo_buttons().into_iter().enumerate() {
            match overflow.get(i) {
                Some(action) => {
                    slot.set_visible(true);
                    self.render_action_on(&mut slot, action, icons.get(*action), false);
                    // The column's glyphs are smaller than the satellites'.
                    slot.add_theme_constant_override("icon_max_width", COMBO_ICON_MAX_WIDTH);
                }
                None => slot.set_visible(false),
            }
        }
        self.combo_column
            .set_visible(self.combo_opened && show_plus);
        self.position_combo_column();
        self.jump_slot_is_big = n > 0 && visible[0] == JUMP;
    }

    /// Puts `action` on `node`: sets its trigger, clears the previous occupant's look, then a
    /// scene icon, or the action's glyph pair through the button's orb.
    fn render_action_on(
        &mut self,
        node: &mut Gd<TouchActionButton>,
        action: &str,
        custom: Option<&CustomIcon>,
        is_big: bool,
    ) {
        node.bind_mut().trigger_action = action.into();
        reset_node_visuals(node);

        if let Some(custom) = custom {
            if action == JUMP && is_big {
                self.jump_icon_overridden = true;
            }
            async_set_button_icon(node.clone(), custom.clone());
            return;
        }
        // Jump on the big button keeps its dynamic glider and double-jump behavior.
        if action == JUMP && is_big {
            self.jump_icon_overridden = false;
            self.apply_jump_icon(self.current_icon.unwrap_or(JumpIcon::Double));
            return;
        }
        let mut button = node.clone().upcast::<Button>();
        if action == JUMP {
            // Demoted jump: the static glyph, no glider or hold.
            let single = DclGlobal::singleton().bind().is_double_jump_disabled();
            let (normal, pressed) = if single {
                (IC_JUMP_NORMAL, IC_JUMP_PRESSED)
            } else {
                (IC_DJUMP_NORMAL, IC_DJUMP_PRESSED)
            };
            set_slot_icons(&mut button, icon(normal), icon(pressed), None, is_big);
            return;
        }
        if let Some((normal, pressed)) = action_icons::button_icons(action) {
            set_slot_icons(&mut button, Some(normal), Some(pressed), None, is_big);
        }
    }
}

fn orb_of(button: &Gd<Button>) -> Option<Gd<OrbSkin>> {
    button
        .get_children()
        .iter_shared()
        .find_map(|c| c.try_cast::<OrbSkin>().ok())
}

/// Gives the button's orb its glyphs and sizes the icon (100 big, 56 small).
fn set_slot_icons(
    node: &mut Gd<Button>,
    normal: Option<Gd<Texture2D>>,
    pressed: Option<Gd<Texture2D>>,
    hold: Option<Gd<Texture2D>>,
    is_big: bool,
) {
    if let Some(mut orb) = orb_of(node) {
        orb.bind_mut().set_icons(normal, pressed, hold);
    }
    node.set_text("");
    node.set_expand_icon(true);
    node.add_theme_constant_override(
        "icon_max_width",
        if is_big {
            BIG_ICON_MAX_WIDTH
        } else {
            SMALL_ICON_MAX_WIDTH
        },
    );
}

fn mark_attenuated(node: &Gd<Node>) {
    let mut node = node.clone();
    if node.is_class("Button") {
        node.set_meta("attenuated_sound", &true.to_variant());
    }
    for child in node.get_children().iter_shared() {
        mark_attenuated(&child);
    }
}

/// Clears a previous occupant's presentation so a reused slot starts clean.
fn reset_node_visuals(node: &mut Gd<TouchActionButton>) {
    node.set_meta("tc_icon_hash", &"".to_variant());
    {
        let mut button = node.bind_mut();
        button.clear_custom_icon();
        button.set_hold(false);
    }
    if let Some(mut orb) = orb_of(&node.clone().upcast()) {
        orb.bind_mut().set_icons(None, None, None);
    }
    let mut button = node.clone().upcast::<Button>();
    button.set_button_icon(Option::<&Gd<Texture2D>>::None);
    button.set_text("");
    button.remove_theme_constant_override("icon_max_width");
    button.remove_theme_font_size_override("font_size");
}

/// Loads a scene's icon into a button's overlay. A stale apply (the config changed while it
/// loaded) is dropped.
fn async_set_button_icon(node: Gd<TouchActionButton>, custom: CustomIcon) {
    let mut button = node.clone();
    button.set_meta("tc_icon_hash", &custom.hash.as_str().to_variant());
    spawn_local(async move {
        let Some(texture) = load_custom_icon(&custom).await else {
            return;
        };
        if !node.is_instance_valid() {
            return;
        }
        let current: GString = node
            .get_meta_ex("tc_icon_hash")
            .default(&"".to_variant())
            .done()
            .to();
        if current.to_string() == custom.hash {
            apply_custom_icon(node, texture);
        }
    });
}

/// Shows the custom icon on the button's overlay and blanks the native glyph so it replaces it.
fn apply_custom_icon(mut node: Gd<TouchActionButton>, texture: Gd<Texture2D>) {
    node.bind_mut().set_custom_icon(Some(texture));
    let mut button = node.upcast::<Button>();
    button.set_button_icon(Option::<&Gd<Texture2D>>::None);
    button.set_text("");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hidden(actions: &[&str]) -> HashSet<String> {
        actions.iter().map(|a| a.to_string()).collect()
    }

    #[test]
    fn the_default_stack_is_the_priority_order() {
        assert_eq!(
            visible_actions(&HashSet::new(), ""),
            PRIORITY_ORDER.to_vec()
        );
    }

    #[test]
    fn hiding_cascades_the_rest_up() {
        let visible = visible_actions(&hidden(&["ia_jump", "ia_primary"]), "");
        assert_eq!(visible[0], "ia_pointer");
        assert!(!visible.contains(&"ia_primary"));
    }

    #[test]
    fn the_main_action_moves_to_the_front() {
        let visible = visible_actions(&HashSet::new(), "ia_secondary");
        assert_eq!(&visible[..3], &["ia_secondary", "ia_jump", "ia_pointer"]);
    }

    #[test]
    fn a_hidden_main_action_is_not_resurrected() {
        let visible = visible_actions(&hidden(&["ia_secondary"]), "ia_secondary");
        assert!(!visible.contains(&"ia_secondary"));
        assert_eq!(visible[0], "ia_jump");
    }

    #[test]
    fn jump_as_main_action_changes_nothing() {
        assert_eq!(
            visible_actions(&HashSet::new(), "ia_jump"),
            PRIORITY_ORDER.to_vec()
        );
    }

    #[test]
    fn the_plus_appears_only_past_five_buttons() {
        assert_eq!(slot_plan(5), (false, 4));
        assert_eq!(slot_plan(6), (true, 3));
        assert_eq!(slot_plan(8), (true, 3));
        assert_eq!(slot_plan(0), (false, 4));
    }
}
