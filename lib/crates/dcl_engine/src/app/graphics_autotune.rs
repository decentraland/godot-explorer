//! The first-launch hardware benchmark and the dynamic-graphics wiring: the profile and the FPS cap
//! follow the device's measured power and its temperature.

use godot::classes::{RefCounted, SceneTree};
use godot::prelude::*;

use super::graphics::{self, PROFILE_NAMES};
use super::hardware_benchmark::HardwareBenchmark;
use crate::fetcher::globals;
use crate::godot_classes::dcl_global::DclGlobal;
use crate::ui::host;

#[derive(GodotClass)]
#[class(base=RefCounted, init)]
pub struct GraphicsAutotune {
    base: Base<RefCounted>,
    benchmark: Option<Gd<HardwareBenchmark>>,
    /// Off while a benchmark holds the frame rate uncapped.
    #[init(val = true)]
    thermal_cap_applies: bool,
}

fn setting_i32(name: &str) -> i32 {
    globals::engine()
        .bind()
        .setting(name.into())
        .try_to::<i64>()
        .unwrap_or(0) as i32
}

fn bench_mode() -> bool {
    globals::cli().bind().bench_mode
}

#[godot_api]
impl GraphicsAutotune {
    pub fn set_thermal_cap_applies(&mut self, applies: bool) {
        self.thermal_cap_applies = applies;
    }

    /// Whether the first-launch benchmark should run (a phone, on a first launch or a dev build).
    /// Skipped in bench mode: it calls `viewport_set_measure_render_time` and could clobber the
    /// benchmark runner's own measurement state.
    #[func]
    pub fn should_run_first_launch_benchmark(&self) -> bool {
        if bench_mode() {
            return false;
        }
        let completed = globals::engine()
            .bind()
            .setting("first_launch_completed".into())
            .try_to::<bool>()
            .unwrap_or(false);
        host::is_mobile() && (!completed || DclGlobal::is_dev())
    }

    /// Runs the benchmark; called once the engine is up so the first frames are not blocked.
    #[func]
    pub fn run_first_launch_benchmark(&mut self) {
        if self.benchmark.is_some() {
            return;
        }
        godot_print!("[HardwareBenchmark] Running first launch benchmark...");
        let mut benchmark = HardwareBenchmark::new_alloc();
        let me = self.to_gd();
        benchmark.signals().benchmark_completed().connect_other(
            &me,
            |this: &mut Self, profile, gpu, ram| {
                this.on_benchmark_completed(profile as i32, gpu as f32, ram as f32)
            },
        );
        if let Some(root) = DclGlobal::singleton()
            .get_tree_or_null()
            .and_then(|tree: Gd<SceneTree>| tree.get_root())
        {
            let mut root = root;
            root.add_child(&benchmark);
        }
        benchmark.bind_mut().run_benchmark();
        self.benchmark = Some(benchmark);
    }

    /// Hooks the dynamic-graphics manager to the settings, the loading screens and the graphics
    /// applier. Skipped in bench mode: the runner pins its own profile and the manager's startup
    /// work would only add CPU noise and race the thermal-cap signal with the bench setup.
    #[func]
    pub fn init_dynamic_graphics(&mut self) {
        if bench_mode() {
            godot_print!("[DynamicGraphics] skipped (bench_mode=true)");
            return;
        }
        let global = DclGlobal::singleton();
        let mut manager = global.bind().dynamic_graphics_manager.clone();
        let enabled = globals::engine()
            .bind()
            .setting("dynamic_graphics_enabled".into())
            .try_to::<bool>()
            .unwrap_or(false);
        manager.bind_mut().initialize(
            enabled,
            setting_i32("graphic_profile"),
            setting_i32("limit_fps"),
        );
        let me = self.to_gd();
        let started = manager.clone();
        host::connect_fn("loading_started", move |_| {
            started.clone().bind_mut().on_loading_started();
        });
        let finished = manager.clone();
        host::connect_fn("loading_finished", move |_| {
            finished.clone().bind_mut().on_loading_finished();
        });
        manager
            .signals()
            .profile_change_requested()
            .connect_other(&me, |this: &mut Self, profile| {
                this.on_dynamic_profile_change(profile)
            });
        manager
            .signals()
            .thermal_fps_cap_changed()
            .connect_other(&me, |this: &mut Self, cap| {
                this.on_thermal_fps_cap_changed(cap)
            });
    }

    /// The benchmark runner disconnects this handler by name to pin its own FPS.
    #[func(rename = _on_thermal_fps_cap_changed)]
    fn gd_on_thermal_fps_cap_changed(&mut self, fps_cap: i32) {
        self.on_thermal_fps_cap_changed(fps_cap);
    }
}

impl GraphicsAutotune {
    fn on_benchmark_completed(&mut self, profile: i32, gpu_score: f32, ram_gb: f32) {
        godot_print!(
            "[HardwareBenchmark] First launch complete: profile={profile}, gpu={gpu_score:.1}ms, ram={ram_gb:.1}GB"
        );
        let engine = globals::engine();
        engine
            .bind()
            .adopt_setting("benchmark_gpu_score".into(), gpu_score.to_variant());
        engine
            .bind()
            .adopt_setting("benchmark_ram_gb".into(), ram_gb.to_variant());
        engine
            .bind()
            .adopt_setting("first_launch_completed".into(), true.to_variant());
        graphics::apply_graphic_profile(profile);
        let mut manager = DclGlobal::singleton()
            .bind()
            .dynamic_graphics_manager
            .clone();
        manager.bind_mut().on_manual_profile_change(profile);
        engine.bind().save_settings();
        if let Some(mut benchmark) = self.benchmark.take() {
            if benchmark.is_instance_valid() {
                benchmark.queue_free();
            }
        }
    }

    /// Applies the effective FPS limit: the player's setting and the thermal cap, the lower wins.
    fn on_thermal_fps_cap_changed(&mut self, fps_cap: i32) {
        if !self.thermal_cap_applies {
            return;
        }
        graphics::apply_fps_limit_with_thermal_cap(setting_i32("limit_fps"), fps_cap);
        if fps_cap == 0 {
            godot_print!("[ThermalFPS] Cap removed, using user setting");
        } else {
            godot_print!("[ThermalFPS] Cap applied: {fps_cap} FPS");
        }
    }

    fn on_dynamic_profile_change(&mut self, new_profile: i32) {
        let old_profile = setting_i32("graphic_profile");
        if old_profile == new_profile {
            return;
        }
        let name = |i: i32| {
            usize::try_from(i)
                .ok()
                .and_then(|i| PROFILE_NAMES.get(i))
                .copied()
                .unwrap_or("?")
        };
        godot_print!(
            "[DynamicGraphics] Profile changed: {} -> {}",
            name(old_profile),
            name(new_profile)
        );
        graphics::apply_graphic_profile(new_profile);
        globals::engine().bind().save_settings();
    }
}
