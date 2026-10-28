//! Moves the shell's host between the avatar-only scene and the explorer.

use godot::classes::Window;
use godot::prelude::*;

use super::{api, state};
use crate::engine::local_task::next_frame;
use crate::fetcher::globals;
use crate::godot_classes::dcl_global::DclGlobal;

const EXPLORER_SCENE: &str = "res://src/ui/explorer.tscn";
const AVATAR_HOST_SCENE: &str = "res://src/engine_host/avatar_host.tscn";

/// The shell flips the device orientation right around enter and exit; building or freeing a world
/// while the Android surface is being recreated crashed the Mali driver, so wait for the window
/// to settle.
const SURFACE_WAIT_MS: u64 = 700;
const SURFACE_SETTLE_MS: u64 = 300;

/// Goes to the explorer. Returns false when `is_cancelled` turned true while waiting, leaving the
/// host scene untouched. The explorer reads the destination from the deeplink object on boot.
pub async fn enter(parcel: Vector2i, realm: String, is_cancelled: impl Fn() -> bool) -> bool {
    settle_surface().await;
    if is_cancelled() {
        return false;
    }
    {
        let global = DclGlobal::singleton();
        let global = global.bind();
        let mut link = global.deep_link_obj.clone();
        let mut link = link.bind_mut();
        link.realm = realm.as_str().into();
        link.location = parcel;
    }
    if let Some(mut viewport) = DclGlobal::singleton().get_viewport() {
        viewport.set_transparent_background(false);
    }
    change_scene(EXPLORER_SCENE);
    true
}

/// The same world teardown as returning to the lobby once did, but the identity stays and the next
/// scene is the avatar host; the orientation is left to the shell.
pub async fn exit() {
    settle_surface().await;
    if let Some(world) = state::current() {
        world.loading.bind().cancel();
    }
    api::prepare_for_logout();
    let analytics = DclGlobal::singleton().bind().analytics_controller.clone();
    if let Some(mut analytics) = analytics {
        analytics.bind_mut().cancel_first_move_poll();
    }
    globals::scene_runner().bind_mut().kill_all_scenes();
    globals::realm().bind_mut().async_clear_realm();
    globals::scene_fetcher().bind_mut().reset_for_logout();
    globals::comms().bind_mut().leave(false);
    globals::scene_runner().bind_mut().detach_base_ui();
    change_scene(AVATAR_HOST_SCENE);
}

fn change_scene(path: &'static str) {
    if let Some(mut tree) = DclGlobal::singleton().get_tree_or_null() {
        crate::engine::local_task::defer(move || {
            tree.change_scene_to_file(path);
        });
    }
}

/// Waits until the window stopped changing size (or a fixed time passed with no change).
async fn settle_surface() {
    let Some(window) = DclGlobal::singleton().get_window() else {
        return;
    };
    let mut size = window.get_size();
    let start = godot::classes::Time::singleton().get_ticks_msec();
    let mut last_change = start;
    let mut changed = false;
    loop {
        next_frame().await;
        let now = godot::classes::Time::singleton().get_ticks_msec();
        let current = Window::get_size(&window);
        if current != size {
            size = current;
            last_change = now;
            changed = true;
        }
        if now - last_change >= SURFACE_SETTLE_MS && (changed || now - start >= SURFACE_WAIT_MS) {
            return;
        }
    }
}
