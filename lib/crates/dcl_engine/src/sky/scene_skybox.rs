//! A scene's sky over the time-of-day one: the overrides `scene_runner::components::skybox`
//! publishes, applied after the day/night cycle wrote the sky. Sky, cloud and sun colors ride on
//! the shader's per-time-of-day LUTs, the sun and fog on the lights and the environment, and a
//! `skybox_texture` swaps the sky for a panorama. Every value it replaces comes back exactly.

use std::cell::{Cell, RefCell};

use godot::classes::environment::FogMode;
use godot::classes::light_3d::Param;
use godot::classes::{
    DirectionalLight3D, Environment, Gradient, GradientTexture1D, Material, PanoramaSkyMaterial,
    RenderingServer, ShaderMaterial, WorldEnvironment,
};
use godot::prelude::*;

use super::environment_selector::{apply_view_distance, view_distance_setting};
use super::sky_base::SkyBase;
use crate::content::texture::TextureEntry;
use crate::engine::local_task::{await_promise, spawn_local};
use crate::fetcher::globals;
use crate::godot_classes::dcl_global::DclGlobal;
use crate::godot_classes::promise::PromiseError;
use crate::scene_runner::components::skybox::{GradientKeys, SceneSkyConfig};

const LUT_WIDTH: i32 = 256;
const EQUIRECTANGULAR_TOLERANCE: f32 = 0.02;

/// The LUTs of zenith, horizon, nadir and rim. The first three are `source_color`, so the shader
/// decodes sRGB on sample: they are written sRGB-encoded to land the proto's linear color.
const SKY_COLOR_LUTS: [(&str, bool); 4] = [
    ("tint_color_cycle_bottom", true),
    ("tint_color_cycle_top", true),
    ("tint_color_cycle_nadir", true),
    ("rim_color_cycle", false),
];
const CLOUDS_COLOR: &str = "clouds_color_cycle";
const CLOUD_OPACITY: &str = "cloud_opacity";
const CLOUDS_SPEED: &str = "clouds_scroll_speed";
const STARS_ENERGY: &str = "stars_energy";
const STARS_OVERRIDE: &str = "stars_override";
const SUN_COLOR: &str = "sun_color_cycle";

/// Every sky uniform an override writes: their startup values are what a scene restores, so each
/// needs a stored default in `assets/sky.tres` (an unset uniform reads back as nil).
const OVERRIDE_PARAMS: [&str; 10] = [
    SKY_COLOR_LUTS[0].0,
    SKY_COLOR_LUTS[1].0,
    SKY_COLOR_LUTS[2].0,
    SKY_COLOR_LUTS[3].0,
    CLOUDS_COLOR,
    CLOUD_OPACITY,
    CLOUDS_SPEED,
    STARS_ENERGY,
    STARS_OVERRIDE,
    SUN_COLOR,
];

/// The sky before any scene override.
struct Pristine {
    params: Vec<(&'static str, Variant)>,
    fog_density: f32,
    sky_material: Option<Gd<Material>>,
}

thread_local! {
    /// Captured once: a quality change rebuilds the sky while overrides are live, and the low,
    /// medium and high skies share one environment and material, so a later capture would record
    /// the scene's values as the defaults.
    static PRISTINE: RefCell<Option<Pristine>> = const { RefCell::new(None) };
    /// The sky that took the last config. Only it may restore: a stale teardown would erase the
    /// sky its successor just applied.
    static OWNER: Cell<Option<InstanceId>> = const { Cell::new(None) };
}

pub struct SceneSkybox {
    sky_id: InstanceId,
    world_environment: Gd<WorldEnvironment>,
    main_light: Gd<DirectionalLight3D>,
    moon_light: Option<Gd<DirectionalLight3D>>,
    revision: Option<u64>,
    /// Some while overrides are applied.
    config: Option<SceneSkyConfig>,
    scene_id: i32,
    texture_token: u64,
    pano_material: Option<Gd<PanoramaSkyMaterial>>,
    saved_material: Option<Gd<Material>>,
    params_material: Option<Gd<ShaderMaterial>>,
    saved_params: Vec<(&'static str, Variant)>,
    fog_overridden: bool,
    sun_basis: Option<Basis>,
}

impl SceneSkybox {
    pub fn new(
        sky_id: InstanceId,
        world_environment: Gd<WorldEnvironment>,
        main_light: Gd<DirectionalLight3D>,
        moon_light: Option<Gd<DirectionalLight3D>>,
    ) -> Self {
        if PRISTINE.with_borrow(Option::is_none) {
            if let Some(environment) = world_environment.get_environment() {
                let sky_material = environment.get_sky().and_then(|sky| sky.get_material());
                let params = sky_material
                    .clone()
                    .and_then(|material| material.try_cast::<ShaderMaterial>().ok())
                    .map(|material| {
                        OVERRIDE_PARAMS
                            .iter()
                            .map(|param| (*param, material.get_shader_parameter(*param)))
                            .collect()
                    })
                    .unwrap_or_default();
                PRISTINE.set(Some(Pristine {
                    params,
                    fog_density: environment.get_fog_density(),
                    sky_material,
                }));
            }
        }
        Self {
            sky_id,
            world_environment,
            main_light,
            moon_light,
            revision: None,
            config: None,
            scene_id: -1,
            texture_token: 0,
            pano_material: None,
            saved_material: None,
            params_material: None,
            saved_params: Vec::new(),
            fog_overridden: false,
            sun_basis: None,
        }
    }

    /// Takes a newly published config, then re-applies the per-frame overrides over what the
    /// day/night cycle just wrote.
    pub fn update(&mut self, time_of_day: f32) {
        let published = {
            let global = DclGlobal::singleton();
            let global = global.bind();
            let sky = &global.scene_sky;
            (self.revision != Some(sky.revision)).then(|| {
                (
                    sky.revision,
                    sky.config.clone(),
                    sky.scene_id.map_or(-1, |id| id.0),
                )
            })
        };
        if let Some((revision, config, scene_id)) = published {
            self.revision = Some(revision);
            self.apply_config(config, scene_id);
        }
        self.apply_frame(time_of_day);
    }

    /// Replaces every override with the ones in `config` (empty = back to the defaults).
    fn apply_config(&mut self, config: SceneSkyConfig, scene_id: i32) {
        self.restore();
        self.scene_id = scene_id;
        if config.is_empty() {
            return;
        }
        OWNER.set(Some(self.sky_id));
        self.sun_basis = config
            .sun
            .direction
            .map(|[x, y, z]| sun_basis_for(Vector3::new(x, y, z)));
        let texture = config.skybox_texture.clone();
        let density = config.fog.density;
        self.config = Some(config);

        match texture {
            None => self.apply_procedural_overrides(),
            Some(src) => self.load_panorama(src),
        }
        if let (Some(density), Some(mut environment)) =
            (density, self.world_environment.get_environment())
        {
            apply_exponential_fog(&mut environment, density);
            self.fog_overridden = true;
        }
    }

    fn apply_frame(&mut self, time_of_day: f32) {
        let Some(config) = &self.config else {
            return;
        };
        let environment = self.world_environment.get_environment();
        let mut rendering = RenderingServer::singleton();

        // The View distance setting rewrites the fog mode when it changes, which would silently
        // drop the scene density until the config changes again.
        if let (Some(density), Some(mut environment)) = (config.fog.density, environment.clone()) {
            if environment.get_fog_mode() != FogMode::EXPONENTIAL {
                apply_exponential_fog(&mut environment, density);
            }
        }

        let sun = &config.sun;
        if let Some(keys) = &sun.color {
            let color = sample(keys, time_of_day);
            self.main_light.set_color(color);
            rendering.global_shader_parameter_set(
                "sun_color",
                &Vector3::new(color.r, color.g, color.b).to_variant(),
            );
        }
        if let Some(intensity) = sun.intensity {
            let energy = self.main_light.get_param(Param::ENERGY);
            self.main_light.set_param(Param::ENERGY, energy * intensity);
        }
        if let Some(basis) = self.sun_basis {
            let mut transform = self.main_light.get_global_transform();
            transform.basis = basis;
            self.main_light.set_global_transform(transform);
            if let Some(moon) = self.moon_light.as_mut() {
                let mut transform = moon.get_global_transform();
                transform.basis = basis;
                moon.set_global_transform(transform);
            }
            rendering.global_shader_parameter_set("sun_direction", &basis.col_c().to_variant());
        }
        if sun.visible == Some(false) {
            // Hides the discs and the flare; the light itself stays on (proto contract).
            rendering.global_shader_parameter_set("sun_opacity", &0.0_f32.to_variant());
        }

        let Some(mut environment) = environment else {
            return;
        };
        // The proto says the sky colors drive the ambient light without saying which band: the
        // horizon is the broad band the eye reads as "the sky".
        if let Some(keys) = &config.sky_colors.horizon {
            let color = sample(keys, time_of_day);
            environment.set_ambient_light_color(color);
            rendering.global_shader_parameter_set(
                "ambient_color",
                &Vector3::new(color.r, color.g, color.b).to_variant(),
            );
        }
        if let Some(keys) = &config.fog.color {
            environment.set_fog_light_color(sample(keys, time_of_day));
        }
    }

    /// Drops every override and hands the sky back to the day/night cycle.
    pub fn restore(&mut self) {
        if OWNER.get().is_some_and(|owner| owner != self.sky_id) {
            return;
        }
        OWNER.set(None);
        self.texture_token += 1;

        if self.world_environment.is_instance_valid() {
            if let Some(mut material) = self.params_material.clone().or_else(|| self.material()) {
                for (param, value) in &self.saved_params {
                    material.set_shader_parameter(*param, value);
                }
            }
            if let Some(mut environment) = self.world_environment.get_environment() {
                if self.fog_overridden {
                    // The View distance setting owns depth fog and may have changed meanwhile.
                    let density = PRISTINE.with_borrow(|p| p.as_ref().map(|p| p.fog_density));
                    environment.set_fog_density(density.unwrap_or(1.0));
                    apply_view_distance(&mut environment, view_distance_setting());
                }
                if let (Some(pano), Some(mut sky)) = (&self.pano_material, environment.get_sky()) {
                    if sky.get_material() == Some(pano.clone().upcast()) {
                        sky.set_material(self.saved_material.as_ref());
                    }
                }
            }
        }

        self.saved_params.clear();
        self.params_material = None;
        self.pano_material = None;
        self.saved_material = None;
        self.fog_overridden = false;
        self.sun_basis = None;
        self.config = None;
        self.scene_id = -1;
    }

    fn apply_procedural_overrides(&mut self) {
        let Some(config) = self.config.clone() else {
            return;
        };
        let colors = &config.sky_colors;
        for (keys, (param, srgb)) in [&colors.zenith, &colors.horizon, &colors.nadir, &colors.rim]
            .into_iter()
            .zip(SKY_COLOR_LUTS)
        {
            if let Some(keys) = keys {
                self.set_param(param, &make_lut(keys, srgb).to_variant());
            }
        }

        let clouds = &config.clouds;
        if let Some(keys) = &clouds.color {
            self.set_param(CLOUDS_COLOR, &make_lut(keys, false).to_variant());
        }
        if let Some(opacity) = clouds.opacity {
            self.set_param(CLOUD_OPACITY, &opacity.to_variant());
        }
        if let Some(speed) = clouds.speed {
            // The shader advances clouds in turns per second; the proto's speed is radians.
            self.set_param(CLOUDS_SPEED, &(speed / std::f32::consts::TAU).to_variant());
        }

        if let Some(brightness) = config.stars_brightness {
            self.set_param(STARS_ENERGY, &brightness.to_variant());
            // A pinned brightness is a constant star layer at every hour, which is what the Unity
            // explorer draws for the same value: the time of day must not fade it.
            self.set_param(STARS_OVERRIDE, &1.0_f32.to_variant());
        }

        if let Some(keys) = &config.sun.color {
            self.set_param(SUN_COLOR, &make_lut(keys, false).to_variant());
        }
    }

    /// The token and the scene are stamped now, not when the load runs, so a restore landing
    /// before it finishes drops the texture.
    fn load_panorama(&mut self, src: String) {
        self.texture_token += 1;
        let (token, scene_id, sky_id) = (self.texture_token, self.scene_id, self.sky_id);
        spawn_local(async move {
            let mapping = globals::scene_runner()
                .bind()
                .get_scene_content_mapping(scene_id);
            let promise = globals::content_provider()
                .bind_mut()
                .fetch_texture(src.to_godot(), mapping);
            let result = await_promise(promise).await;
            let Ok(mut sky) = Gd::<SkyBase>::try_from_instance_id(sky_id) else {
                return;
            };
            let mut sky = sky.bind_mut();
            if let Some(skybox) = sky.scene_skybox.as_mut() {
                skybox.on_panorama_loaded(token, &src, result);
            }
        });
    }

    fn on_panorama_loaded(&mut self, token: u64, src: &str, result: Variant) {
        if token != self.texture_token {
            return;
        }
        let texture = match result.try_to::<Gd<TextureEntry>>() {
            Ok(entry) if !entry.bind().failed => entry.bind().texture.clone(),
            _ => {
                let reason = result
                    .try_to::<Gd<PromiseError>>()
                    .map(|error| error.bind().get_error().to_string())
                    .unwrap_or_else(|_| "not a supported image".to_owned());
                tracing::warn!(
                    "PBSkybox.skybox_texture could not be loaded ({src}): {reason}; keeping the default sky"
                );
                self.apply_procedural_overrides();
                return;
            }
        };

        let size = texture.get_size();
        if !is_equirectangular(size) {
            // The proto asks for 2:1, but the Unity explorer stretches whatever arrives over the
            // dome: rejecting it would show a different sky than Unity does.
            tracing::info!(
                "PBSkybox.skybox_texture is not equirectangular 2:1, got {}x{} ({src}); rendering it stretched",
                size.x,
                size.y
            );
        }

        let Some(mut sky) = self
            .world_environment
            .get_environment()
            .and_then(|environment| environment.get_sky())
        else {
            return;
        };
        // Only the visible sky: `game_environment.tres` takes reflections from a custom source, so
        // reflective props keep mirroring the default sky (#3350).
        let Some(current) = sky.get_material() else {
            tracing::warn!("PBSkybox.skybox_texture: the sky has no material to replace");
            return;
        };
        let pristine = PRISTINE.with_borrow(|p| p.as_ref().and_then(|p| p.sky_material.clone()));
        self.saved_material = Some(pristine.unwrap_or(current));
        let mut pano = PanoramaSkyMaterial::new_gd();
        pano.set_panorama(&texture);
        pano.set_filtering_enabled(true);
        sky.set_material(&pano);
        self.pano_material = Some(pano);
    }

    fn material(&self) -> Option<Gd<ShaderMaterial>> {
        self.world_environment
            .get_environment()?
            .get_sky()?
            .get_material()?
            .try_cast::<ShaderMaterial>()
            .ok()
    }

    fn set_param(&mut self, param: &'static str, value: &Variant) {
        let Some(mut material) = self.material() else {
            return;
        };
        if !self.saved_params.iter().any(|(name, _)| *name == param) {
            // Presence, not value: an unset uniform captures as nil, and restoring nil is what
            // clears the override so the shader default applies again.
            let pristine = PRISTINE.with_borrow(|p| {
                p.as_ref()
                    .and_then(|p| p.params.iter().find(|(name, _)| *name == param))
                    .map(|(_, value)| value.clone())
            });
            self.saved_params
                .push((param, pristine.unwrap_or_default()));
        }
        material.set_shader_parameter(param, value);
        self.params_material = Some(material);
    }
}

/// The proto's density is per meter: fog = 1 - e^(-density * distance).
fn apply_exponential_fog(environment: &mut Gd<Environment>, density: f32) {
    environment.set_fog_mode(FogMode::EXPONENTIAL);
    environment.set_fog_density(density.max(0.0));
}

fn make_lut(keys: &GradientKeys, srgb_encode: bool) -> Gd<GradientTexture1D> {
    let offsets: PackedFloat32Array = keys.iter().map(|key| key[0]).collect();
    let colors: PackedColorArray = keys
        .iter()
        .map(|key| {
            let color = key_color(key);
            if srgb_encode {
                color.linear_to_srgb()
            } else {
                color
            }
        })
        .collect();
    let mut gradient = Gradient::new_gd();
    gradient.set_offsets(&offsets);
    gradient.set_colors(&colors);

    let mut lut = GradientTexture1D::new_gd();
    lut.set_gradient(&gradient);
    lut.set_width(LUT_WIDTH);
    lut.set_use_hdr(true);
    lut
}

/// Piecewise-linear sample at a normalized time of day, as the proto defines it: before the
/// first key or after the last, that key's color.
pub fn sample(keys: &[[f32; 5]], time_of_day: f32) -> Color {
    let (Some(first), Some(last)) = (keys.first(), keys.last()) else {
        return Color::WHITE;
    };
    if time_of_day <= first[0] {
        return key_color(first);
    }
    if time_of_day >= last[0] {
        return key_color(last);
    }
    for pair in keys.windows(2) {
        let (lower, upper) = (&pair[0], &pair[1]);
        if time_of_day <= upper[0] {
            let span = upper[0] - lower[0];
            if span <= 0.0 {
                return key_color(upper);
            }
            let weight = (time_of_day - lower[0]) / span;
            let mix = |i: usize| lower[i] + (upper[i] - lower[i]) * weight;
            return Color::from_rgba(mix(1), mix(2), mix(3), mix(4));
        }
    }
    key_color(last)
}

fn key_color(key: &[f32; 5]) -> Color {
    Color::from_rgba(key[1], key[2], key[3], key[4])
}

/// The light basis for a scene sun direction, as `Basis.looking_at(direction, up)`: -Z along the
/// light's travel, so the sky draws the disc at +Z, where the sun is. Straight overhead, the
/// proto's own example, is colinear with UP, so it takes BACK as its up.
pub fn sun_basis_for(direction: Vector3) -> Basis {
    let forward = direction.normalized();
    let up = if forward.dot(Vector3::UP).abs() < 0.999 {
        Vector3::UP
    } else {
        Vector3::BACK
    };
    let z = -forward;
    let x = up.cross(z).normalized();
    let y = z.cross(x);
    Basis::from_cols(x, y, z)
}

pub fn is_equirectangular(size: Vector2) -> bool {
    size.y > 0.0 && (size.x / size.y - 2.0).abs() <= EQUIRECTANGULAR_TOLERANCE
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: Color, b: Color) -> bool {
        [a.r - b.r, a.g - b.g, a.b - b.b, a.a - b.a]
            .iter()
            .all(|d| d.abs() < 1e-5)
    }

    fn approx_vec(a: Vector3, b: Vector3) -> bool {
        (a - b).length() < 1e-5
    }

    #[test]
    fn a_gradient_samples_piecewise_linear() {
        let keys = [
            [0.0, 1.0, 0.0, 0.0, 1.0],
            [0.5, 0.0, 1.0, 0.0, 0.5],
            [1.0, 0.0, 0.0, 1.0, 1.0],
        ];

        assert!(approx(
            sample(&keys, 0.0),
            Color::from_rgba(1.0, 0.0, 0.0, 1.0)
        ));
        assert!(approx(
            sample(&keys, 0.5),
            Color::from_rgba(0.0, 1.0, 0.0, 0.5)
        ));
        assert!(approx(
            sample(&keys, 0.25),
            Color::from_rgba(0.5, 0.5, 0.0, 0.75)
        ));
        // Outside the key range, the nearest key's color.
        assert!(approx(
            sample(&keys, -1.0),
            Color::from_rgba(1.0, 0.0, 0.0, 1.0)
        ));
        assert!(approx(
            sample(&keys, 2.0),
            Color::from_rgba(0.0, 0.0, 1.0, 1.0)
        ));
    }

    #[test]
    fn a_single_key_is_flat() {
        let keys = [[0.3, 0.25, 0.5, 0.75, 1.0]];
        let flat = Color::from_rgba(0.25, 0.5, 0.75, 1.0);

        assert!(approx(sample(&keys, 0.0), flat));
        assert!(approx(sample(&keys, 1.0), flat));
    }

    #[test]
    fn only_two_to_one_is_equirectangular() {
        assert!(is_equirectangular(Vector2::new(1024.0, 512.0)));
        assert!(!is_equirectangular(Vector2::new(1024.0, 1024.0)));
        assert!(!is_equirectangular(Vector2::new(1024.0, 341.0)));
        assert!(!is_equirectangular(Vector2::new(1024.0, 0.0)));
    }

    #[test]
    fn an_overhead_sun_draws_its_disc_at_the_zenith() {
        let overhead = sun_basis_for(Vector3::new(0.0, -1.0, 0.0));
        assert!(approx_vec(overhead.col_c(), Vector3::UP), "{overhead}");
        assert!((overhead.determinant() - 1.0).abs() < 1e-5, "a rotation");

        let west = sun_basis_for(Vector3::new(-1.0, 0.0, 0.0));
        assert!(approx_vec(west.col_c(), Vector3::RIGHT), "{west}");
        assert!((west.determinant() - 1.0).abs() < 1e-5, "a rotation");
    }

    #[test]
    fn every_override_uniform_has_a_stored_default() {
        // A uniform with no stored default captures as nil, and a sky swap then keeps the live,
        // already-overridden value: the scene sky would leak into the defaults.
        let sky = include_str!("../../../../../godot/assets/sky.tres");
        for param in OVERRIDE_PARAMS {
            assert!(
                sky.contains(&format!("\nshader_parameter/{param} = ")),
                "assets/sky.tres stores a default for {param}"
            );
        }
        assert!(
            sky.contains("\nshader_parameter/stars_override = 0.0\n"),
            "the default look keeps its night-only star fade"
        );
    }
}
