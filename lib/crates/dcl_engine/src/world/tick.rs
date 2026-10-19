//! What the world does each frame: applies the scene's touch-control rules and follows the
//! player from parcel to parcel.

use godot::prelude::*;

use super::state;
use crate::fetcher::globals;
use crate::godot_classes::dcl_global::DclGlobal;

/// Meters per parcel, inverted: positions to parcel coordinates.
const PARCEL_SCALE: f32 = 0.0625;

pub fn tick() {
    let Some(world) = state::current() else {
        return;
    };
    world
        .sdk_touch
        .clone()
        .bind_mut()
        .apply(world.controls_hidden_for_hide_ui.get());

    let position = world.player.get_position();
    let real = Vector2::new(position.x * PARCEL_SCALE, -position.z * PARCEL_SCALE);
    let parcel = Vector2i::new(real.x.floor() as i32, real.y.floor() as i32);
    world.parcel_real.set(real);
    world.parcel.set(parcel);
    if world.last_parcel.get() == parcel {
        return;
    }
    world.last_parcel.set(parcel);
    globals::scene_fetcher()
        .bind_mut()
        .update_position(parcel, false);
    let engine = globals::engine();
    engine
        .bind()
        .set_setting("last_parcel_x".into(), parcel.x.to_variant(), false);
    engine
        .bind()
        .set_setting("last_parcel_y".into(), parcel.y.to_variant(), false);
    world.dirty_save_position.set(true);
    DclGlobal::singleton()
        .signals()
        .change_parcel()
        .emit(parcel);
    globals::metrics()
        .bind_mut()
        .update_position(format!("{},{}", parcel.x, parcel.y));
}
