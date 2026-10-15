//! The main scene: picks what the process runs (the lobby, the explorer, a test runner or a
//! tool) from the command line, then hands off to that scene.

use std::time::Instant;

use godot::classes::{ClassDb, InputMap, Os, ResourceLoader};
use godot::prelude::*;

use crate::godot_classes::dcl_global::DclGlobal;

const EXPLORER: &str = "res://src/ui/explorer.tscn";
const ENGINE_IDLE: &str = "res://src/engine_host/avatar_host.tscn";

/// Where a command line sends the process. The first match wins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootMode {
    AssetServer,
    /// The floating-islands benchmark builds its own scene.
    FiBenchmark,
    AvatarImpostorBenchmark,
    Scene(&'static str),
    /// Scene tests and scene renders: a throwaway guest, straight into the explorer.
    SceneTest,
    /// No host: the emulator, a throwaway guest straight into a world.
    Emulator,
}

/// The routing facts of a command line.
#[derive(Debug, Default, Clone)]
pub struct BootFlags {
    pub asset_server: bool,
    pub fi_benchmark: bool,
    pub avatar_impostor_benchmark: bool,
    pub emote_test: bool,
    pub avatar_renderer: bool,
    pub asset_renderer: bool,
    pub client_test: bool,
    pub lighting_renderer: bool,
    /// The host booted the core before the engine started.
    pub host_driven: bool,
    pub scene_test: bool,
    pub scene_renderer: bool,
    pub preview: bool,
    pub has_realm: bool,
}

pub fn boot_mode(flags: &BootFlags) -> BootMode {
    if flags.asset_server {
        BootMode::AssetServer
    } else if flags.fi_benchmark {
        BootMode::FiBenchmark
    } else if flags.avatar_impostor_benchmark {
        BootMode::AvatarImpostorBenchmark
    } else if flags.emote_test {
        BootMode::Scene("res://src/test/emote/emote_tester_standalone.tscn")
    } else if flags.avatar_renderer {
        BootMode::Scene("res://src/tool/avatar_renderer/avatar_renderer_standalone.tscn")
    } else if flags.asset_renderer {
        BootMode::Scene("res://src/tool/asset_renderer/asset_renderer_standalone.tscn")
    } else if flags.client_test {
        BootMode::Scene("res://src/client_tests/client_test_scene.tscn")
    } else if flags.lighting_renderer {
        BootMode::Scene("res://src/tool/lighting_renderer/lighting_renderer_standalone.tscn")
    } else if flags.host_driven {
        BootMode::Scene(ENGINE_IDLE)
    } else if flags.scene_test || flags.scene_renderer {
        BootMode::SceneTest
    } else {
        BootMode::Emulator
    }
}

/// Calls a `static func` of a GDScript class.
pub(super) fn call_static(script: &str, method: &str, args: &[Variant]) -> Variant {
    let loaded = ResourceLoader::singleton()
        .load_ex(script)
        .type_hint("Script")
        .done();
    match loaded {
        Some(mut script) => script.call(method, args),
        None => {
            godot_error!("root: cannot load {script}");
            Variant::nil()
        }
    }
}

#[derive(GodotClass)]
#[class(base=Node)]
pub struct DclRoot {
    started: Instant,
    base: Base<Node>,
}

#[godot_api]
impl INode for DclRoot {
    fn init(base: Base<Node>) -> Self {
        Self {
            started: Instant::now(),
            base,
        }
    }

    fn ready(&mut self) {
        godot_print!(
            "[Startup] root._ready: {}ms",
            self.started.elapsed().as_millis()
        );
        let mut global = DclGlobal::singleton();
        if !global.bind().get_engine().bind().is_host_driven() {
            global.call("set_orientation_portrait", &[]);
        }
        // #2386: apply the UI content-scale synchronously, before the first frame is presented,
        // so the splash overlay and all UI render at their final scale from frame 0.
        if let Some(window) = self.base().get_window() {
            call_static(
                "res://src/config/graphic_settings.gd",
                "apply_ui_zoom",
                &[window.to_variant()],
            );
        }
        self.base_mut().call_deferred("start", &[]);
    }
}

#[godot_api]
impl DclRoot {
    #[func]
    fn start(&mut self) {
        godot_print!(
            "[Startup] root.start: {}ms",
            self.started.elapsed().as_millis()
        );
        let global = DclGlobal::singleton();
        let cli = global.bind().get_cli();

        if let Some(mut tree) = self.base().get_tree() {
            tree.set_quit_on_go_back(false);
            if cli.bind().is_help_requested() {
                // The help text is already printed.
                tree.quit_ex().exit_code(0).done();
                return;
            }
        }
        // `Global` already started the test runner.
        if cli.bind().test_runner {
            return;
        }

        if Os::singleton().has_feature("Server") {
            godot_print!(
                "Running from Server - version {}",
                global.bind().renderer_version
            );
        } else {
            godot_print!(
                "Running from platform - version {}",
                global.bind().renderer_version
            );
            self.apply_display_settings(&global);
        }

        if global.clone().call("is_mobile", &[]).to::<bool>() {
            InputMap::singleton().action_erase_events("ia_pointer");
        }
        self.base_mut().call_deferred("start_mode", &[]);
    }

    fn apply_display_settings(&self, global: &Gd<DclGlobal>) {
        let Some(mut window) = self.base().get_tree().and_then(|tree| tree.get_root()) else {
            return;
        };
        call_static(
            "res://src/config/graphic_settings.gd",
            "apply_window_config",
            &[],
        );
        call_static(
            "res://src/config/graphic_settings.gd",
            "apply_low_processor_mode",
            &[],
        );
        window.move_to_center();
        let window_variant = window.to_variant();
        call_static(
            "res://src/config/graphic_settings.gd",
            "connect_global_signal",
            std::slice::from_ref(&window_variant),
        );
        call_static(
            "res://src/config/graphic_settings.gd",
            "apply_ui_zoom",
            &[window_variant],
        );
        let scale = global.bind().get_config().get("resolution_3d_scale");
        window.set("scaling_3d_scale", &scale);
        call_static(
            "res://src/config/audio_settings.gd",
            "apply_volume_settings",
            &[],
        );
        call_static(
            "res://src/config/general_settings.gd",
            "apply_max_cache_size",
            &[],
        );
    }

    #[func]
    fn start_mode(&mut self) {
        godot_print!(
            "[Startup] root.start_mode: {}ms",
            self.started.elapsed().as_millis()
        );
        let global = DclGlobal::singleton();
        let flags = {
            let cli = global.bind().get_cli();
            let cli = cli.bind();
            BootFlags {
                asset_server: cli.asset_server,
                fi_benchmark: cli.fi_benchmark_size >= 0,
                avatar_impostor_benchmark: cli.avatar_impostor_benchmark,
                emote_test: cli.emote_test_mode,
                avatar_renderer: cli.avatar_renderer_mode,
                asset_renderer: cli.asset_renderer_mode,
                client_test: cli.client_test_mode,
                lighting_renderer: cli.lighting_renderer,
                host_driven: global.bind().get_engine().bind().is_host_driven(),
                scene_test: cli.scene_test_mode,
                scene_renderer: cli.scene_renderer_mode,
                preview: cli.preview_mode,
                has_realm: !cli.realm.is_empty(),
            }
        };

        match boot_mode(&flags) {
            BootMode::AssetServer => {
                godot_print!("Running in Asset Server mode");
                self.start_asset_server(&global);
            }
            // The benchmark runner already built its own scene.
            BootMode::FiBenchmark => {}
            BootMode::AvatarImpostorBenchmark => {
                godot_print!("Running in Avatar Impostor Benchmark mode");
                Self::start_throwaway_guest(&global, true);
                self.change_scene("res://src/tools/avatar_impostor_benchmark.tscn");
            }
            BootMode::Scene(path) => {
                godot_print!("Running {path}");
                self.change_scene(path);
            }
            BootMode::SceneTest => {
                godot_print!("Running in Scene Test mode");
                Self::start_throwaway_guest(&global, true);
                Self::remember_throwaway_account(&global);
                self.change_scene(EXPLORER);
            }
            BootMode::Emulator => {
                godot_print!("Running in Emulator mode");
                Self::complete_emulator_arguments(&global);
                Self::start_throwaway_guest(&global, false);
                Self::remember_throwaway_account(&global);
                self.change_scene(EXPLORER);
            }
        }
    }

    fn change_scene(&mut self, path: &str) {
        if let Some(mut tree) = self.base().get_tree() {
            tree.change_scene_to_file(path);
        }
    }

    /// `--preview <url>` names the realm when `--realm` is not given, and a fresh guest starts at
    /// 0,0 unless `--location` says otherwise (otherwise it would stand where the last session
    /// ended, possibly parcels away from a preview scene).
    fn complete_emulator_arguments(global: &Gd<DclGlobal>) {
        let mut cli = global.bind().get_cli();
        let url = cli.bind().get_arg("--preview".into());
        if cli.bind().realm.is_empty() && !url.is_empty() {
            cli.bind_mut().realm = url;
        }
        if cli.bind().location.is_empty() {
            cli.bind_mut().location = "0,0".into();
        }
    }

    /// Creates a disposable guest. `persist` clears and saves the stored guest profile (as the
    /// scene tests always did); the emulator leaves the saved session alone.
    fn start_throwaway_guest(global: &Gd<DclGlobal>, persist: bool) {
        let mut config = global.bind().get_config();
        if persist {
            config.set("guest_profile", &VarDictionary::new().to_variant());
            config.call("save_to_settings_file", &[]);
        } else {
            config.set("session_is_ephemeral", &true.to_variant());
        }
        let mut identity = global.bind().get_player_identity();
        identity.bind_mut().set_default_profile();
        identity.call("create_disposable_account", &[]);
    }

    /// Mirrors the new account into the config so the explorer finds a session.
    fn remember_throwaway_account(global: &Gd<DclGlobal>) {
        let identity = global.bind().get_player_identity();
        let stored = VarDictionary::new();
        if identity.bind().get_recover_account_to(stored.clone()) {
            global
                .bind()
                .get_config()
                .set("session_account", &stored.to_variant());
        }
    }

    fn start_asset_server(&mut self, global: &Gd<DclGlobal>) {
        let class = StringName::from("DclAssetServer");
        if !ClassDb::singleton().class_exists(&class) {
            godot_error!(
                "Asset server requires the 'asset_server' feature to be enabled during build."
            );
            godot_error!("Build with: cargo run -- build --features asset_server");
            if let Some(mut tree) = self.base().get_tree() {
                tree.quit_ex().exit_code(1).done();
            }
            return;
        }
        let Some(instance) = ClassDb::singleton()
            .instantiate(&class)
            .try_to::<Gd<Node>>()
            .ok()
        else {
            godot_error!("Cannot create the asset server");
            return;
        };
        let mut server = instance;
        server.call(
            "set_port",
            &[global
                .bind()
                .get_cli()
                .bind()
                .asset_server_port
                .to_variant()],
        );
        server.set_name("AssetServer");
        if let Some(mut root) = self.base().get_tree().and_then(|tree| tree.get_root()) {
            root.add_child(&server);
        }
        server.call("start", &[]);
        godot_print!("Asset server is running. Press Ctrl+C to stop.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flags(change: impl FnOnce(&mut BootFlags)) -> BootFlags {
        let mut flags = BootFlags::default();
        change(&mut flags);
        flags
    }

    #[test]
    fn a_plain_command_line_opens_the_emulator() {
        assert_eq!(boot_mode(&BootFlags::default()), BootMode::Emulator);
    }

    #[test]
    fn the_first_matching_mode_wins() {
        let both = flags(|f| {
            f.asset_server = true;
            f.scene_test = true;
        });
        assert_eq!(boot_mode(&both), BootMode::AssetServer);

        let host_before_scene_test = flags(|f| {
            f.host_driven = true;
            f.scene_test = true;
        });
        assert_eq!(
            boot_mode(&host_before_scene_test),
            BootMode::Scene(ENGINE_IDLE)
        );
    }

    #[test]
    fn scene_tests_and_renders_share_a_boot() {
        assert_eq!(
            boot_mode(&flags(|f| f.scene_test = true)),
            BootMode::SceneTest
        );
        assert_eq!(
            boot_mode(&flags(|f| f.scene_renderer = true)),
            BootMode::SceneTest
        );
    }

    #[test]
    fn a_realm_or_preview_without_a_host_is_the_emulator() {
        assert_eq!(boot_mode(&flags(|f| f.preview = true)), BootMode::Emulator);
        assert_eq!(
            boot_mode(&flags(|f| f.has_realm = true)),
            BootMode::Emulator
        );
    }

    #[test]
    fn a_host_never_boots_the_emulator() {
        let hosted = flags(|f| {
            f.host_driven = true;
            f.has_realm = true;
        });
        assert_eq!(boot_mode(&hosted), BootMode::Scene(ENGINE_IDLE));
    }

    #[test]
    fn tools_have_their_own_scenes() {
        assert!(
            matches!(boot_mode(&flags(|f| f.client_test = true)), BootMode::Scene(path) if path.contains("client_test"))
        );
        assert!(
            matches!(boot_mode(&flags(|f| f.lighting_renderer = true)), BootMode::Scene(path) if path.contains("lighting"))
        );
    }
}
