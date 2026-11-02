//! The explorer's on-screen controls: which of them show, where the crosshair and the pointer
//! tooltip sit, the safe areas scenes draw into, and how the world reacts to the signals it
//! listens to. Everything reads the world's handles from `state`.

use godot::classes::input::MouseMode;
use godot::classes::Input;
use godot::classes::{DisplayServer, InputEvent, InputEventKey, InputEventMouseButton, Node3D};
use godot::global::Key;
use godot::prelude::*;

use super::explorer::Explorer;
use super::state::{self, World};
use crate::auth::dcl_player_identity::DclPlayerIdentity;
use crate::engine::local_task::defer;
use crate::fetcher::globals;
use crate::godot_classes::dcl_global::DclGlobal;

pub fn connect_signals(explorer: &Gd<Explorer>) {
    DclGlobal::singleton()
        .signals()
        .orientation_changed()
        .connect_other(explorer, |_: &mut Explorer, portrait| {
            if let Some(world) = state::current() {
                on_orientation_changed(&world, portrait);
            }
        });
}

/// The signals that need the world's nodes, connected once it exists.
pub fn connect_world_signals(world: &World) {
    let explorer = &world.explorer;
    globals::scene_runner()
        .signals()
        .pointer_tooltip_changed()
        .connect_other(explorer, |this: &mut Explorer| {
            let _ = this;
            defer(change_tooltips);
        });
    let push = |_: &mut Explorer| push_scene_interactable_area();
    world
        .scene_ui
        .signals()
        .resized()
        .connect_other(explorer, push);
    if let Some(window) = explorer.get_window() {
        window
            .signals()
            .size_changed()
            .connect_other(explorer, push);
    }
    explorer
        .get_node_as::<godot::classes::Timer>("UI/Timer_SavePosition")
        .signals()
        .timeout()
        .connect_other(explorer, |_: &mut Explorer| save_position_if_dirty());
    world
        .emote_wheel
        .signals()
        .emote_wheel_closed()
        .connect_other(explorer, |_: &mut Explorer| on_emote_wheel_closed());
    world
        .emote_wheel
        .signals()
        .emote_wheel_opened()
        .connect_other(explorer, |_: &mut Explorer| {
            if let Some(world) = state::current() {
                world.joystick.clone().hide();
            }
        });
    defer(push_scene_interactable_area);
}

pub fn connect_identity(explorer: &Gd<Explorer>, identity: &Gd<DclPlayerIdentity>) {
    identity
        .signals()
        .logout()
        .connect_other(explorer, |_: &mut Explorer| {
            // Every logout (a session expiring too) goes through the one canonical teardown;
            // `sign_out` guards its own re-entrancy, so it may be what emitted this.
            if state::current().is_some() {
                DclGlobal::sign_out();
            }
        });
    identity
        .signals()
        .profile_changed()
        .connect_other(explorer, |_: &mut Explorer, profile| {
            globals::avatars_scene()
                .bind_mut()
                .update_primary_player_profile(profile);
        });
    globals::dcl_tokio_rpc()
        .signals()
        .need_open_url()
        .connect_other(explorer, |_: &mut Explorer, url, _description, _webview| {
            if !globals::player_identity()
                .bind()
                .get_address_str()
                .is_empty()
            {
                DclGlobal::open_url(url, false);
            }
        });
}

/// First-frame state of the controls: the joystick passes touches through, the HUD shows.
pub fn prepare_controls(world: &World) {
    let mut joystick = world.joystick.clone();
    joystick.set_mouse_filter(godot::classes::control::MouseFilter::IGNORE);
    world.mobile_ui.clone().show();
    if DclGlobal::singleton().bind().is_mobile {
        world.crosshair.clone().show();
        reset_cursor_position();
    }
    update_virtual_controls_visibility(world);
    world.tooltip.clone().hide();
}

pub fn unhandled_input(event: &Gd<InputEvent>) {
    let Some(world) = state::current() else {
        return;
    };
    if let Ok(button) = event.clone().try_cast::<InputEventMouseButton>() {
        let desktop = !DclGlobal::singleton().bind().is_mobile;
        if desktop
            && button.is_pressed()
            && world.ui_root.has_focus()
            && Input::singleton().get_mouse_mode() != MouseMode::CAPTURED
        {
            capture_mouse();
        }
    }
    if let Ok(key) = event.clone().try_cast::<InputEventKey>() {
        if world.ui_root.has_focus()
            && key.is_pressed()
            && key.get_keycode() == Key::ESCAPE
            && Input::singleton().get_mouse_mode() == MouseMode::CAPTURED
        {
            release_mouse();
        }
    }
}

pub fn capture_mouse() {
    let Some(world) = state::current() else {
        return;
    };
    if DisplayServer::singleton().has_feature(godot::classes::display_server::Feature::MOUSE) {
        Input::singleton().set_mouse_mode(MouseMode::CAPTURED);
    }
    // Scene-driven crosshair hiding is respected.
    if !DclGlobal::singleton().bind().touch_controls_hide_crosshair {
        world.crosshair.clone().show();
    }
    DclGlobal::explorer_grab_focus();
}

pub fn release_mouse() {
    let Some(world) = state::current() else {
        return;
    };
    if DisplayServer::singleton().has_feature(godot::classes::display_server::Feature::MOUSE) {
        Input::singleton().set_mouse_mode(MouseMode::VISIBLE);
    }
    if !DclGlobal::singleton().bind().is_mobile {
        world.crosshair.clone().hide();
    }
}

/// The scene hides the interface: the joystick keeps walking but is fully transparent (a hidden
/// control stops receiving input, so it is shown and only loses its alpha).
pub fn apply_mobile_controls_hide_ui(hidden: bool) {
    let Some(world) = state::current() else {
        return;
    };
    world.controls_hidden_for_hide_ui.set(hidden);
    let mut joystick = world.joystick.clone();
    if hidden {
        world.joypad.clone().hide();
        joystick.show();
        joystick.set_modulate(Color::from_rgba(1.0, 1.0, 1.0, 0.0));
    } else {
        world.joypad.clone().show();
        joystick.set_modulate(Color::WHITE);
    }
}

fn update_virtual_controls_visibility(world: &World) {
    let mut joystick = world.joystick.clone();
    if world.controls_hidden_for_hide_ui.get() {
        world.joypad.clone().hide();
        joystick.show();
        joystick.set_modulate(Color::from_rgba(1.0, 1.0, 1.0, 0.0));
        return;
    }
    world.joypad.clone().show();
    joystick.show();
}

/// The shell's chat takes the screen: out of its way go the emote wheel and the movement controls.
pub fn set_chat_focus(open: bool) {
    let Some(world) = state::current() else {
        return;
    };
    world.chat_open.set(open);
    if DclGlobal::is_orientation_portrait() || world.controls_hidden_for_hide_ui.get() {
        return;
    }
    let mut wheel = world.emote_wheel.clone();
    if open {
        wheel.bind_mut().close();
        wheel.hide();
        world.joypad.clone().hide();
        world.joystick.clone().hide();
    } else {
        wheel.show();
        world.joypad.clone().show();
        world.joystick.clone().show();
    }
}

fn on_orientation_changed(world: &World, portrait: bool) {
    let mut wheel = world.emote_wheel.clone();
    if portrait {
        world.mobile_ui.clone().hide();
        world.joystick.clone().hide();
        wheel.hide();
    } else {
        world.mobile_ui.clone().show();
        update_virtual_controls_visibility(world);
        wheel.show();
    }
    let mut base_ui = globals::scene_runner().bind().base_ui.clone();
    if base_ui.is_instance_valid() {
        base_ui.set_visible(!portrait);
    }
}

fn on_emote_wheel_closed() {
    let Some(world) = state::current() else {
        return;
    };
    if !DclGlobal::is_orientation_portrait() && !world.chat_open.get() {
        world.joystick.clone().show();
    }
}

fn save_position_if_dirty() {
    let Some(world) = state::current() else {
        return;
    };
    if world.dirty_save_position.replace(false) {
        globals::engine().bind().save_settings();
    }
}

pub fn set_cursor_position(position: Vector2) {
    let Some(world) = state::current() else {
        return;
    };
    let mut runner = globals::scene_runner();
    if !runner.bind().get_raycast_use_cursor_position() {
        return;
    }
    place_crosshair(&world, position - Vector2::new(0.0, 1.0));
    world
        .tooltip
        .clone()
        .bind_mut()
        .set_global_cursor_position(position);
    runner.bind_mut().set_cursor_position(position);
}

/// The crosshair back at the center of the screen.
pub fn reset_cursor_position() {
    let Some(world) = state::current() else {
        return;
    };
    let center = world
        .explorer
        .get_viewport()
        .map(|v| v.get_visible_rect().size * 0.5)
        .unwrap_or_default();
    place_crosshair(&world, center - Vector2::new(0.0, 1.0));
    world
        .tooltip
        .clone()
        .bind_mut()
        .set_global_cursor_position(center);
}

fn place_crosshair(world: &World, point: Vector2) {
    let half = world.crosshair.get_size() / 2.0;
    world.crosshair.clone().set_global_position(point - half);
}

/// The pointer prompts and outlines follow what the crosshair is over.
pub fn change_tooltips() {
    let Some(world) = state::current() else {
        return;
    };
    let tooltip_data = globals::scene_runner()
        .bind()
        .pointer_tooltips
        .duplicate_shallow();

    let under_crosshair = world.player.bind().get_avatar_under_crosshair();
    if under_crosshair != *world.last_outlined_avatar.borrow() {
        if let Some(mut outline) = world.player.bind().outline_system.clone() {
            outline
                .bind_mut()
                .set_outlined_avatar(under_crosshair.clone());
        }
        *world.last_outlined_avatar.borrow_mut() = under_crosshair;
    }

    // The highlight of scene objects with show_highlight.
    let highlighted: Option<Gd<Node3D>> = globals::scene_runner()
        .bind()
        .highlighted_entity
        .clone()
        .filter(|e| e.is_instance_valid());
    if highlighted != *world.last_outlined_entity.borrow() {
        if let Some(mut outline) = world.player.bind().outline_system.clone() {
            outline.bind_mut().set_outlined_entity(highlighted.clone());
        }
        *world.last_outlined_entity.borrow_mut() = highlighted;
    }

    let mut tooltip = world.tooltip.clone();
    if tooltip_data.is_empty() {
        tooltip.hide();
    } else {
        tooltip.bind_mut().set_pointer_data(tooltip_data);
        tooltip.show();
    }
}

/// The HUD safe rectangle (in SceneUIContainer coordinates): the device safe area plus the HUD
/// margins, without the chat column.
pub fn safe_area(world: &World) -> Rect2 {
    let canvas = world.scene_ui.get_size();
    let margin = |name: &str| world.hud.get_theme_constant(name) as f32;
    let (left, right, top, bottom) = (
        margin("margin_left"),
        margin("margin_right"),
        margin("margin_top"),
        margin("margin_bottom"),
    );
    Rect2::new(
        Vector2::new(left, top),
        Vector2::new(
            (canvas.x - left - right).max(0.0),
            (canvas.y - top - bottom).max(0.0),
        ),
    )
}

/// Where scenes may draw interactive UI without being covered by the client UI: the safe
/// rectangle minus the chat column on the left.
fn interactable_area(world: &World) -> Rect2 {
    let safe = safe_area(world);
    let chat = DclGlobal::singleton().bind().chat_notifications_width as f32;
    Rect2::new(
        Vector2::new(safe.position.x + chat, safe.position.y),
        Vector2::new((safe.size.x - chat).max(0.0), safe.size.y),
    )
}

/// Pushes the areas (in canvas pixels) to the scene runner, so scenes get a correct
/// `UiCanvasInformation.interactable_area` on every resize.
pub fn push_scene_interactable_area() {
    let Some(world) = state::current() else {
        return;
    };
    let area = interactable_area(&world);
    if area.size.x > 0.0 && area.size.y > 0.0 {
        let mut runner = globals::scene_runner();
        let mut runner = runner.bind_mut();
        runner.set_interactable_area(area.to_rect2i());
        runner.set_safe_area(safe_area(&world).to_rect2i());
    }
}
