//! Graphics settings: the four quality profiles, the FPS limit and the UI scale. The numbers are
//! plain data and pure functions (tested); the `apply_*` functions are the engine calls.

use godot::classes::display_server::VSyncMode;
use godot::classes::{DisplayServer, Engine, Os, ProjectSettings, RefCounted, Window};
use godot::prelude::*;

use crate::fetcher::globals;
use crate::godot_classes::dcl_global::DclGlobal;
use crate::lights::component::DclLightSourceComponent;
use crate::ui::host;

/// The Custom profile (manual settings): applying it changes nothing.
pub const PROFILE_CUSTOM: i32 = 4;
/// English on purpose: these feed logs and the debug overlay.
pub const PROFILE_NAMES: [&str; 5] = ["Very Low", "Low", "Medium", "High", "Custom"];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FpsLimitMode {
    Vsync = 0,
    NoLimit = 1,
    Fps18 = 2,
    Fps30 = 3,
    Fps60 = 4,
    Fps120 = 5,
}

/// Everything a profile sets. `mesh_lod_threshold` is the screen-space error in pixels before a
/// lower LOD takes over: raising it makes the glTF import pipeline's LOD chain do real work. The
/// light fields cap scene-authored dynamic lights, which cost per-fragment on mobile.
#[derive(Clone, Copy, Debug)]
pub struct Profile {
    pub aa: i32,
    pub shadow: i32,
    pub bloom: i32,
    pub skybox: i32,
    pub texture: i32,
    pub fps: FpsLimitMode,
    pub scale: f32,
    pub mesh_lod_threshold: f32,
    pub view_distance: f32,
    pub particle_quality: i32,
    pub dcl_lights: bool,
    pub dcl_light_shadows: bool,
    pub dcl_max_lights: i32,
    pub dcl_light_range_cap: f32,
}

/// Very Low, Low, Medium, High.
///
/// Medium has AA off: on a Mali-G68 it cost about 8 ms of GPU in Genesis Plaza for no visible
/// gain at typical mobile resolution. High keeps shadows at Low for the same reason: quality 2
/// cost about 10 ms there (62% of the geometry goes through the shadow pass).
pub const PROFILES: [Profile; 4] = [
    Profile {
        aa: 0,
        shadow: 0,
        bloom: 0,
        skybox: 0,
        texture: 0,
        fps: FpsLimitMode::Fps18,
        scale: 0.5,
        mesh_lod_threshold: 8.0,
        view_distance: 100.0,
        particle_quality: 0,
        dcl_lights: false,
        dcl_light_shadows: false,
        dcl_max_lights: 0,
        dcl_light_range_cap: 15.0,
    },
    Profile {
        aa: 0,
        shadow: 0,
        bloom: 0,
        skybox: 0,
        texture: 0,
        fps: FpsLimitMode::Fps30,
        scale: 0.75,
        mesh_lod_threshold: 6.0,
        view_distance: 150.0,
        particle_quality: 1,
        dcl_lights: true,
        dcl_light_shadows: false,
        dcl_max_lights: 2,
        dcl_light_range_cap: 20.0,
    },
    Profile {
        aa: 0,
        shadow: 1,
        bloom: 1,
        skybox: 1,
        texture: 1,
        fps: FpsLimitMode::Fps30,
        scale: 1.0,
        mesh_lod_threshold: 3.0,
        view_distance: 200.0,
        particle_quality: 2,
        dcl_lights: true,
        dcl_light_shadows: false,
        dcl_max_lights: 4,
        dcl_light_range_cap: 30.0,
    },
    Profile {
        aa: 1,
        shadow: 1,
        bloom: 2,
        skybox: 2,
        texture: 2,
        fps: FpsLimitMode::Fps60,
        scale: 1.0,
        mesh_lod_threshold: 2.0,
        view_distance: 300.0,
        particle_quality: 3,
        dcl_lights: true,
        dcl_light_shadows: true,
        dcl_max_lights: 8,
        dcl_light_range_cap: 40.0,
    },
];

/// The profile at `index`, none for Custom or an invalid index.
pub fn profile(index: i32) -> Option<&'static Profile> {
    usize::try_from(index).ok().and_then(|i| PROFILES.get(i))
}

/// The FPS a limit mode asks for; 0 is unlimited.
pub fn fps_limit_mode_to_fps(mode: i32) -> i32 {
    match mode {
        m if m == FpsLimitMode::Vsync as i32 || m == FpsLimitMode::NoLimit as i32 => 0,
        m if m == FpsLimitMode::Fps18 as i32 => 18,
        m if m == FpsLimitMode::Fps30 as i32 => 30,
        m if m == FpsLimitMode::Fps60 as i32 => 60,
        _ => 30,
    }
}

/// What the engine is set to for a limit mode and a thermal cap.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FpsPlan {
    pub max_fps: i32,
    pub vsync: bool,
    pub physics_ticks: i32,
}

/// The more restrictive of the user's limit and the thermal cap applies; VSync mode only holds
/// while nothing caps it.
pub fn plan_fps_limit(mode: i32, thermal_cap: i32) -> FpsPlan {
    let user = fps_limit_mode_to_fps(mode);
    let effective = match (user, thermal_cap) {
        (u, cap) if cap <= 0 => u,
        (0, cap) => cap,
        (u, cap) => u.min(cap),
    };
    if mode == FpsLimitMode::Vsync as i32 && thermal_cap == 0 {
        FpsPlan {
            max_fps: 0,
            vsync: true,
            physics_ticks: 60,
        }
    } else if effective == 0 {
        FpsPlan {
            max_fps: 0,
            vsync: false,
            physics_ticks: 60,
        }
    } else {
        FpsPlan {
            max_fps: effective,
            vsync: false,
            physics_ticks: effective.min(60),
        }
    }
}

/// `(scene budget, per-emitter cap, CPU emitters)` for a particle quality. Zero turns scene
/// particles off; the low tiers use CPU particles, cheaper on mobile GPUs (no bursts there).
pub fn particle_budgets(quality: i32) -> (i32, i32, bool) {
    match quality {
        0 => (0, 0, true),
        1 => (2_000, 500, true),
        2 => (15_000, 2_000, false),
        _ => (50_000, 5_000, false),
    }
}

/// The design resolution from the project settings, its axes swapped when the screen's
/// orientation differs (1600x720 landscape is 720x1600 portrait).
pub fn base_resolution(project: Vector2, screen: Vector2) -> Vector2 {
    let project_portrait = project.y > project.x;
    let screen_portrait = screen.y > screen.x;
    if project_portrait != screen_portrait {
        Vector2::new(project.y, project.x)
    } else {
        project
    }
}

/// The UI content scale: the smaller axis ratio, so content always fits.
pub fn ui_scale(screen: Vector2, base: Vector2) -> f32 {
    (screen.x / base.x).min(screen.y / base.y)
}

fn project_resolution() -> Vector2 {
    let settings = ProjectSettings::singleton();
    let get = |key: &str, default: f32| {
        settings
            .get_setting_ex(key)
            .default_value(&default.to_variant())
            .done()
            .try_to::<f32>()
            .unwrap_or(default)
    };
    Vector2::new(
        get("display/window/size/viewport_width", 1600.0),
        get("display/window/size/viewport_height", 720.0),
    )
}

fn setting(name: &str) -> Variant {
    globals::engine().bind().setting(name.into())
}

fn root_window() -> Option<Gd<Window>> {
    DclGlobal::singleton().get_tree_or_null()?.get_root()
}

/// Scales the UI from the window size. On a real phone the OS can report swapped dimensions for a
/// moment around background/foreground, and applying that scale would lay the UI out for the wrong
/// orientation: a size that contradicts the tracked orientation is skipped.
pub fn apply_ui_zoom(mut root: Gd<Window>) {
    let size = Vector2::new(root.get_size().x as f32, root.get_size().y as f32);
    if size.x <= 0.0 || size.y <= 0.0 {
        return;
    }
    if host::is_mobile() && !DclGlobal::singleton().bind().is_virtual_mobile {
        let says_portrait = size.y > size.x;
        if says_portrait != host::is_portrait() {
            return;
        }
    }
    let base = base_resolution(project_resolution(), size);
    root.set_content_scale_factor(ui_scale(size, base));
}

/// The design base resolution for a screen, for the debug overlay.
pub fn get_base_resolution(screen: Vector2) -> Vector2 {
    base_resolution(project_resolution(), screen)
}

/// Keeps the UI scale in step with the window and the orientation.
pub fn connect_global_signal(root: &Gd<Window>) {
    let handle = root.clone();
    root.signals()
        .size_changed()
        .connect(move || apply_ui_zoom(handle.clone()));
    // Orientation flips swap the base axes: recompute once the window finished resizing (mobile
    // rotates asynchronously).
    let handle = root.clone();
    host::connect_fn("orientation_changed", move |_| {
        let mut handle = handle.clone();
        handle.call_deferred("emit_signal", &["size_changed".to_variant()]);
    });
}

pub fn apply_window_config() {
    if host::is_mobile() {
        return;
    }
    use godot::classes::display_server::WindowMode;
    let mode = match setting("window_mode").try_to::<i64>().unwrap_or(0) {
        0 => WindowMode::WINDOWED,
        1 => WindowMode::FULLSCREEN,
        _ => WindowMode::EXCLUSIVE_FULLSCREEN,
    };
    DisplayServer::singleton().window_set_mode(mode);
}

pub fn apply_fps_limit() {
    let mode = setting("limit_fps").try_to::<i64>().unwrap_or(0) as i32;
    apply_fps_limit_with_thermal_cap(mode, 0);
}

pub fn apply_fps_limit_with_thermal_cap(mode: i32, thermal_cap: i32) {
    let plan = plan_fps_limit(mode, thermal_cap);
    let mut engine = Engine::singleton();
    engine.set_max_fps(plan.max_fps);
    DisplayServer::singleton().window_set_vsync_mode(if plan.vsync {
        VSyncMode::ENABLED
    } else {
        VSyncMode::DISABLED
    });
    engine.set_physics_ticks_per_second(plan.physics_ticks);
}

/// For the lobby and menus: save CPU, no 3D.
pub fn apply_low_processor_mode() {
    Os::singleton().set_low_processor_usage_mode(true);
    DisplayServer::singleton().window_set_vsync_mode(VSyncMode::ENABLED);
    let mut engine = Engine::singleton();
    engine.set_max_fps(0);
    engine.set_physics_ticks_per_second(60);
    if let Some(mut viewport) = root_window().and_then(|w| w.get_viewport()) {
        viewport.set_disable_3d(true);
    }
}

/// For the world: full performance, the player's FPS limit.
pub fn apply_full_processor_mode() {
    Os::singleton().set_low_processor_usage_mode(false);
    apply_fps_limit();
    if let Some(mut viewport) = root_window().and_then(|w| w.get_viewport()) {
        viewport.set_disable_3d(false);
    }
}

pub fn apply_particle_quality(quality: i32) {
    let (budget, per_emitter, cpu) = particle_budgets(quality);
    DclGlobal::set_particle_profile_budgets(budget, per_emitter, cpu);
}

/// Applies a profile by index: every graphics setting, the FPS limit, view distance, the 3D scale,
/// the LOD threshold, particles and the dynamic-light budget. Custom or an invalid index does
/// nothing. The values are the engine's own choice: stored and announced, not applied again.
pub fn apply_graphic_profile(index: i32) {
    let Some(p) = profile(index) else {
        return;
    };
    let engine = globals::engine();
    let adopt = |name: &str, value: Variant| engine.bind().adopt_setting(name.into(), value);
    adopt("anti_aliasing", p.aa.to_variant());
    adopt("shadow_quality", p.shadow.to_variant());
    adopt("bloom_quality", p.bloom.to_variant());
    adopt("skybox", p.skybox.to_variant());
    adopt("texture_quality", p.texture.to_variant());
    adopt("limit_fps", (p.fps as i32).to_variant());
    adopt("resolution_3d_scale", p.scale.to_variant());
    adopt("view_distance", p.view_distance.to_variant());
    adopt("particle_quality", p.particle_quality.to_variant());
    adopt("graphic_profile", index.to_variant());

    apply_fps_limit();
    if let Ok(mut camera) = DclGlobal::singleton()
        .get("player_camera_node")
        .try_to::<Gd<godot::classes::Camera3D>>()
    {
        camera.set_far(p.view_distance);
    }
    if let Some(mut viewport) = root_window().and_then(|w| w.get_viewport()) {
        viewport.set_scaling_3d_scale(p.scale);
        viewport.set_mesh_lod_threshold(p.mesh_lod_threshold);
    }
    crate::avatars::avatar::lod::apply_particles_enabled(p.particle_quality > 0);
    apply_particle_quality(p.particle_quality);
    DclLightSourceComponent::apply_graphic_profile_settings(
        p.dcl_lights,
        p.dcl_light_shadows,
        p.dcl_max_lights as i64,
        p.dcl_light_range_cap,
    );
}

/// The graphics functions GDScript still calls, until the callers are Rust too.
#[derive(GodotClass)]
#[class(base=RefCounted, init)]
pub struct GraphicSettings {
    base: Base<RefCounted>,
}

#[godot_api]
impl GraphicSettings {
    /// The FPS limit mode that lifts the cap.
    #[constant]
    const NO_LIMIT: i32 = FpsLimitMode::NoLimit as i32;

    #[func(rename = apply_graphic_profile)]
    fn gd_apply_graphic_profile(index: i32) {
        apply_graphic_profile(index);
    }

    #[func(rename = apply_particle_quality)]
    fn gd_apply_particle_quality(quality: i32) {
        apply_particle_quality(quality);
    }

    #[func(rename = apply_full_processor_mode)]
    fn gd_apply_full_processor_mode() {
        apply_full_processor_mode();
    }

    #[func(rename = apply_fps_limit_with_thermal_cap)]
    fn gd_apply_fps_limit_with_thermal_cap(mode: i32, thermal_cap: i32) {
        apply_fps_limit_with_thermal_cap(mode, thermal_cap);
    }

    #[func(rename = get_base_resolution)]
    fn gd_get_base_resolution(screen: Vector2) -> Vector2 {
        get_base_resolution(screen)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_profiles_climb_in_cost() {
        for pair in PROFILES.windows(2) {
            assert!(pair[0].scale <= pair[1].scale);
            assert!(pair[0].view_distance < pair[1].view_distance);
            assert!(pair[0].dcl_max_lights <= pair[1].dcl_max_lights);
            assert!(pair[0].mesh_lod_threshold > pair[1].mesh_lod_threshold);
        }
        assert!(!PROFILES[0].dcl_lights);
    }

    #[test]
    fn custom_and_invalid_indices_have_no_profile() {
        assert!(profile(0).is_some() && profile(3).is_some());
        assert!(profile(PROFILE_CUSTOM).is_none());
        assert!(profile(-1).is_none());
        assert!(profile(99).is_none());
    }

    #[test]
    fn limit_modes_map_to_fps() {
        assert_eq!(fps_limit_mode_to_fps(FpsLimitMode::Vsync as i32), 0);
        assert_eq!(fps_limit_mode_to_fps(FpsLimitMode::NoLimit as i32), 0);
        assert_eq!(fps_limit_mode_to_fps(FpsLimitMode::Fps18 as i32), 18);
        assert_eq!(fps_limit_mode_to_fps(FpsLimitMode::Fps60 as i32), 60);
        // No dedicated mode for 120: it falls back like any unknown one.
        assert_eq!(fps_limit_mode_to_fps(FpsLimitMode::Fps120 as i32), 30);
    }

    #[test]
    fn vsync_holds_only_without_a_thermal_cap() {
        let vsync = FpsLimitMode::Vsync as i32;
        assert_eq!(
            plan_fps_limit(vsync, 0),
            FpsPlan {
                max_fps: 0,
                vsync: true,
                physics_ticks: 60
            }
        );
        let capped = plan_fps_limit(vsync, 24);
        assert_eq!(
            (capped.max_fps, capped.vsync, capped.physics_ticks),
            (24, false, 24)
        );
    }

    #[test]
    fn the_more_restrictive_limit_wins() {
        let sixty = FpsLimitMode::Fps60 as i32;
        assert_eq!(plan_fps_limit(sixty, 30).max_fps, 30);
        assert_eq!(plan_fps_limit(sixty, 0).max_fps, 60);
        let none = FpsLimitMode::NoLimit as i32;
        assert_eq!(
            plan_fps_limit(none, 0),
            FpsPlan {
                max_fps: 0,
                vsync: false,
                physics_ticks: 60
            }
        );
        assert_eq!(plan_fps_limit(none, 20).max_fps, 20);
    }

    #[test]
    fn particles_scale_with_quality() {
        assert_eq!(particle_budgets(0), (0, 0, true));
        assert!(particle_budgets(1).2);
        assert!(!particle_budgets(2).2);
        assert_eq!(particle_budgets(3), (50_000, 5_000, false));
        assert_eq!(particle_budgets(9), particle_budgets(3));
    }

    #[test]
    fn the_base_resolution_follows_the_screen_orientation() {
        let project = Vector2::new(1600.0, 720.0);
        assert_eq!(
            base_resolution(project, Vector2::new(2400.0, 1080.0)),
            project
        );
        assert_eq!(
            base_resolution(project, Vector2::new(1080.0, 2400.0)),
            Vector2::new(720.0, 1600.0)
        );
    }

    #[test]
    fn the_ui_scale_fits_the_tighter_axis() {
        let base = Vector2::new(1600.0, 720.0);
        assert_eq!(ui_scale(Vector2::new(3200.0, 1440.0), base), 2.0);
        assert_eq!(ui_scale(Vector2::new(3200.0, 720.0), base), 1.0);
    }
}

/// Run inside the engine by `cargo run -- run --itest`: every key a profile owns must land on its
/// target (issue #2672): the settings, the light budget, the avatar particle gate and the particle
/// budgets.
mod itests {
    use super::*;
    use crate::framework::TestContext;
    use crate::scene_runner::components::particle_system::get_particle_profile_budgets;

    fn engine_setting_f32(name: &str) -> f32 {
        setting(name).try_to::<f64>().unwrap_or(f64::NAN) as f32
    }

    fn engine_setting_i32(name: &str) -> i32 {
        setting(name).try_to::<i64>().unwrap_or(-1) as i32
    }

    #[godot::test::itest]
    fn a_profile_lands_on_every_target(_ctx: &TestContext) {
        for (i, p) in PROFILES.iter().enumerate() {
            let index = i as i32;
            apply_graphic_profile(index);
            assert_eq!(engine_setting_i32("graphic_profile"), index, "profile {i}");
            assert!((engine_setting_f32("view_distance") - p.view_distance).abs() < 1e-3);
            assert_eq!(engine_setting_i32("particle_quality"), p.particle_quality);
            assert_eq!(
                DclLightSourceComponent::get_max_active_lights(),
                p.dcl_max_lights as i64,
                "profile {i}: light budget"
            );
            assert_eq!(
                crate::avatars::avatar::lod::particles_enabled(),
                p.particle_quality > 0,
                "profile {i}: avatar particles gate"
            );
            let (budget, cap, cpu) = particle_budgets(p.particle_quality);
            assert_eq!(
                get_particle_profile_budgets(),
                (budget, cap, cpu),
                "profile {i}"
            );
        }
    }

    #[godot::test::itest]
    fn custom_and_invalid_profiles_change_nothing(_ctx: &TestContext) {
        apply_graphic_profile(0);
        let view = engine_setting_f32("view_distance");
        let particles = engine_setting_i32("particle_quality");
        let lights = DclLightSourceComponent::get_max_active_lights();
        for index in [PROFILE_CUSTOM, -1, PROFILES.len() as i32 + 1] {
            apply_graphic_profile(index);
            assert_eq!(engine_setting_f32("view_distance"), view, "index {index}");
            assert_eq!(engine_setting_i32("particle_quality"), particles);
            assert_eq!(DclLightSourceComponent::get_max_active_lights(), lights);
        }
    }
}
