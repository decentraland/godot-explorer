//! The main scene: picks what the process runs (the lobby, the explorer, a test runner or a
//! tool) from the command line, then hands off to that scene.

use std::time::Instant;

use godot::classes::{ClassDb, InputMap, Os};
use godot::prelude::*;

use crate::app::{audio_settings, general_settings, graphics};
use crate::godot_classes::dcl_global::DclGlobal;

const EXPLORER: &str = "res://src/ui/explorer.tscn";
const ENGINE_IDLE: &str = "res://src/engine_host/avatar_host.tscn";

/// Where a command line sends the process. The first match wins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootMode {
    AssetServer,
    /// The floating-islands benchmark builds its own scene.
    Scene(&'static str),
    /// Scene tests: a throwaway guest, straight into the explorer.
    SceneTest,
    /// No host: the emulator, a throwaway guest straight into a world.
    Emulator,
}

/// The routing facts of a command line.
#[derive(Debug, Default, Clone)]
pub struct BootFlags {
    pub asset_server: bool,
    pub avatar_renderer: bool,
    pub asset_renderer: bool,
    pub client_test: bool,
    /// The host booted the core before the engine started.
    pub host_driven: bool,
    pub scene_test: bool,
    pub preview: bool,
    pub has_realm: bool,
}

pub fn boot_mode(flags: &BootFlags) -> BootMode {
    if flags.asset_server {
        BootMode::AssetServer
    } else if flags.avatar_renderer {
        BootMode::Scene("res://src/tool/avatar_renderer/avatar_renderer_standalone.tscn")
    } else if flags.asset_renderer {
        BootMode::Scene("res://src/tool/asset_renderer/asset_renderer_standalone.tscn")
    } else if flags.client_test {
        BootMode::Scene("res://src/client_tests/client_test_scene.tscn")
    } else if flags.host_driven {
        BootMode::Scene(ENGINE_IDLE)
    } else if flags.scene_test {
        BootMode::SceneTest
    } else {
        BootMode::Emulator
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
        let global = DclGlobal::singleton();
        if !global.bind().get_engine().bind().is_host_driven() {
            DclGlobal::set_orientation_portrait();
        }
        // #2386: apply the UI content-scale synchronously, before the first frame is presented,
        // so the splash overlay and all UI render at their final scale from frame 0.
        if let Some(window) = self.base().get_window() {
            graphics::apply_ui_zoom(window);
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

        if let Some(mut tree) = self.base().get_tree_or_null() {
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

        if global.bind().is_mobile {
            InputMap::singleton().action_erase_events("ia_pointer");
        }
        self.base_mut().call_deferred("start_mode", &[]);
    }

    fn apply_display_settings(&self, global: &Gd<DclGlobal>) {
        let Some(mut window) = self
            .base()
            .get_tree_or_null()
            .and_then(|tree| tree.get_root())
        else {
            return;
        };
        graphics::apply_window_config();
        graphics::apply_low_processor_mode();
        window.move_to_center();
        graphics::connect_global_signal(&window);
        graphics::apply_ui_zoom(window.clone());
        let scale = global
            .bind()
            .get_engine()
            .bind()
            .setting("resolution_3d_scale".into());
        window.set("scaling_3d_scale", &scale);
        audio_settings::apply_volume_settings();
        general_settings::apply_max_cache_size();
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
                avatar_renderer: cli.avatar_renderer_mode,
                asset_renderer: cli.asset_renderer_mode,
                client_test: cli.client_test_mode,
                host_driven: global.bind().get_engine().bind().is_host_driven(),
                scene_test: cli.scene_test_mode,
                preview: cli.preview_mode,
                has_realm: !cli.realm.is_empty(),
            }
        };

        match boot_mode(&flags) {
            BootMode::AssetServer => {
                godot_print!("Running in Asset Server mode");
                self.start_asset_server(&global);
            }
            BootMode::Scene(path) => {
                godot_print!("Running {path}");
                self.change_scene(path);
            }
            BootMode::SceneTest => {
                godot_print!("Running in Scene Test mode");
                Self::start_throwaway_guest(&global);
                self.change_scene(EXPLORER);
            }
            BootMode::Emulator => {
                godot_print!("Running in Emulator mode");
                Self::complete_emulator_arguments(&global);
                Self::start_throwaway_guest(&global);
                self.change_scene(EXPLORER);
            }
        }
    }

    fn change_scene(&mut self, path: &str) {
        if let Some(mut tree) = self.base().get_tree_or_null() {
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

    /// Creates a disposable guest; nothing of it outlives the run.
    fn start_throwaway_guest(global: &Gd<DclGlobal>) {
        let mut identity = global.bind().get_player_identity();
        identity.bind_mut().set_default_profile();
        identity.call("create_disposable_account", &[]);
    }

    fn start_asset_server(&mut self, global: &Gd<DclGlobal>) {
        let class = StringName::from("DclAssetServer");
        if !ClassDb::singleton().class_exists(&class) {
            godot_error!(
                "Asset server requires the 'asset_server' feature to be enabled during build."
            );
            godot_error!("Build with: cargo run -- build --features asset_server");
            if let Some(mut tree) = self.base().get_tree_or_null() {
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
        if let Some(mut root) = self
            .base()
            .get_tree_or_null()
            .and_then(|tree| tree.get_root())
        {
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
    fn scene_tests_have_their_own_boot() {
        assert_eq!(
            boot_mode(&flags(|f| f.scene_test = true)),
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
    }
}
