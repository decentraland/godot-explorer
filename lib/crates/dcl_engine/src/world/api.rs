//! What the rest of the engine asks of the world: moving and aiming the player, chat, logout.

use godot::prelude::*;

use super::{controls, state};
use crate::fetcher::globals;

/// Moves the player. `skip_loading` is for teleports inside a scene, which must not load again an
/// area that already is.
pub fn move_to(position: Vector3, skip_loading: bool, check_stuck: bool) {
    let Some(world) = state::current() else {
        return;
    };
    if world.disable_move_to.get() {
        return;
    }
    // Keeps an emote from being cancelled by the teleport.
    if let Some(mut avatar) = world.player.bind().avatar.clone() {
        avatar.bind_mut().set_teleport_grace();
    }
    world
        .player
        .clone()
        .bind_mut()
        .move_to(position, check_stuck);
    // Remote peers snap to the new position instead of interpolating across the jump.
    globals::comms()
        .bind_mut()
        .notify_player_teleported(position);
    if skip_loading {
        return;
    }
    let now = world.player.get_position();
    let parcel_x = (now.x * 0.0625).floor() as i32;
    let parcel_y = -((now.z * 0.0625).floor() as i32);
    let loaded = globals::scene_fetcher()
        .bind()
        .is_scene_loaded(parcel_x, parcel_y);
    if !loaded && !world.loading.bind().is_running() {
        world
            .loading
            .bind()
            .begin("".into(), "on_moveto".into(), true);
    }
}

/// Places the player on a parcel of the realm that is already loaded and records it as where
/// they are. Changing realm on the way is a navigation, which resolves that first.
pub fn teleport_to(parcel: Vector2i) {
    let Some(world) = state::current() else {
        return;
    };
    move_to(
        Vector3::new((parcel.x * 16 + 8) as f32, 3.0, (-parcel.y * 16 - 8) as f32),
        false,
        true,
    );
    globals::scene_fetcher()
        .bind_mut()
        .update_position(parcel, true);
    world.dirty_save_position.set(true);
}

pub fn player_look_at(target: Vector3) {
    if let Some(world) = state::current() {
        world.player.clone().bind_mut().avatar_look_at(target);
    }
}

pub fn camera_look_at(target: Vector3) {
    if let Some(world) = state::current() {
        world.player.clone().bind_mut().camera_look_at(target);
    }
}

pub fn avatar_look_at_independent(target: Vector3) {
    if let Some(world) = state::current() {
        world
            .player
            .clone()
            .bind_mut()
            .set_avatar_rotation_independent(target);
    }
}

/// Where the shell's native controls sit, in device pixels: the HUD and the scene UI keep clear
/// of these margins.
pub fn set_hud_insets(top: f32, right: f32, bottom: f32, left: f32) {
    let Some(world) = state::current() else {
        return;
    };
    let mut hud = world.hud.clone();
    for (side, pixels) in [
        ("top", top),
        ("right", right),
        ("bottom", bottom),
        ("left", left),
    ] {
        let units = device_pixels_to_viewport_units(pixels);
        hud.add_theme_constant_override(&format!("margin_{side}"), units as i32);
    }
    controls::push_scene_interactable_area();
}

/// The shell reports device pixels; Godot lays out in its scaled viewport units.
pub fn device_pixels_to_viewport_units(pixels: f32) -> f32 {
    let window_width = godot::classes::DisplayServer::singleton()
        .window_get_size()
        .x;
    let viewport_width = state::current()
        .and_then(|w| w.explorer.get_viewport())
        .map(|v| v.get_visible_rect().size.x);
    match viewport_width {
        Some(width) if window_width > 0 => pixels * width / window_width as f32,
        _ => pixels,
    }
}

/// Whether the explorer's UI holds focus; true with no world to ask.
pub fn ui_has_focus() -> bool {
    state::current().is_none_or(|w| w.ui_root.has_focus())
}

/// Severs the world from the services that outlive it, before they are torn down, so none of
/// them reaches an explorer about to be freed. Called by sign-out and by leaving the world.
pub fn prepare_for_logout() {
    state::detach();
}
