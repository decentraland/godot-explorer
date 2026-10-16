//! What the rest of the app asks of `Global`: its signals, the camera mode, the orientation, the
//! device safe area, the focus and mouse of the explorer, the platform URL openers and the sign-out.
//! Everything here is static (or takes the handle) so none of it runs under `Global`'s own borrow.

use godot::classes::{DisplayServer, Input, Node, Window};
use godot::prelude::*;

use super::safe_area_presets::{self, Emulated};
use super::session;
use crate::fetcher::globals;
use crate::godot_classes::dcl_global::DclGlobal;

/// The orientation the window's shape says: portrait when it is at least as tall as wide.
pub fn window_is_portrait(size: Vector2i) -> bool {
    size.y >= size.x
}

/// The window size of an orientation when no phone is emulated.
pub fn default_window_size(portrait: bool) -> Vector2i {
    if portrait {
        Vector2i::new(720, 1280)
    } else {
        Vector2i::new(1280, 720)
    }
}

fn global() -> Gd<DclGlobal> {
    DclGlobal::singleton()
}

fn root_window() -> Option<Gd<Window>> {
    global().get_tree_or_null()?.get_root()
}

/// Which phone the window emulates; emulation flags are desktop-preview only, ignored on a real
/// device (the editor forwards its run args to one-click device deploys, which would otherwise force
/// the emulated window size on the phone).
pub fn emulated_phone() -> Option<Emulated> {
    let g = global();
    let g = g.bind();
    if g.is_android || g.is_ios {
        return None;
    }
    let cli = g.cli.bind();
    if cli.emulate_ios {
        Some(Emulated::Ios)
    } else if cli.emulate_android {
        Some(Emulated::Android)
    } else {
        None
    }
}

/// Sizes the window for an emulated phone and centers it; a plain desktop gets a default size.
fn resize_window(portrait: bool) {
    let Some(mut window) = root_window() else {
        return;
    };
    let size = match emulated_phone() {
        Some(phone) => safe_area_presets::window_size(phone, portrait),
        None => default_window_size(portrait),
    };
    window.set_size(size);
    window.move_to_center();
}

fn set_orientation(portrait: bool) {
    // Set the orientation before changing the window so listeners see the right value.
    global().bind_mut().is_portrait = portrait;
    let (virtual_mobile, mobile) = {
        let g = global();
        let g = g.bind();
        (g.is_virtual_mobile, g.is_mobile)
    };
    if mobile && !virtual_mobile {
        DisplayServer::singleton().screen_set_orientation(if portrait {
            godot::classes::display_server::ScreenOrientation::PORTRAIT
        } else {
            godot::classes::display_server::ScreenOrientation::SENSOR_LANDSCAPE
        });
    } else {
        resize_window(portrait);
    }
    global().emit_signal("orientation_changed", &[portrait.to_variant()]);
}

/// The RN host owns the device orientation: mirror the window's shape into the layout state, and
/// keep mirroring it as the window resizes.
pub fn follow_host_orientation() {
    let Some(window) = root_window() else {
        return;
    };
    let mut me = global();
    if !window.is_connected(
        "size_changed",
        &Callable::from_object_method(&me, "follow_host_orientation"),
    ) {
        window.clone().connect(
            "size_changed",
            &Callable::from_object_method(&me, "follow_host_orientation"),
        );
    }
    let portrait = window_is_portrait(window.get_size());
    let changed = {
        let mut g = me.bind_mut();
        let changed = g.is_portrait != portrait;
        g.is_portrait = portrait;
        changed
    };
    if changed {
        me.clone()
            .emit_signal("orientation_changed", &[portrait.to_variant()]);
    }
}

fn is_host_driven() -> bool {
    globals::engine().bind().is_host_driven()
}

#[godot_api(secondary)]
impl DclGlobal {
    // Camera mode.

    #[func]
    pub fn set_camera_mode(camera_mode: i32) {
        global().bind_mut().current_camera_mode = camera_mode;
        global().emit_signal("camera_mode_set", &[camera_mode.to_variant()]);
    }

    // Orientation and the device safe area.

    #[func]
    pub fn is_orientation_portrait() -> bool {
        global().bind().is_portrait
    }

    /// While tearing the session down toward a portrait-only screen, stray landscape flips are
    /// ignored: the loading machinery reacts to the realm being cleared during teardown and would
    /// otherwise flip back to landscape right before the swap and strand the next screen there
    /// (issue #2508).
    #[func]
    pub fn set_orientation_landscape() {
        if global().bind().signing_out {
            return;
        }
        if is_host_driven() {
            follow_host_orientation();
            return;
        }
        set_orientation(false);
    }

    #[func]
    pub fn set_orientation_portrait() {
        if is_host_driven() {
            follow_host_orientation();
            return;
        }
        set_orientation(true);
    }

    /// The window shape changed under the host: re-mirror it. Connected by name to the window.
    #[func(rename = follow_host_orientation)]
    fn gd_follow_host_orientation() {
        follow_host_orientation();
    }

    #[func]
    pub fn is_emulating_safe_area() -> bool {
        emulated_phone().is_some()
    }

    /// The safe area in window pixels: the emulated phone's, or the device's.
    #[func]
    pub fn get_safe_area() -> Rect2i {
        match (emulated_phone(), root_window()) {
            (Some(phone), Some(window)) => {
                safe_area_presets::safe_area(phone, global().bind().is_portrait, window.get_size())
            }
            _ => DisplayServer::singleton().get_display_safe_area(),
        }
    }

    // The explorer: focus, mouse and the platform.

    #[func]
    pub fn get_explorer() -> Option<Gd<Node>> {
        global()
            .get_tree_or_null()?
            .get_root()?
            .get_node_or_null("explorer")
            .filter(|n| n.is_instance_valid())
    }

    #[func]
    pub fn explorer_has_focus() -> bool {
        let Some(explorer) = Self::get_explorer() else {
            return false;
        };
        explorer
            .get("ui_root")
            .try_to::<Gd<godot::classes::Control>>()
            .is_ok_and(|ui| ui.has_focus())
    }

    #[func]
    pub fn explorer_grab_focus() {
        let Some(explorer) = Self::get_explorer() else {
            return;
        };
        // A scene text input being typed in keeps focus (and its keyboard) until it blurs.
        if globals::engine().bind().is_text_input_focused() {
            return;
        }
        if let Ok(mut ui) = explorer
            .get("ui_root")
            .try_to::<Gd<godot::classes::Control>>()
        {
            ui.call_deferred("grab_focus", &[]);
        }
    }

    #[func]
    pub fn explorer_release_focus() {
        if let Some(explorer) = Self::get_explorer() {
            if let Ok(mut ui) = explorer
                .get("ui_root")
                .try_to::<Gd<godot::classes::Control>>()
            {
                ui.call_deferred("release_focus", &[]);
            }
        }
    }

    #[func]
    pub fn capture_mouse() {
        match Self::get_explorer() {
            Some(mut explorer) => {
                explorer.call("capture_mouse", &[]);
            }
            None if DisplayServer::singleton()
                .has_feature(godot::classes::display_server::Feature::MOUSE) =>
            {
                Input::singleton().set_mouse_mode(godot::classes::input::MouseMode::CAPTURED);
            }
            None => {}
        }
    }

    #[func]
    pub fn release_mouse() {
        match Self::get_explorer() {
            Some(mut explorer) => {
                explorer.call("release_mouse", &[]);
            }
            None if DisplayServer::singleton()
                .has_feature(godot::classes::display_server::Feature::MOUSE) =>
            {
                Input::singleton().set_mouse_mode(godot::classes::input::MouseMode::VISIBLE);
            }
            None => {}
        }
    }

    /// Vibrates the handheld; a no-op on desktops.
    #[func]
    pub fn send_haptic_feedback(
        #[opt(default = 20)] duration_ms: i32,
        #[opt(default = -1.0)] amplitude: f32,
    ) {
        if global().bind().is_mobile {
            Input::singleton()
                .vibrate_handheld_ex()
                .duration_ms(duration_ms)
                .amplitude(amplitude)
                .done();
        }
    }

    // URLs and the guest device.

    #[func]
    pub fn open_webview_url(url: GString) {
        session::open_webview_url(&url.to_string());
    }

    #[func]
    pub fn open_url(url: GString, #[opt(default = false)] use_webkit: bool) {
        session::open_url(&url.to_string(), use_webkit);
    }

    #[func]
    pub fn get_device_anchor_id() -> GString {
        session::get_device_anchor_id()
    }

    #[func]
    pub fn clear_guest_device_storage() -> i32 {
        session::clear_guest_device_storage()
    }

    // The debug aids.

    #[func]
    pub fn set_raycast_debugger_enable(enable: bool) {
        use crate::app::raycast_debugger::RaycastDebugger;
        let current = global()
            .bind()
            .raycast_debugger
            .clone()
            .filter(|d| d.is_instance_valid());
        match (current, enable) {
            (None, true) => {
                let debugger = RaycastDebugger::new_alloc();
                global().add_child(&debugger);
                global().bind_mut().raycast_debugger = Some(debugger);
            }
            (Some(mut debugger), false) => {
                global().remove_child(&debugger);
                debugger.queue_free();
                global().bind_mut().raycast_debugger = None;
            }
            _ => {}
        }
    }

    /// Shows a scene raycast in the debugger, when it is on.
    pub fn add_raycast(id: i64, time: f32, from: Vector3, to: Vector3) {
        let debugger = global().bind().raycast_debugger.clone();
        if let Some(mut debugger) = debugger.filter(|d| d.is_instance_valid()) {
            debugger.bind_mut().add_raycast(id, time, from, to);
        }
    }

    #[func]
    pub fn is_gp_benchmark() -> bool {
        let g = global();
        let g = g.bind();
        g.cli.bind().gp_benchmark || g.deep_link_obj.bind().get_gp_benchmark()
    }

    // The entry points the emulator's scene-request sink calls.

    #[func]
    pub fn async_teleport_to(parcel_position: Vector2i, new_realm: GString) {
        let _ = crate::fetcher::api::DclNavigator::teleport_to(parcel_position, new_realm);
    }

    #[func]
    pub fn async_join_world(world_realm: GString) {
        let _ = crate::fetcher::api::DclNavigator::join_world(world_realm);
    }

    /// Runs the logging self-test if the CLI or the deeplink `test-logging` param asked for it; at
    /// most once. The test is taken out of `Global` while it runs, since it reads `Global` back.
    pub fn maybe_run_logging_selftest(deeplink_param: &str) {
        let cli_flag = globals::cli().bind().test_logging;
        let mut test = std::mem::take(&mut global().bind_mut().logging_selftest);
        test.maybe_run(cli_flag, deeplink_param);
        global().bind_mut().logging_selftest = test;
    }

    /// Signs the player out: the single teardown every logout funnels through. It stops the
    /// session's background work, kills every scene, resets the realm and the fetcher, closes
    /// comms and clears the identity, then returns the layout to portrait. Graphics, the last realm
    /// and the content cache stay: this is a sign-out, not a factory reset.
    #[func]
    pub fn sign_out() {
        crate::app::sign_out::sign_out();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_square_window_counts_as_portrait() {
        assert!(window_is_portrait(Vector2i::new(720, 1280)));
        assert!(window_is_portrait(Vector2i::new(800, 800)));
        assert!(!window_is_portrait(Vector2i::new(1280, 720)));
    }

    #[test]
    fn the_default_windows_match_their_orientation() {
        assert!(window_is_portrait(default_window_size(true)));
        assert!(!window_is_portrait(default_window_size(false)));
    }
}
