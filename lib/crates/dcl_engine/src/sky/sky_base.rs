//! One sky: the world environment plus the sun and moon lights, driven every frame by the sky
//! clock. A baked 144-keyframe animation turns the key light; the rest is shader parameters,
//! gradients and curves sampled at the time of day.

use godot::classes::{
    AnimationPlayer, Curve, DirectionalLight3D, Environment, Gradient, INode, RenderingServer,
    WorldEnvironment,
};
use godot::prelude::*;

use super::cycle_math;

#[derive(GodotClass)]
#[class(base=Node, init)]
pub struct SkyBase {
    base: Base<Node>,
    /// Time-of-day colors of the key light, the ambient light and the fog.
    #[export]
    directional_light_gradient: Option<Gd<Gradient>>,
    #[export]
    ambient_light_gradient: Option<Gd<Gradient>>,
    #[export]
    fog_color_gradient: Option<Gd<Gradient>>,
    /// Sun and moon rendering in the sky shader.
    #[export]
    sun_opacity_curve: Option<Gd<Curve>>,
    #[export]
    sun_size_curve: Option<Gd<Curve>>,
    #[export]
    moon_mask_size_curve: Option<Gd<Curve>>,
    /// When above 0, replaces the clock with a fast cycle of this many seconds per day, to check the
    /// transitions without waiting a day. 0 in production.
    #[export]
    debug_cycle_seconds: f32,
    /// The moon is the night fill: the baked animation doubles as its trajectory on the night arc,
    /// so the moon light copies the key light's rotation and ramps with the moon factor.
    #[export]
    #[init(val = Color::from_rgb(0.55, 0.65, 0.95))]
    moon_light_color: Color,
    #[export]
    #[init(val = 0.9)]
    moon_light_energy: f32,
    /// The least sun elevation in degrees by day. Below it the key light grazes the horizon and
    /// avatars and objects lose their ground shadow; only the light is clamped, the visual sun in
    /// the sky shader keeps following the animation.
    #[export]
    #[init(val = 12.0)]
    min_sun_elevation_degrees: f32,
    moon_smooth_dir: Vector3,
    initial_sun_energy: f32,
    #[init(node = "WorldEnvironment")]
    world_environment: OnReady<Gd<WorldEnvironment>>,
    #[init(node = "SkyLights/MainLight")]
    main_light: OnReady<Gd<DirectionalLight3D>>,
    moon_light: Option<Gd<DirectionalLight3D>>,
    #[init(node = "SkyLights/AnimationPlayer")]
    anim_player: OnReady<Gd<AnimationPlayer>>,
}

#[godot_api]
impl INode for SkyBase {
    fn ready(&mut self) {
        self.moon_smooth_dir = Vector3::UP;
        self.initial_sun_energy = self
            .main_light
            .get_param(godot::classes::light_3d::Param::ENERGY);
        self.moon_light = self
            .base()
            .try_get_node_as::<DirectionalLight3D>("SkyLights/MoonLight");
        // Play, pause, and sample by seeking.
        self.anim_player.play_ex().name("light_cycle").done();
        self.anim_player.pause();
    }

    fn process(&mut self, delta: f64) {
        let time = self.current_time();
        self.drive(time, delta as f32);
    }
}

#[godot_api]
impl SkyBase {
    /// The environment of this sky, for the settings that tune it.
    pub fn environment(&self) -> Option<Gd<Environment>> {
        self.world_environment.get_environment()
    }

    pub fn main_light(&self) -> Gd<DirectionalLight3D> {
        self.main_light.clone()
    }

    pub fn moon_light(&self) -> Option<Gd<DirectionalLight3D>> {
        self.moon_light.clone()
    }
}

impl SkyBase {
    fn current_time(&self) -> f32 {
        if self.debug_cycle_seconds > 0.0 {
            let ms = godot::classes::Time::singleton().get_ticks_msec() as f32;
            return (ms / (self.debug_cycle_seconds * 1000.0)) % 1.0;
        }
        crate::fetcher::globals::skybox_time()
            .map(|t| t.bind().get_normalized_time())
            .unwrap_or(0.0)
    }

    fn drive(&mut self, time: f32, delta: f32) {
        use godot::classes::light_3d::Param;

        // The imported sun rotation animation at this time.
        self.anim_player.seek_ex(time as f64).update(true).done();
        let mut rendering = RenderingServer::singleton();
        // The shader derives the cloud color, the sun and moon color and the floor gradients, and
        // the baked cubemap layer and blend, from this one global.
        rendering.global_shader_parameter_set("day_night_cycle", &time.to_variant());

        let moon_factor = cycle_math::moon_factor(time);
        // The dusk whip must stay dark: no light may follow that sweep.
        let lights_out = cycle_math::is_lights_out(time);

        // The keyframes came from Unity without its handedness correction (180 degrees around Y);
        // only the sky shader's disc was compensated. Correct the lights too, so shadows fall away
        // from the visible sun and moon instead of toward them (#2516).
        let animated = self.main_light.get_global_transform().basis;
        let light_basis = animated.rotated(Vector3::UP, std::f32::consts::PI);
        let light_dir = -light_basis.col_c();
        let elevation = -light_dir.y;
        let energy_factor = cycle_math::sun_energy_factor(elevation);

        let mut transform = self.main_light.get_global_transform();
        transform.basis = light_basis;
        self.main_light.set_global_transform(transform);

        // By day only, keep the key light off the horizon so it always casts a readable shadow.
        if moon_factor < 1.0 && elevation > 0.0 {
            let clamped = cycle_math::clamp_direction_elevation(
                -light_dir,
                self.min_sun_elevation_degrees.to_radians(),
            );
            if clamped != -light_dir {
                let mut transform = self.main_light.get_global_transform();
                transform.basis = Basis::looking_at(-clamped);
                self.main_light.set_global_transform(transform);
            }
        }

        // The sun hands over to the moon at night; the gate keeps it off while the animation is on
        // the night arc, or it would flash warm light from the moonset position at 4 am.
        let sun_factor = energy_factor * (1.0 - moon_factor) * cycle_math::sun_gate(time);
        self.main_light
            .set_visible(sun_factor > 0.01 && !lights_out);
        self.main_light
            .set_param(Param::ENERGY, self.initial_sun_energy * sun_factor);

        // The moon: a cool fill on the corrected night arc, so night scenes keep a directional
        // contribution. Its energy is time-driven: the fade-in starts once the arc is stable, the
        // fade-out ends at the horizon crossing, so it never cuts abruptly or uplights from below.
        if let Some(moon) = self.moon_light.as_mut() {
            moon.set_visible(moon_factor > 0.01 && !lights_out);
            if moon.is_visible() {
                let mut transform = moon.get_global_transform();
                transform.basis = light_basis;
                moon.set_global_transform(transform);
                moon.set_param(Param::ENERGY, self.moon_light_energy * moon_factor);
                moon.set_color(self.moon_light_color);
            }
        }

        // The visual sun is the corrected light direction, exactly where the shader draws the disc.
        let sun_dir = light_basis.col_c();
        rendering.global_shader_parameter_set("sun_direction", &sun_dir.to_variant());
        // The moon is opposite the sun: mostly below the horizon, above it only around dusk and
        // dawn, with its visibility gated by the mask curve. The slerp filters the dusk jumps.
        self.moon_smooth_dir = self
            .moon_smooth_dir
            .slerp(-sun_dir, (delta * 3.0).clamp(0.0, 1.0));
        rendering.global_shader_parameter_set("moon_direction", &self.moon_smooth_dir.to_variant());

        if let Some(curve) = &self.sun_opacity_curve {
            rendering.global_shader_parameter_set("sun_opacity", &curve.sample(time).to_variant());
        }
        if let Some(curve) = &self.sun_size_curve {
            rendering.global_shader_parameter_set("sun_size", &curve.sample(time).to_variant());
        }
        if let Some(curve) = &self.moon_mask_size_curve {
            rendering
                .global_shader_parameter_set("moon_mask_size", &curve.sample(time).to_variant());
        }

        // The gradients drive the light, ambient and fog colors, assigned directly each frame.
        if let Some(gradient) = self.directional_light_gradient.as_mut() {
            let sun_color = gradient.sample(time);
            self.main_light.set_color(sun_color);
            // The effective key color (sun by day, moon by night) for unshaded materials such as
            // the octahedral impostor, to match the cycle.
            let key = sun_color.lerp(self.moon_light_color, moon_factor as f64);
            rendering.global_shader_parameter_set(
                "sun_color",
                &Vector3::new(key.r, key.g, key.b).to_variant(),
            );
        }
        let Some(mut environment) = self.world_environment.get_environment() else {
            return;
        };
        if let Some(gradient) = self.ambient_light_gradient.as_mut() {
            let ambient = gradient.sample(time);
            environment.set_ambient_light_color(ambient);
            rendering.global_shader_parameter_set(
                "ambient_color",
                &Vector3::new(ambient.r, ambient.g, ambient.b).to_variant(),
            );
        }
        if let Some(gradient) = self.fog_color_gradient.as_mut() {
            environment.set_fog_light_color(gradient.sample(time));
        }
    }
}
