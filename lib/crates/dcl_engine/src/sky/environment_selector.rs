//! Chooses the sky for the skybox quality and applies the graphics settings that live on the
//! environment: shadows, bloom, fog (the view distance) and anti-aliasing, which is here because it
//! applies to the viewport the environment is rendered in.

use std::cell::Cell;

use godot::classes::environment::{BgMode, FogMode};
use godot::classes::rendering_server::{ShadowQuality, ViewportMsaa};
use godot::classes::{
    Environment, INode, PackedScene, ProjectSettings, RenderingServer, WorldEnvironment,
};
use godot::prelude::*;

use super::sky_base::SkyBase;
use crate::fetcher::globals;

const SKY_LOW: &str = "res://assets/environment/sky_low/sky_low.tscn";
const SKY_MEDIUM: &str = "res://assets/environment/sky_medium/sky_medium.tscn";
const SKY_HIGH: &str = "res://assets/environment/sky_high/sky_high.tscn";

thread_local! {
    /// The engine's directional soft shadow filter, which starts at the project setting. It
    /// outlives the node, so a re-entered explorer does not clear the pipelines again.
    static SHADOW_FILTER: Cell<ShadowQuality> = Cell::new(project_shadow_filter());
}

fn project_shadow_filter() -> ShadowQuality {
    let setting = ProjectSettings::singleton().get_setting_with_override(
        "rendering/lights_and_shadows/directional_shadow/soft_shadow_filter_quality",
    );
    ShadowQuality::try_from_ord(setting.try_to::<i32>().unwrap_or(0)).unwrap_or(ShadowQuality::HARD)
}

/// The scene of a skybox quality (0 low, 1 medium, 2 high); testing always uses the high one.
pub fn sky_scene(skybox_index: i32, testing: bool) -> Option<&'static str> {
    if testing {
        return Some(SKY_HIGH);
    }
    match skybox_index {
        0 => Some(SKY_LOW),
        1 => Some(SKY_MEDIUM),
        2 => Some(SKY_HIGH),
        _ => None,
    }
}

/// How the directional shadow is set up for a shadow quality.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ShadowPlan {
    pub enabled: bool,
    /// Meters; none leaves the light's own.
    pub max_distance: Option<f32>,
    pub atlas_size: i32,
    pub atlas_16_bits: bool,
    pub soft: bool,
}

/// Quality 0 is off. 1 is tuned for Mali's tile bandwidth, which scales with atlas size times bit
/// depth: 1024 at 16 bits is about an eighth of 2048 at 32. 2 is the full distance with soft edges.
pub fn shadow_plan(quality: i32) -> ShadowPlan {
    let default = ShadowPlan {
        enabled: false,
        max_distance: None,
        atlas_size: 2048,
        atlas_16_bits: false,
        soft: false,
    };
    match quality {
        1 => ShadowPlan {
            enabled: true,
            max_distance: Some(30.0),
            atlas_size: 1024,
            atlas_16_bits: true,
            soft: false,
        },
        2 => ShadowPlan {
            enabled: true,
            max_distance: Some(50.0),
            soft: true,
            ..default
        },
        _ => default,
    }
}

/// Glow settings of a bloom quality: `(intensity, level 1, level 2)`; none turns bloom off. Levels
/// 3 and 5 are always 0.
pub fn bloom_plan(quality: i32) -> Option<(f32, f32, f32)> {
    match quality {
        1 => Some((0.7, 0.2, 0.15)),
        2 => Some((1.5, 0.4, 0.3)),
        _ => None,
    }
}

/// The anti-aliasing setting as a MSAA mode: off, x2, x4, x8.
pub fn msaa_mode(setting: i32) -> ViewportMsaa {
    match setting {
        1 => ViewportMsaa::MSAA_2X,
        2 => ViewportMsaa::MSAA_4X,
        3 => ViewportMsaa::MSAA_8X,
        _ => ViewportMsaa::DISABLED,
    }
}

/// Where depth fog begins and ends for a view distance.
pub fn fog_depth(far: f32) -> (f32, f32) {
    (far * 0.6, far * 0.9)
}

fn setting_i32(name: &str) -> i32 {
    globals::engine()
        .bind()
        .setting(name.into())
        .try_to::<i64>()
        .unwrap_or(0) as i32
}

#[derive(GodotClass)]
#[class(base=Node, init)]
pub struct EnvironmentSelector {
    base: Base<Node>,
    sky: Option<Gd<SkyBase>>,
}

#[godot_api]
impl INode for EnvironmentSelector {
    fn ready(&mut self) {
        self.set_skybox_and_shadow(setting_i32("skybox"));
        self.set_anti_aliasing(setting_i32("anti_aliasing"));
        self.set_bloom(setting_i32("bloom_quality"));
        self.set_view_distance(
            globals::engine()
                .bind()
                .setting("view_distance".into())
                .try_to::<f64>()
                .unwrap_or(200.0) as f32,
        );
        let me = self.to_gd();
        globals::engine()
            .signals()
            .setting_changed()
            .connect_other(&me, |this: &mut Self, name| {
                this.on_setting_changed(&name.to_string())
            });
    }
}

impl EnvironmentSelector {
    fn on_setting_changed(&mut self, name: &str) {
        match name {
            "skybox" => self.set_skybox_and_shadow(setting_i32("skybox")),
            "shadow_quality" => self.set_shadow(setting_i32("shadow_quality")),
            "bloom_quality" => self.set_bloom(setting_i32("bloom_quality")),
            "anti_aliasing" => self.set_anti_aliasing(setting_i32("anti_aliasing")),
            "view_distance" => self.set_view_distance(
                globals::engine()
                    .bind()
                    .setting("view_distance".into())
                    .try_to::<f64>()
                    .unwrap_or(200.0) as f32,
            ),
            _ => {}
        }
    }

    fn sky_environment(&self) -> Option<Gd<Environment>> {
        self.sky.as_ref()?.bind().environment()
    }

    fn set_view_distance(&mut self, far: f32) {
        // Without a sky (the kill-sky path) there is no fog to tune.
        let Some(mut environment) = self.sky_environment() else {
            return;
        };
        let (begin, end) = fog_depth(far);
        environment.set_fog_enabled(true);
        environment.set_fog_mode(FogMode::DEPTH);
        environment.set_fog_depth_begin(begin);
        environment.set_fog_depth_end(end);
        environment.set_fog_depth_curve(1.0);
        environment.set_fog_sky_affect(0.0);
    }

    fn set_skybox_and_shadow(&mut self, skybox_index: i32) {
        if let Some(mut sky) = self.sky.take() {
            sky.queue_free();
        }
        // `kill-sky` (or index -1) skips every sky scene and clears to a flat color, to measure the
        // rendering floor without the expensive sky shader.
        if skybox_index == -1 || globals::cli().bind().kill_sky {
            self.use_flat_environment();
            return;
        }
        let testing = crate::godot_classes::dcl_global::DclGlobal::singleton()
            .bind()
            .testing_scene_mode;
        let Some(path) = sky_scene(skybox_index, testing) else {
            return;
        };
        let Some(sky) = godot::tools::load::<PackedScene>(path)
            .instantiate()
            .and_then(|n| n.try_cast::<SkyBase>().ok())
        else {
            godot_error!("EnvironmentSelector: {path} is not a SkyBase");
            return;
        };
        self.base_mut().add_child(&sky);
        self.sky = Some(sky);
        self.set_shadow(setting_i32("shadow_quality"));
        // The new sky has a new environment: fog and view distance again.
        let far = globals::engine()
            .bind()
            .setting("view_distance".into())
            .try_to::<f64>()
            .unwrap_or(200.0) as f32;
        self.set_view_distance(far);
    }

    fn use_flat_environment(&mut self) {
        let mut environment = Environment::new_gd();
        environment.set_background(BgMode::COLOR);
        environment.set_bg_color(Color::BLACK);
        environment.set_ambient_source(godot::classes::environment::AmbientSource::COLOR);
        environment.set_ambient_light_color(Color::from_rgb(0.5, 0.5, 0.5));
        environment.set_ambient_light_energy(1.0);
        let mut world = WorldEnvironment::new_alloc();
        world.set_environment(&environment);
        self.base_mut().add_child(&world);
        self.stomp_world_environments();
    }

    /// Flattens every other WorldEnvironment in the tree that still has a sky background: the
    /// default environment and any scene's own would otherwise bring the sky back.
    fn stomp_world_environments(&self) {
        let Some(root) = self.base().get_tree_or_null().and_then(|t| t.get_root()) else {
            return;
        };
        let mut stack: Vec<Gd<Node>> = vec![root.upcast()];
        while let Some(node) = stack.pop() {
            if let Ok(world) = node.clone().try_cast::<WorldEnvironment>() {
                if let Some(mut environment) = world.get_environment() {
                    environment.set_background(BgMode::COLOR);
                    environment.set_bg_color(Color::BLACK);
                    environment.set_bg_energy_multiplier(0.0);
                }
            }
            stack.extend(node.get_children().iter_shared());
        }
    }

    fn set_shadow(&mut self, quality: i32) {
        let Some(sky) = self.sky.clone() else {
            return;
        };
        let plan = shadow_plan(quality);
        let mut main = sky.bind().main_light();
        main.set_shadow(plan.enabled);
        if let Some(distance) = plan.max_distance {
            main.set_param(
                godot::classes::light_3d::Param::SHADOW_MAX_DISTANCE,
                distance,
            );
        }
        // The moon is the night fill: shadows wherever there are shadows at all, since the sun is
        // hidden at night and only one shadow-casting directional light is active (briefly two at
        // dusk and dawn).
        if let Some(mut moon) = sky.bind().moon_light() {
            moon.set_shadow(quality >= 1);
            moon.set_param(
                godot::classes::light_3d::Param::SHADOW_MAX_DISTANCE,
                main.get_param(godot::classes::light_3d::Param::SHADOW_MAX_DISTANCE),
            );
        }
        let mut rendering = RenderingServer::singleton();
        // The engine drops every compiled scene pipeline on this call even when the quality is
        // unchanged, so every mesh in view goes undrawn until they recompile.
        let filter = if plan.soft {
            ShadowQuality::SOFT_MEDIUM
        } else {
            ShadowQuality::HARD
        };
        if SHADOW_FILTER.get() != filter {
            SHADOW_FILTER.set(filter);
            rendering.directional_soft_shadow_filter_set_quality(filter);
        }
        rendering.directional_shadow_atlas_set_size(plan.atlas_size, plan.atlas_16_bits);
    }

    fn set_bloom(&mut self, quality: i32) {
        let Some(mut environment) = self.sky_environment() else {
            return;
        };
        match bloom_plan(quality) {
            None => environment.set_glow_enabled(false),
            Some((intensity, level1, level2)) => {
                environment.set_glow_enabled(true);
                environment.set_glow_intensity(intensity);
                environment.set_glow_hdr_bleed_threshold(1.0);
                environment.set_glow_hdr_bleed_scale(2.0);
                environment.set("glow_levels/1", &level1.to_variant());
                environment.set("glow_levels/2", &level2.to_variant());
                environment.set("glow_levels/3", &0.0_f32.to_variant());
                environment.set("glow_levels/5", &0.0_f32.to_variant());
            }
        }
    }

    fn set_anti_aliasing(&mut self, setting: i32) {
        if let Some(viewport) = self.base().get_viewport() {
            RenderingServer::singleton()
                .viewport_set_msaa_3d(viewport.get_viewport_rid(), msaa_mode(setting));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_skybox_quality_picks_a_scene() {
        assert_eq!(sky_scene(0, false), Some(SKY_LOW));
        assert_eq!(sky_scene(1, false), Some(SKY_MEDIUM));
        assert_eq!(sky_scene(2, false), Some(SKY_HIGH));
        assert_eq!(sky_scene(7, false), None);
        // Testing always renders the high sky.
        assert_eq!(sky_scene(0, true), Some(SKY_HIGH));
    }

    #[test]
    fn shadow_quality_trades_distance_and_atlas_for_bandwidth() {
        assert!(!shadow_plan(0).enabled);
        let low = shadow_plan(1);
        let high = shadow_plan(2);
        assert!(low.enabled && high.enabled);
        assert!(low.max_distance < high.max_distance);
        assert!(low.atlas_size < high.atlas_size);
        assert!(low.atlas_16_bits && !high.atlas_16_bits);
        assert!(!low.soft && high.soft);
    }

    #[test]
    fn bloom_is_off_or_two_tiers() {
        assert_eq!(bloom_plan(0), None);
        assert_eq!(bloom_plan(1), Some((0.7, 0.2, 0.15)));
        assert!(bloom_plan(2).unwrap().0 > bloom_plan(1).unwrap().0);
    }

    #[test]
    fn anti_aliasing_maps_to_msaa() {
        assert_eq!(msaa_mode(0), ViewportMsaa::DISABLED);
        assert_eq!(msaa_mode(2), ViewportMsaa::MSAA_4X);
        assert_eq!(msaa_mode(9), ViewportMsaa::DISABLED);
    }

    #[test]
    fn fog_runs_from_sixty_to_ninety_percent_of_the_view_distance() {
        let (begin, end) = fog_depth(200.0);
        assert!((begin - 120.0).abs() < 1e-3 && (end - 180.0).abs() < 1e-3);
    }
}
