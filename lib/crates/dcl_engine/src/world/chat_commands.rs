//! The chat's `/command`s and the plain messages sent to the room.

use godot::classes::{Engine, Os};
use godot::prelude::*;

use super::{chat, navigate, state};
use crate::app::analytics_controller;
use crate::fetcher::api::DclNavigator;
use crate::fetcher::globals;
use crate::godot_classes::dcl_crash_generator::DclCrashGenerator;
use crate::godot_classes::dcl_global::DclGlobal;
use crate::godot_classes::dcl_realm::DclRealm;

/// Crash and hang triggers used to rehearse the Sentry pipeline on a device. Production builds
/// answer with a notice; the crash generator refuses too.
const DEBUG_ONLY_COMMANDS: [&str; 7] = [
    "/scenecrash",
    "/godotcrash",
    "/instantcrash",
    "/nativecrash",
    "/delayedcrash",
    "/anr",
    "/stall",
];
const DEBUG_BLOCK_MS: u32 = 8000;
const ANDROID_PLUGIN: &str = "dcl-godot-android";

pub fn submit_message(message: &str) {
    if message.is_empty() {
        return;
    }
    let params: Vec<&str> = message.split(' ').collect();
    let command = params[0].to_lowercase();
    if !command.starts_with('/') {
        if globals::comms().bind_mut().send_chat(message.into()) {
            analytics_controller::track_first_social_action("chat");
        }
        let address = globals::player_identity()
            .bind()
            .get_address_str()
            .to_string();
        let now = godot::classes::Time::singleton().get_unix_time_from_system();
        DclGlobal::singleton()
            .signals()
            .on_chat_message()
            .emit(address.as_str(), message, now);
        return;
    }
    run_command(&command, &params);
}

fn run_command(command: &str, params: &[&str]) {
    match command {
        "/go" | "/goto" if params.len() > 1 => go_to(&params[1..].join(" ")),
        "/changerealm" if params.len() > 1 => change_realm(params[1]),
        "/pos" => say_position(),
        "/clear" => navigate::clear_realm(),
        "/reload" => {
            DclNavigator::reload_current("on_reload".into());
        }
        _ if DEBUG_ONLY_COMMANDS.contains(&command) => run_debug_command(command),
        _ => chat::say_translated_plain("CHAT_SYSTEM_UNKNOWN_COMMAND"),
    }
}

fn go_to(argument: &str) {
    let argument = argument.trim();
    if let Some(parcel) = parse_coordinates(argument) {
        navigate::go_to_parcel(parcel);
    } else if dcl_core::realm::is_dcl_ens(argument) || !argument.contains('.') {
        let world = if argument.ends_with(".dcl.eth") {
            argument.to_string()
        } else {
            format!("{argument}.dcl.eth")
        };
        DclNavigator::join_world(world.as_str().into());
    } else {
        navigate::change_realm_with_notice(argument, "on_goto_realm");
    }
}

fn change_realm(target: &str) {
    if dcl_core::realm::is_dcl_ens(target) {
        DclNavigator::join_world(target.into());
    } else {
        navigate::change_realm_with_notice(target, "on_changerealm");
    }
}

fn run_debug_command(command: &str) {
    if DclGlobal::is_production() {
        chat::say("🔴 Debug commands are disabled in production builds");
        return;
    }
    match command {
        "/scenecrash" => globals::scene_runner()
            .bind_mut()
            .debug_force_crash_current_scene(),
        "/godotcrash" => Os::singleton().crash("User crashed on purpose"),
        "/instantcrash" | "/nativecrash" => DclCrashGenerator::static_crash(),
        "/delayedcrash" => {
            if let Some(world) = state::current() {
                world
                    .explorer
                    .clone()
                    .add_child(&DclCrashGenerator::new_alloc());
            }
        }
        "/anr" => block_ui_thread(),
        _ => block_main_loop(),
    }
}

fn block_main_loop() {
    // Trips the Rust main-thread stall detector (memory_monitor.rs).
    chat::say_dim(&format!(
        "Blocking the main loop for {}s",
        DEBUG_BLOCK_MS / 1000
    ));
    Os::singleton().delay_msec(DEBUG_BLOCK_MS as i32);
}

/// On Android the main loop runs on the render thread, so blocking it never gives a system ANR;
/// the plugin blocks the UI thread instead and the tester has to keep tapping so input dispatch
/// times out.
fn block_ui_thread() {
    if DclGlobal::singleton().bind().is_android {
        // `memory_trim` marks the plugin revision that also ships `debugBlockUiThread`
        // (`has_method` never sees JNISingleton methods).
        if let Some(mut plugin) = Engine::singleton().get_singleton(ANDROID_PLUGIN) {
            if plugin.has_signal("memory_trim") {
                chat::say_dim(&format!(
                    "Blocking the Android UI thread for {}s - keep tapping the screen",
                    DEBUG_BLOCK_MS / 1000
                ));
                plugin.call("debugBlockUiThread", &[DEBUG_BLOCK_MS.to_variant()]);
                return;
            }
        }
    }
    block_main_loop();
}

fn is_integer(text: &str) -> bool {
    let digits = text.strip_prefix('-').unwrap_or(text);
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}

/// `x,y` or `(x, y)`; both parts must be integers.
pub fn parse_coordinates(text: &str) -> Option<Vector2i> {
    let cleaned: String = text
        .trim()
        .chars()
        .filter(|c| !matches!(c, '(' | ')' | ' '))
        .collect();
    let mut parts = cleaned.split(',');
    let (x, y) = (parts.next()?, parts.next()?);
    (is_integer(x) && is_integer(y))
        .then(|| Some(Vector2i::new(x.parse().ok()?, y.parse().ok()?)))
        .flatten()
}

fn say_position() {
    let Some(world) = state::current() else {
        return;
    };
    let Some(camera) = world
        .explorer
        .get_viewport()
        .and_then(|v| v.get_camera_3d())
    else {
        chat::say_translated_plain("CHAT_SYSTEM_NO_CAMERA");
        return;
    };
    // Decentraland is X right, Y up, Z north; Godot's Z points backward: dcl = (x, y, -z).
    let to_dcl = |p: Vector3| Vector3::new(p.x, p.y, -p.z);
    let parcel_of =
        |p: Vector3| Vector2i::new((p.x / 16.0).floor() as i32, (p.z / 16.0).floor() as i32);
    let relative = |p: Vector3, parcel: Vector2i| {
        Vector3::new(
            p.x - parcel.x as f32 * 16.0,
            p.y,
            p.z - parcel.y as f32 * 16.0,
        )
    };

    let player = to_dcl(world.player.get_global_position());
    let cam = to_dcl(camera.get_global_position());
    let (parcel_player, parcel_cam) = (parcel_of(player), parcel_of(cam));
    let (scene_id, base) = {
        let runner = globals::scene_runner();
        let runner = runner.bind();
        let id = runner.get_current_parcel_scene_id();
        (id, runner.get_scene_base_parcel(id))
    };
    let _ = scene_id;
    let forward_godot = -camera.get_global_transform().basis.col_c();
    let mut forward = to_dcl(forward_godot);
    if forward.length_squared() > 0.0001 {
        forward = forward.normalized();
    }
    let (realm_display, realm_type) = describe_realm();
    let (rel_pp, rel_bp) = (relative(player, parcel_player), relative(player, base));
    let (rel_pc, rel_bc) = (relative(cam, parcel_cam), relative(cam, base));
    let message = format!(
        "[color=#cfc][b]Position (DCL)[/b][/color]\n\
         Realm: {realm_display}  [{realm_type}]\n\
         Player world: ({:.2}, {:.2}, {:.2})  Parcel: ({}, {})\n  \
         rel parcel: ({:.2}, {:.2}, {:.2})  rel base: ({:.2}, {:.2}, {:.2})\n\
         Camera world: ({:.2}, {:.2}, {:.2})  Parcel: ({}, {})\n  \
         rel parcel: ({:.2}, {:.2}, {:.2})  rel base: ({:.2}, {:.2}, {:.2})\n\
         Camera dir (unit): ({:.4}, {:.4}, {:.4})",
        player.x,
        player.y,
        player.z,
        parcel_player.x,
        parcel_player.y,
        rel_pp.x,
        rel_pp.y,
        rel_pp.z,
        rel_bp.x,
        rel_bp.y,
        rel_bp.z,
        cam.x,
        cam.y,
        cam.z,
        parcel_cam.x,
        parcel_cam.y,
        rel_pc.x,
        rel_pc.y,
        rel_pc.z,
        rel_bc.x,
        rel_bc.y,
        rel_bc.z,
        forward.x,
        forward.y,
        forward.z,
    );
    chat::say(&message);
}

/// The realm's display name and kind (main, world, preview or other).
fn describe_realm() -> (String, &'static str) {
    let realm: Gd<DclRealm> = globals::realm();
    let url = realm.bind().realm_url.to_string();
    let mut display = realm.bind().get_realm_string().to_string();
    if display.is_empty() {
        display = url.clone();
    }
    let kind = if dcl_core::realm::is_genesis_city(&url) {
        "main"
    } else if dcl_core::realm::is_dcl_ens(&display) || display.ends_with(".dcl.eth") {
        "world"
    } else if dcl_core::realm::is_local_preview(&url) {
        "preview"
    } else {
        "realm"
    };
    (display, kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinates_take_parentheses_and_spaces() {
        assert_eq!(parse_coordinates("12,-4"), Some(Vector2i::new(12, -4)));
        assert_eq!(parse_coordinates("( 3 , 5 )"), Some(Vector2i::new(3, 5)));
    }

    #[test]
    fn coordinates_need_two_integers() {
        assert_eq!(parse_coordinates("genesis"), None);
        assert_eq!(parse_coordinates("1"), None);
        assert_eq!(parse_coordinates("1,x"), None);
        assert_eq!(parse_coordinates("+1,2"), None);
        assert_eq!(parse_coordinates("1.5,2"), None);
    }

    #[test]
    fn debug_commands_are_listed() {
        assert!(DEBUG_ONLY_COMMANDS.contains(&"/anr"));
        assert!(!DEBUG_ONLY_COMMANDS.contains(&"/pos"));
    }
}
