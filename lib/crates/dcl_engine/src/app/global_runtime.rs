//! The boot and the foreground notification of `Global`. The boot takes the handle and runs from
//! the autoload script's `_ready`, because a Rust `ready` would hold `Global`'s borrow while the
//! services read `Global` back at every step; the notification has a node of its own for the same
//! reason.

use godot::classes::display_server::VSyncMode;
use godot::classes::{ConfigFile, DirAccess, DisplayServer, Engine, INode, Node, Os};
use godot::prelude::*;

use super::deep_link;
use super::graphics;
use super::safe_area_presets;
use super::session::LOCAL_ASSETS_CACHE_VERSION;
use crate::engine::bootstrap;
use crate::engine::local_task::{await_promise, spawn_local_now};
use crate::fetcher::api::DclNavigator;
use crate::fetcher::globals;
use crate::godot_classes::dcl_android_plugin::DclAndroidPlugin;
use crate::godot_classes::dcl_global::DclGlobal;
use crate::godot_classes::dcl_ios_plugin::DclIosPlugin;
use crate::godot_classes::dcl_parse_deep_link::DclParseDeepLink;
use crate::godot_classes::dcl_swift_lib::DclSwiftLibPlugin;
use crate::profile::profile_service::ProfileService;

const GENERATED_BUILD_CONFIG: &str = "res://src/generated/build_config.cfg";
const ROOT_SCENE: &str = "res://src/dcl_root.tscn";

/// The deeplink an xtask run baked into the build, so it applies even where no CLI args reach the
/// app (a plain build and manual install, a tapped icon, TestFlight). Never honored in production:
/// the iOS runner exports with `clean: false`, so a stale file from a debug build could otherwise
/// leak into a production IPA.
fn generated_deeplink() -> String {
    if DclGlobal::is_production() {
        return String::new();
    }
    let mut config = ConfigFile::new_gd();
    if config.load(GENERATED_BUILD_CONFIG) != godot::global::Error::OK {
        return String::new();
    }
    config
        .get_value_ex("build", "deeplink")
        .default(&"".to_variant())
        .done()
        .try_to::<GString>()
        .map(|s| s.to_string())
        .unwrap_or_default()
}

/// The deeplink to apply at startup: `--fake-deeplink`, else the baked one.
fn startup_deeplink(cli_link: &str) -> String {
    if cli_link.is_empty() {
        generated_deeplink()
    } else {
        cli_link.to_string()
    }
}

/// Runs the boot from an autoload of its own, placed after `Global`: the boot adds the services as
/// children of `Global` and reads `Global` back at every step, which neither `Global`'s own `ready`
/// (it holds the borrow) nor a child of `Global` (its parent is still setting up) allows.
#[derive(GodotClass)]
#[class(base=Node, init)]
pub struct GlobalBoot {
    base: Base<Node>,
}

#[godot_api]
impl INode for GlobalBoot {
    fn ready(&mut self) {
        if godot::classes::Engine::singleton().is_editor_hint() {
            return;
        }
        let started = godot::classes::Time::singleton().get_ticks_msec();
        godot_print!("[Startup] global._ready start: 0ms");
        boot_services();
        let host = crate::world::EngineHost::new_alloc();
        self.base_mut().add_child(&host);
        let watcher = AppFocusWatcher::new_alloc();
        self.base_mut().add_child(&watcher);
        spawn_local_now(finish_boot(started));
        // The launch intent (the tap that started the app).
        check_launch_link(true);
    }
}

/// Reacts to the app coming to the foreground. A node of its own, with no other work, because
/// every `_notification` runs under the node's borrow: on the boot node it would be re-entered by
/// the notifications the boot itself causes (a locale change reaches every node).
#[derive(GodotClass)]
#[class(base=Node, init)]
pub struct AppFocusWatcher {
    base: Base<Node>,
}

#[godot_api]
impl INode for AppFocusWatcher {
    fn on_notification(&mut self, what: godot::classes::notify::NodeNotification) {
        if what != godot::classes::notify::NodeNotification::APPLICATION_FOCUS_IN {
            return;
        }
        let real_mobile = {
            let g = DclGlobal::singleton();
            let g = g.bind();
            g.is_mobile && !g.is_virtual_mobile
        };
        if real_mobile {
            // Mobile OSes cut networking shortly after backgrounding, killing every comms session.
            // Tell comms we are back so it forgives transient reconnect failures and treats
            // Duplicate* evictions as our own stale session being reclaimed. Real phones only: on
            // the desktop every alt-tab fires focus-in, and a 30 s grace window armed that often
            // would retry genuine evictions instead of surfacing the modal.
            globals::comms().bind_mut().notify_app_resumed();
        }
        check_launch_link(false);
    }
}

/// Reads a link the platform plugin received, on a cold start (the launch intent) or a resume.
fn check_launch_link(cold: bool) {
    let (real_mobile, is_android) = {
        let g = DclGlobal::singleton();
        let g = g.bind();
        (g.is_mobile && !g.is_virtual_mobile, g.is_android)
    };
    if !real_mobile {
        return;
    }
    let args = if DclAndroidPlugin::is_available() {
        DclAndroidPlugin::get_deeplink_args()
    } else if DclIosPlugin::is_available() {
        DclIosPlugin::get_deeplink_args()
    } else {
        return;
    };
    let _ = is_android;
    let url = args
        .get("data")
        .and_then(|v| v.try_to::<GString>().ok())
        .map(|s| s.to_string())
        .unwrap_or_default();
    // Only a new URL: overwriting the stored one with an empty one would clobber what the iOS signal
    // path already set.
    if url.is_empty() {
        return;
    }
    if cold {
        // On a cold start the environment comes from the link before it is processed, or the dclenv
        // change check would see the default "org" against the link's and sign out prematurely,
        // skipping the orientation and UI scale setup of the root.
        let parsed = DclParseDeepLink::parse_decentraland_link(url.as_str().into());
        let env = parsed.bind().get_dclenv().to_string();
        if !env.is_empty() {
            DclGlobal::set_dcl_environment(env.as_str().into());
            DclGlobal::singleton().bind_mut().dcl_env_explicit = true;
        }
    }
    let mut router = DclGlobal::singleton().bind().deep_link_router.clone();
    router.bind_mut().process_deep_link(
        url.as_str().into(),
        if cold { "cold" } else { "warm" }.into(),
    );
}

/// The synchronous part of the boot: the mobile and emulation flags, the startup deeplink and the
/// services every boot needs.
fn boot_services() {
    let mut global = DclGlobal::singleton();
    let cli = globals::cli();
    // Bench only: uncap the FPS and disable vsync before any code path can re-pin them. Real users
    // keep their saved cap and vsync.
    if cli.bind().bench_mode {
        let mut engine = Engine::singleton();
        engine.set_max_fps(0);
        DisplayServer::singleton().window_set_vsync_mode(VSyncMode::DISABLED);
        Os::singleton().set_low_processor_usage_mode(false);
    }
    if cli.bind().force_mobile {
        global.bind_mut()._set_is_mobile(true);
    }

    // Emulating a phone enables mobile mode and resizes the window.
    if let Some(phone) = super::global_api::emulated_phone() {
        global.bind_mut()._set_is_mobile(true);
        let portrait = global.bind().is_portrait;
        if let Some(mut window) = global.get_tree_or_null().and_then(|t| t.get_root()) {
            window.set_size(safe_area_presets::window_size(phone, portrait));
            window.move_to_center();
            let mut overlay = super::phone_frame::PhoneFrameOverlay::new_alloc();
            overlay.set_name("PhoneFrameOverlay");
            global.clone().add_child(&overlay);
        }
        if cli.bind().landscape {
            DclGlobal::set_orientation_landscape();
        }
    }

    apply_startup_deeplink();

    // iOS delivers a warm link through a plugin signal; the cold one is read at ready.
    if DclIosPlugin::is_available() {
        if let Some(mut ios) = Engine::singleton().get_singleton("DclGodotiOS") {
            let mut router = global.bind().deep_link_router.clone();
            // The plugin is not a class of this crate: its signal is connected by name.
            ios.connect(
                "deeplink_received",
                &Callable::from_fn("deeplink_received", move |args| {
                    if let Some(url) = args.first().and_then(|v| v.try_to::<GString>().ok()) {
                        router.bind_mut().process_deep_link(url, "warm".into());
                    }
                    Variant::nil()
                }),
            );
        }
    }
    smoke_test_swift_lib();
    // The services every boot needs (realm, identity, ...) are created in Rust.
    bootstrap::create_core_services(&mut global.clone());

    // A run of one scene opened directly (not the app root) is standalone.
    let args = Os::singleton().get_cmdline_args();
    if args.len() == 1 {
        if let Some(scene) = args.get(0).map(|a| a.to_string()) {
            if scene.starts_with("res://") && scene != ROOT_SCENE {
                global.clone().bind_mut().standalone = true;
            }
        }
    }
}

/// The deeplink applied at startup, from `--fake-deeplink` or the baked one.
fn apply_startup_deeplink() {
    let cli_link = globals::cli().bind().fake_deeplink.to_string();
    let link = startup_deeplink(&cli_link);
    if link.is_empty() {
        return;
    }
    let obj = DclParseDeepLink::parse_decentraland_link(link.as_str().into());
    {
        let mut global = DclGlobal::singleton();
        let mut global = global.bind_mut();
        global.deep_link_url = link.as_str().into();
        global.deep_link_obj = obj.clone();
    }
    godot_print!(
        "[DEEPLINK] Parsed fake deep_link_obj: location={} realm={} preview={}",
        obj.bind().get_location(),
        obj.bind().get_realm(),
        obj.bind().get_preview()
    );
    godot_print!("[DEEPLINK] All params: {}", obj.bind().get_params());
    deep_link::apply_link_settings(&obj);
    godot_print!("[DEEPLINK] scene-stats={}", obj.bind().get_scene_stats());
}

/// A smoke test of the Swift extension, on iOS only where its class can be reached.
fn smoke_test_swift_lib() {
    if !DclSwiftLibPlugin::is_available() {
        return;
    }
    godot_print!(
        "[DclSwiftLib] ping() -> {} | version() -> {}",
        DclSwiftLibPlugin::ping(),
        DclSwiftLibPlugin::version()
    );
}

/// The rest of the boot, after the content cache was cleared if needed.
async fn finish_boot(started_ms: u64) {
    let global = DclGlobal::singleton();
    let cli = globals::cli();
    let engine = globals::engine();

    // Restore the particle budgets and the avatar dust gate: the Rust atomics default to the full
    // budget, and nothing re-applies the profile until the player picks one again.
    let particle_quality = engine
        .bind()
        .setting("particle_quality".into())
        .try_to::<i64>()
        .unwrap_or(0) as i32;
    graphics::apply_particle_quality(particle_quality);
    crate::avatars::avatar::lod::apply_particles_enabled(particle_quality > 0);

    let link = global.bind().deep_link_obj.clone();
    deep_link::capture_debug_guest_rotate(&link, DclGlobal::is_production());

    // Resolve the UI language before any scene renders: Godot picks the OS locale at boot, which
    // would surface a partially translated locale the moment its catalogue has content.
    super::locale::apply_locale();
    // Bench only: keep limit_fps at NO_LIMIT after the settings load (which would restore a saved
    // cap) so no later apply re-pins the engine.
    if cli.bind().bench_mode {
        engine.bind().adopt_setting(
            "limit_fps".into(),
            (graphics::FpsLimitMode::NoLimit as i32).to_variant(),
        );
        Engine::singleton().set_max_fps(0);
        DisplayServer::singleton().window_set_vsync_mode(VSyncMode::DISABLED);
    }

    // The environment: `--dclenv` over a deeplink `dclenv` over `org`.
    let (env, source) = {
        let cli = cli.bind();
        let link_env = link.bind().get_dclenv().to_string();
        if !cli.dcl_env.is_empty() {
            (cli.dcl_env.to_string(), "--dclenv")
        } else if !link_env.is_empty() {
            (link_env, "deeplink")
        } else {
            ("org".to_string(), "default")
        }
    };
    DclGlobal::set_dcl_environment(env.as_str().into());
    global.clone().bind_mut().dcl_env_explicit = source != "default";
    if env != "org" {
        godot_print!("[GLOBAL] Environment set to: {env} (source: {source})");
    }

    // Dev/testing: no profile deploys, so fake-owned wearables never publish.
    if link.bind().get_disable_profile_deploy() {
        ProfileService::set_deploy_disabled(true);
        godot_print!("[GLOBAL] Profile deploy DISABLED (local-only profile updates)");
    }
    globals::realm()
        .signals()
        .realm_access_denied()
        .connect(|new_realm, _world| DclNavigator::recover_from_refusal(new_realm));

    // The content cache folder must exist before it is cleared (a fresh install has none).
    ensure_content_dir();
    clear_cache_if_needed().await;

    // The test runner and the floating-islands benchmark boot without the app services.
    if bootstrap::start_special_profile(&mut global.clone()) {
        return;
    }
    // The Genesis Plaza profiling benchmark (issue #1862) lives alongside the full explorer flow:
    // it primes the realm and parcel, then samples once the scene loads. Skipped if the deeplink
    // path already started the runner (the mobile cold-start race).
    if DclGlobal::is_gp_benchmark() && !global.has_node("GPBenchmarkRunner") {
        godot_print!("Running Genesis Plaza Benchmark...");
        let mut node: Gd<DclGlobal> = global.clone();
        bootstrap::spawn_gp_benchmark_runner(&mut node);
    }
    ensure_content_dir();

    let session_id = DclGlobal::generate_uuid_v4();
    global.clone().bind_mut().session_id = session_id.clone();
    // No Segment metrics or Sentry tagging in asset-server mode, or when telemetry is disabled at
    // build time (CI desktop builds use the `disable_telemetry` cargo feature).
    let telemetry = !cli.bind().asset_server && !DclGlobal::is_telemetry_disabled();
    bootstrap::create_app_services(&mut global.clone(), &session_id.to_string(), telemetry);
    // Install attribution fires once per install, on Android.
    let install_pending = global.bind().is_android
        && !engine
            .bind()
            .setting("install_referrer_sent".into())
            .try_to::<bool>()
            .unwrap_or(false);
    if install_pending {
        globals::metrics().run_deferred(|metrics| metrics.track_install_referrer());
        engine
            .bind()
            .set_setting("install_referrer_sent".into(), true.to_variant(), true);
    }
    bootstrap::create_extra_services(&mut global.clone());

    // The dynamic graphics manager starts after the settings are loaded; it skips itself in bench
    // mode so results stay comparable, while production keeps the thermal cap and adaptive downgrade.
    let mut autotune = global.bind().graphics_autotune.clone();
    autotune.run_deferred(|autotune| autotune.init_dynamic_graphics());

    if cli.bind().raycast_debugger {
        DclGlobal::set_raycast_debugger_enable(true);
    }
    global
        .bind()
        .network_inspector
        .clone()
        .bind_mut()
        .set_is_active(cli.bind().network_debugger);

    // The logging self-test, from the CLI (e.g. iOS's baked arg) now and from a deeplink on
    // arrival; it runs at most once.
    DclGlobal::maybe_run_logging_selftest("");
    let router = global.bind().deep_link_router.clone();
    router.signals().deep_link_received().connect(|| {
        let param = DclGlobal::singleton()
            .bind()
            .deep_link_obj
            .bind()
            .get_params()
            .get("test-logging")
            .map(|v| v.stringify().to_string())
            .unwrap_or_default();
        DclGlobal::maybe_run_logging_selftest(&param);
    });
    godot_print!(
        "[Startup] global._ready end: {}ms",
        godot::classes::Time::singleton().get_ticks_msec() - started_ms
    );
}

fn ensure_content_dir() {
    if !DirAccess::dir_exists_absolute("user://content/") {
        let _ = DirAccess::make_dir_absolute("user://content/");
    }
}

/// Clears the content cache before anything loads, when the startup flag asked for it or the cache
/// format version changed. Not in asset-server mode: it keeps the download cache.
async fn clear_cache_if_needed() {
    let cli = globals::cli();
    if cli.bind().asset_server {
        return;
    }
    let engine = globals::engine();
    let clear_at_startup = cli.bind().clear_cache_startup;
    let version_changed = engine
        .bind()
        .setting("local_assets_cache_version".into())
        .try_to::<i64>()
        .unwrap_or(-1)
        != LOCAL_ASSETS_CACHE_VERSION;
    if !clear_at_startup && !version_changed {
        return;
    }
    if clear_at_startup {
        godot_print!("Clear cache startup!");
    }
    if version_changed {
        godot_print!("Local assets cache version changed, clearing cache!");
    }
    let promise = globals::content_provider().bind_mut().clear_cache_folder();
    await_promise(promise).await;
    godot_print!("Cache cleared successfully!");
    if version_changed {
        engine.bind().set_setting(
            "local_assets_cache_version".into(),
            LOCAL_ASSETS_CACHE_VERSION.to_variant(),
            true,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::startup_deeplink;

    #[test]
    fn the_cli_link_beats_the_baked_one() {
        assert_eq!(
            startup_deeplink("decentraland://open?x=1"),
            "decentraland://open?x=1"
        );
    }
}
