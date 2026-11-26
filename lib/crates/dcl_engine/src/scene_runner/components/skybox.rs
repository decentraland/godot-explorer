//! The sky a scene asks for: `PBSkybox`, `PBSun` and `PBEnvironmentFog` on the ROOT entity of the
//! parcel scene the player is in, resolved into the overrides `sky::scene_skybox` applies.

use crate::{
    dcl::{
        components::{
            proto_components::{
                common::ColorGradient,
                sdk::components::{PbEnvironmentFog, PbSkybox, PbSun},
            },
            SceneComponentId, SceneEntityId,
        },
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
        SceneId,
    },
    godot_classes::dcl_global::DclGlobal,
    scene_runner::scene::Scene,
};

/// A color over the normalized time of day: `[time, r, g, b, a]` keys sorted by time, linear
/// color (values above 1 allowed).
pub type GradientKeys = Vec<[f32; 5]>;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SkyColorsOverride {
    pub zenith: Option<GradientKeys>,
    pub horizon: Option<GradientKeys>,
    pub nadir: Option<GradientKeys>,
    pub rim: Option<GradientKeys>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CloudsOverride {
    pub opacity: Option<f32>,
    /// Radians per second.
    pub speed: Option<f32>,
    pub color: Option<GradientKeys>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SunOverride {
    pub color: Option<GradientKeys>,
    pub visible: Option<bool>,
    pub intensity: Option<f32>,
    /// Normalized direction the light travels.
    pub direction: Option<[f32; 3]>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FogOverride {
    pub color: Option<GradientKeys>,
    /// Per meter: fog = 1 - e^(-density * distance).
    pub density: Option<f32>,
}

/// What a scene overrides. An unset field keeps the time-of-day default; the default value
/// overrides nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SceneSkyConfig {
    pub skybox_texture: Option<String>,
    pub sky_colors: SkyColorsOverride,
    pub clouds: CloudsOverride,
    pub stars_brightness: Option<f32>,
    pub sun: SunOverride,
    pub fog: FogOverride,
}

impl SceneSkyConfig {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// The overrides of the scene the player is in, published on `DclGlobal` for the sky.
#[derive(Debug, Default)]
pub struct PublishedSceneSky {
    pub config: SceneSkyConfig,
    /// The scene they belong to; entering a scene again publishes its components again.
    pub scene_id: Option<SceneId>,
    /// Bumped on every change, so the sky applies each config once.
    pub revision: u64,
}

impl PublishedSceneSky {
    /// Drops the overrides on a scene change, so the next scene publishes its own.
    pub fn reset(&mut self) {
        if !self.config.is_empty() {
            self.config = SceneSkyConfig::default();
            self.revision += 1;
        }
        self.scene_id = None;
    }
}

/// Resolves a proto `ColorGradient`: keys sorted by `time` with out-of-range times clamped, as
/// the proto specifies; on equal times the key that comes later in `keys` wins. Keys with a
/// non-finite time or color are dropped.
pub fn resolve_gradient(gradient: &ColorGradient) -> GradientKeys {
    let mut sorted: GradientKeys = gradient
        .keys
        .iter()
        .filter_map(|key| {
            let color = key.color.as_ref()?;
            let resolved = [key.time, color.r, color.g, color.b, color.a];
            if resolved.iter().any(|value| !value.is_finite()) {
                return None;
            }
            Some([
                resolved[0].clamp(0.0, 1.0),
                resolved[1],
                resolved[2],
                resolved[3],
                resolved[4],
            ])
        })
        .collect();

    sorted.sort_by(|a, b| a[0].total_cmp(&b[0]));

    let mut deduplicated: GradientKeys = Vec::with_capacity(sorted.len());
    for key in sorted {
        match deduplicated.last_mut() {
            Some(last) if last[0] == key[0] => *last = key,
            _ => deduplicated.push(key),
        }
    }
    deduplicated
}

fn gradient_keys(gradient: &Option<ColorGradient>) -> Option<GradientKeys> {
    let keys = resolve_gradient(gradient.as_ref()?);
    (!keys.is_empty()).then_some(keys)
}

fn finite(value: Option<f32>) -> Option<f32> {
    value.filter(|value| value.is_finite())
}

/// The overrides a scene's `PBSkybox` / `PBSun` / `PBEnvironmentFog` (all read from the ROOT
/// entity) ask for, with invalid values clamped and empty or non-finite ones dropped.
pub fn build_skybox_config(
    skybox: Option<&PbSkybox>,
    sun: Option<&PbSun>,
    fog: Option<&PbEnvironmentFog>,
) -> SceneSkyConfig {
    let mut config = SceneSkyConfig::default();

    if let Some(skybox) = skybox {
        if let Some(texture) = skybox.skybox_texture.as_deref() {
            let texture = texture.trim();
            if texture.is_empty() {
                tracing::warn!("PBSkybox.skybox_texture is an empty string, ignoring it");
            } else {
                config.skybox_texture = Some(texture.to_owned());
            }
        }

        if let Some(colors) = skybox.sky_colors.as_ref() {
            let horizon = gradient_keys(&colors.horizon);
            config.sky_colors = SkyColorsOverride {
                zenith: gradient_keys(&colors.zenith),
                nadir: gradient_keys(&colors.nadir),
                // Spec: an unset `rim` follows `horizon` when that one is set.
                rim: gradient_keys(&colors.rim).or_else(|| horizon.clone()),
                horizon,
            };
        }

        if let Some(clouds) = skybox.clouds.as_ref() {
            config.clouds = CloudsOverride {
                opacity: finite(clouds.opacity).map(|opacity| opacity.clamp(0.0, 1.0)),
                speed: finite(clouds.speed),
                color: gradient_keys(&clouds.color),
            };
        }

        if let Some(stars) = skybox.stars.as_ref() {
            config.stars_brightness = finite(stars.brightness).map(|b| b.max(0.0));
        }
    }

    if let Some(sun) = sun {
        config.sun = SunOverride {
            color: gradient_keys(&sun.color),
            visible: sun.visible,
            intensity: finite(sun.intensity).map(|intensity| intensity.max(0.0)),
            // A zero or non-finite direction means the time-of-day path.
            direction: sun.direction.as_ref().and_then(|direction| {
                let direction = [direction.x, direction.y, direction.z];
                let length_squared = direction.iter().map(|v| v * v).sum::<f32>();
                (direction.iter().all(|v| v.is_finite()) && length_squared > 1e-12).then(|| {
                    let length = length_squared.sqrt();
                    direction.map(|v| v / length)
                })
            }),
        };
    }

    if let Some(fog) = fog {
        config.fog = FogOverride {
            color: gradient_keys(&fog.color),
            density: finite(fog.density).map(|density| density.max(0.0)),
        };
    }

    config
}

/// Publishes the current parcel scene's sky overrides to `DclGlobal`. Neighbouring scenes keep
/// the sky the player has until the player walks into them (the scene change resets the
/// published scene, so its components are published again on entry); global scenes carry no
/// parcel sky.
pub fn update_skybox(
    scene: &mut Scene,
    crdt_state: &mut SceneCrdtState,
    current_parcel_scene_id: &SceneId,
) {
    if scene.scene_id != *current_parcel_scene_id {
        return;
    }

    let dirty_lww_components = &scene.current_dirty.lww_components;
    let is_dirty = [
        SceneComponentId::SKYBOX,
        SceneComponentId::SUN,
        SceneComponentId::ENVIRONMENT_FOG,
    ]
    .iter()
    .any(|component_id| {
        dirty_lww_components
            .get(component_id)
            .is_some_and(|dirty| dirty.contains(&SceneEntityId::ROOT))
    });

    let Some(mut global) = DclGlobal::try_singleton() else {
        return;
    };
    let mut global = global.bind_mut();
    let published = &mut global.scene_sky;
    if published.scene_id == Some(scene.scene_id) && !is_dirty {
        return;
    }

    let skybox = SceneCrdtStateProtoComponents::get_skybox(crdt_state)
        .get(&SceneEntityId::ROOT)
        .and_then(|entry| entry.value.as_ref());
    let sun = SceneCrdtStateProtoComponents::get_sun(crdt_state)
        .get(&SceneEntityId::ROOT)
        .and_then(|entry| entry.value.as_ref());
    let fog = SceneCrdtStateProtoComponents::get_environment_fog(crdt_state)
        .get(&SceneEntityId::ROOT)
        .and_then(|entry| entry.value.as_ref());

    let config = build_skybox_config(skybox, sun, fog);
    published.scene_id = Some(scene.scene_id);
    if published.config != config {
        tracing::debug!(
            "Skybox SDK control changed: scene={}, config={:?}",
            scene.scene_id.0,
            config
        );
        published.config = config;
        published.revision += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dcl::components::proto_components::{
        common::{Color4, ColorKey, Vector3},
        sdk::components::pb_skybox::{Clouds, SkyColors, Stars},
    };

    fn key(time: f32, r: f32, g: f32, b: f32) -> ColorKey {
        ColorKey {
            time,
            color: Some(Color4 { r, g, b, a: 1.0 }),
        }
    }

    fn gradient(keys: Vec<ColorKey>) -> ColorGradient {
        ColorGradient { keys }
    }

    #[test]
    fn gradient_keys_are_sorted_and_clamped() {
        let resolved = resolve_gradient(&gradient(vec![
            key(1.5, 1.0, 0.0, 0.0),
            key(-0.5, 0.0, 0.0, 1.0),
            key(0.5, 0.0, 1.0, 0.0),
        ]));

        assert_eq!(resolved.len(), 3);
        assert_eq!(resolved[0], [0.0, 0.0, 0.0, 1.0, 1.0]);
        assert_eq!(resolved[1], [0.5, 0.0, 1.0, 0.0, 1.0]);
        assert_eq!(resolved[2], [1.0, 1.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn equal_times_keep_the_later_key() {
        let resolved = resolve_gradient(&gradient(vec![
            key(0.5, 1.0, 0.0, 0.0),
            key(0.5, 0.0, 1.0, 0.0),
        ]));

        assert_eq!(resolved, vec![[0.5, 0.0, 1.0, 0.0, 1.0]]);
    }

    #[test]
    fn non_finite_keys_are_dropped() {
        let mut bad_color = key(0.0, 1.0, 0.0, 0.0);
        bad_color.color = Some(Color4 {
            r: f32::NAN,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        });
        let resolved = resolve_gradient(&gradient(vec![bad_color, key(0.0, 0.0, 0.0, 1.0)]));

        assert_eq!(resolved, vec![[0.0, 0.0, 0.0, 1.0, 1.0]]);
    }

    #[test]
    fn empty_gradient_is_unset() {
        assert!(gradient_keys(&Some(gradient(vec![]))).is_none());
        assert!(gradient_keys(&None).is_none());
    }

    #[test]
    fn no_components_means_no_overrides() {
        assert!(build_skybox_config(None, None, None).is_empty());
    }

    #[test]
    fn empty_skybox_means_no_overrides() {
        let skybox = PbSkybox::default();
        assert!(build_skybox_config(Some(&skybox), None, None).is_empty());
    }

    #[test]
    fn unset_fields_are_omitted() {
        let skybox = PbSkybox {
            sky_colors: Some(SkyColors {
                horizon: Some(gradient(vec![key(0.0, 1.0, 1.0, 1.0)])),
                ..Default::default()
            }),
            ..Default::default()
        };
        let config = build_skybox_config(Some(&skybox), None, None);

        assert!(config.sky_colors.horizon.is_some());
        assert!(config.sky_colors.zenith.is_none());
    }

    #[test]
    fn unset_rim_follows_horizon() {
        let skybox = PbSkybox {
            sky_colors: Some(SkyColors {
                horizon: Some(gradient(vec![key(0.0, 1.0, 0.5, 0.25)])),
                ..Default::default()
            }),
            ..Default::default()
        };
        let config = build_skybox_config(Some(&skybox), None, None);

        assert_eq!(config.sky_colors.rim, config.sky_colors.horizon);
    }

    #[test]
    fn explicit_rim_is_kept() {
        let skybox = PbSkybox {
            sky_colors: Some(SkyColors {
                horizon: Some(gradient(vec![key(0.0, 1.0, 0.5, 0.25)])),
                rim: Some(gradient(vec![key(0.0, 0.0, 0.0, 1.0)])),
                ..Default::default()
            }),
            ..Default::default()
        };
        let config = build_skybox_config(Some(&skybox), None, None);

        assert_ne!(config.sky_colors.rim, config.sky_colors.horizon);
    }

    #[test]
    fn texture_whitespace_is_ignored() {
        let skybox = PbSkybox {
            skybox_texture: Some("   ".to_owned()),
            ..Default::default()
        };
        assert!(build_skybox_config(Some(&skybox), None, None).is_empty());
    }

    #[test]
    fn values_are_clamped() {
        let sun = PbSun {
            intensity: Some(-2.0),
            ..Default::default()
        };
        let fog = PbEnvironmentFog {
            density: Some(-1.0),
            ..Default::default()
        };
        let config = build_skybox_config(None, Some(&sun), Some(&fog));

        assert_eq!(config.sun.intensity, Some(0.0));
        assert_eq!(config.fog.density, Some(0.0));
    }

    #[test]
    fn sun_direction_is_normalized_and_zero_is_ignored() {
        let sun = PbSun {
            direction: Some(Vector3 {
                x: 0.0,
                y: -4.0,
                z: 0.0,
            }),
            ..Default::default()
        };
        let config = build_skybox_config(None, Some(&sun), None);
        assert_eq!(config.sun.direction, Some([0.0, -1.0, 0.0]));

        let sun = PbSun {
            direction: Some(Vector3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            }),
            ..Default::default()
        };
        let config = build_skybox_config(None, Some(&sun), None);
        assert!(
            config.is_empty(),
            "zero direction keeps the time-of-day path"
        );
    }

    #[test]
    fn clouds_and_stars_are_reported() {
        let skybox = PbSkybox {
            clouds: Some(Clouds {
                opacity: Some(2.0),
                speed: Some(-0.02),
                color: Some(gradient(vec![key(0.0, 1.0, 1.0, 1.0)])),
            }),
            stars: Some(Stars {
                brightness: Some(-3.0),
            }),
            ..Default::default()
        };
        let config = build_skybox_config(Some(&skybox), None, None);

        assert_eq!(config.clouds.opacity, Some(1.0));
        assert!(
            (config.clouds.speed.unwrap() + 0.02).abs() < 1e-6,
            "speed is passed through in radians per second"
        );
        assert!(config.clouds.color.is_some());
        assert_eq!(config.stars_brightness, Some(0.0));
    }

    #[test]
    fn a_scene_change_drops_the_overrides_once() {
        let mut published = PublishedSceneSky {
            config: SceneSkyConfig {
                stars_brightness: Some(2.0),
                ..Default::default()
            },
            scene_id: Some(SceneId(7)),
            revision: 3,
        };

        published.reset();
        assert!(published.config.is_empty());
        assert_eq!(published.scene_id, None);
        assert_eq!(published.revision, 4, "the sky sees the change");

        published.reset();
        assert_eq!(published.revision, 4, "nothing to drop, nothing to apply");
    }
}
