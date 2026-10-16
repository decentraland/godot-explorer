//! `PBTouchScreenControls` on the HUD: a scene can hide the native joystick and crosshair to draw
//! its own touch UI, replace button glyphs, and the crosshair rides between screen center and an
//! anchor above the avatar's head as the camera zooms.

use godot::classes::input::MouseMode;
use godot::classes::{Control, Input, RefCounted, Texture2D};
use godot::prelude::*;

use super::host;
use super::joystick::VirtualJoystick;
use super::pointer_tooltip::PointerTooltip;
use crate::content::texture::TextureEntry;
use crate::engine::local_task::await_promise;
use crate::fetcher::globals;
use crate::godot_classes::dcl_global::DclGlobal;
use crate::godot_classes::promise::PromiseError;
use crate::player::body::Player;

/// A glyph a scene supplied for an action.
#[derive(Clone, Debug, PartialEq)]
pub struct CustomIcon {
    pub hash: String,
    pub url: String,
    /// The scene that declared it: its content mapping resolves the hash, sharing the cache entry
    /// with the scene's own UI (a hot reload purges both).
    pub scene_id: i32,
}

/// The icon a scene set for `action`, or none when the controls are inactive or it has none.
pub fn custom_icon_for_action(action: &str) -> Option<CustomIcon> {
    let global = DclGlobal::singleton();
    let global = global.bind();
    if !global.touch_controls_active {
        return None;
    }
    for entry in global.touch_controls_inputs.iter_shared() {
        let Ok(entry) = entry.try_to::<VarDictionary>() else {
            continue;
        };
        if string_of(&entry, "action") != action {
            continue;
        }
        let hash = string_of(&entry, "icon_hash");
        if hash.is_empty() {
            return None;
        }
        return Some(CustomIcon {
            hash,
            url: string_of(&entry, "icon_url"),
            scene_id: entry
                .get("scene_id")
                .and_then(|v| v.try_to::<i64>().ok())
                .unwrap_or(-1) as i32,
        });
    }
    None
}

/// The scene's icon texture: from the cache when it is there, else fetched. The fetch by hash
/// through the declaring scene's mapping shares the cache entry with the scene's own UI, so a
/// preview hot reload's purge drops it too (#2796); the by-URL path is only for a scene that is
/// already unloaded.
pub async fn load_custom_icon(custom: &CustomIcon) -> Option<Gd<Texture2D>> {
    let cached = globals::content_provider()
        .bind_mut()
        .get_texture_from_hash(custom.hash.as_str().into());
    if cached.is_some() {
        return cached;
    }
    let mapping = globals::scene_runner()
        .bind()
        .get_scene_content_mapping(custom.scene_id);
    let promise = if mapping.bind().get_base_url().is_empty() {
        globals::content_provider()
            .bind_mut()
            .fetch_texture_by_url(custom.hash.as_str().into(), custom.url.as_str().into())
    } else {
        globals::content_provider()
            .bind_mut()
            .fetch_texture_by_hash(custom.hash.as_str().into(), mapping)
    };
    let result = await_promise(promise).await;
    if result.try_to::<Gd<PromiseError>>().is_ok() {
        return None;
    }
    let entry = result.try_to::<Gd<TextureEntry>>().ok()?;
    let texture = entry.bind().texture.clone();
    Some(texture)
}

fn string_of(entry: &VarDictionary, key: &str) -> String {
    entry
        .get(key)
        .and_then(|v| v.try_to::<GString>().ok())
        .map(|s| s.to_string())
        .unwrap_or_default()
}

#[derive(GodotClass)]
#[class(base=RefCounted, init)]
pub struct SdkTouchControlsApplier {
    base: Base<RefCounted>,
    joystick: Option<Gd<VirtualJoystick>>,
    crosshair: Option<Gd<Control>>,
    player: Option<Gd<Player>>,
    pointer_tooltip: Option<Gd<PointerTooltip>>,
    hide_joystick_applied: bool,
    hide_crosshair_applied: bool,
}

#[godot_api]
impl SdkTouchControlsApplier {
    #[func]
    pub fn create(
        joystick: Option<Gd<VirtualJoystick>>,
        crosshair: Option<Gd<Control>>,
        player: Option<Gd<Player>>,
        pointer_tooltip: Option<Gd<PointerTooltip>>,
    ) -> Gd<Self> {
        Gd::from_init_fn(|base| Self {
            base,
            joystick,
            crosshair,
            player,
            pointer_tooltip,
            hide_joystick_applied: false,
            hide_crosshair_applied: false,
        })
    }

    /// The scene icon of `action` as `{hash, url, scene_id}`, empty when there is none.
    #[func]
    pub fn get_custom_icon_for_action(action: GString) -> VarDictionary {
        match custom_icon_for_action(&action.to_string()) {
            Some(icon) => VarDictionary::from_iter([
                ("hash".to_variant(), icon.hash.to_variant()),
                ("url".to_variant(), icon.url.to_variant()),
                ("scene_id".to_variant(), (icon.scene_id as i64).to_variant()),
            ]),
            None => VarDictionary::new(),
        }
    }

    /// `hidden_for_hide_ui` is the explorer's hide-UI state, which wins when restoring.
    #[func]
    pub fn apply(&mut self, hidden_for_hide_ui: bool) {
        self.apply_joystick(hidden_for_hide_ui);
        self.apply_crosshair();
        self.apply_crosshair_anchor();
    }
}

impl SdkTouchControlsApplier {
    /// The hidden state is enforced every frame (other HUD logic may re-show these) but
    /// visibility is restored once, on the transition back. Only the joystick's graphic and touch
    /// area hide, so the camera button stays usable.
    fn apply_joystick(&mut self, hidden_for_hide_ui: bool) {
        let hide = DclGlobal::singleton().bind().touch_controls_hide_joystick;
        if let Some(joystick) = self.joystick.as_mut() {
            if hide {
                joystick.bind_mut().set_visuals_hidden(true);
            } else if self.hide_joystick_applied && !hidden_for_hide_ui {
                joystick.bind_mut().set_visuals_hidden(false);
            }
        }
        self.hide_joystick_applied = hide;
    }

    fn apply_crosshair(&mut self) {
        let Some(crosshair) = self.crosshair.as_mut() else {
            return;
        };
        let hide = DclGlobal::singleton().bind().touch_controls_hide_crosshair;
        if hide {
            crosshair.hide();
        } else if self.hide_crosshair_applied
            && (host::is_mobile() || Input::singleton().get_mouse_mode() == MouseMode::CAPTURED)
        {
            crosshair.show();
        }
        self.hide_crosshair_applied = hide;
    }

    /// Mobile only: cinematic mode owns the crosshair (tap cursor) and desktop keeps the centered
    /// one.
    fn apply_crosshair_anchor(&mut self) {
        if !host::is_mobile() {
            return;
        }
        let mut runner = globals::scene_runner();
        if runner.bind().get_raycast_use_cursor_position() {
            return;
        }
        let Some(crosshair) = self.crosshair.as_mut().filter(|c| c.is_visible()) else {
            // No crosshair on screen: aim from the viewport center.
            runner.bind_mut().clear_crosshair_screen_point();
            return;
        };
        let Some(player) = self.player.as_ref().filter(|p| p.is_instance_valid()) else {
            return;
        };
        let (anchor, alpha) = {
            let player = player.bind();
            if player.get_mount_camera().is_none() {
                return;
            }
            (
                player.get_crosshair_screen_position(),
                player.get_crosshair_alpha(),
            )
        };
        let half = crosshair.get_size() / 2.0;
        crosshair.set_global_position(anchor - half);
        // The fade follows the mode swap: first person to third person fades in over the head.
        let mut modulate = crosshair.get_modulate();
        modulate.a = alpha;
        crosshair.set_modulate(modulate);
        // The tooltip follows the crosshair, or it would sit at the screen center.
        if let Some(tooltip) = self.pointer_tooltip.as_mut() {
            tooltip.bind_mut().set_global_cursor_position(anchor);
        }
        runner.bind_mut().set_crosshair_screen_point(anchor);
    }
}
