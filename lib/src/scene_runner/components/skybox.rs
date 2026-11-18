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
use godot::prelude::{GString, ToGodot};

/// Resolves a proto `ColorGradient` into `[time, r, g, b, a]` keys, linear color
/// (values above 1 allowed). Keys are sorted by `time` and out-of-range times are
/// clamped, as the proto specifies; on equal times the key that comes later in
/// `keys` wins. Keys with a non-finite time or color are dropped: the Godot side
/// consumes this as JSON, which cannot carry NaN or infinity.
pub fn resolve_gradient(gradient: &ColorGradient) -> Vec<[f32; 5]> {
    let mut sorted: Vec<[f32; 5]> = gradient
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

    let mut deduplicated: Vec<[f32; 5]> = Vec::with_capacity(sorted.len());
    for key in sorted {
        match deduplicated.last_mut() {
            Some(last) if last[0] == key[0] => *last = key,
            _ => deduplicated.push(key),
        }
    }
    deduplicated
}

fn gradient_json(gradient: &Option<ColorGradient>) -> Option<serde_json::Value> {
    let keys = resolve_gradient(gradient.as_ref()?);
    if keys.is_empty() {
        return None;
    }
    Some(serde_json::Value::Array(
        keys.iter().map(|key| serde_json::json!(key)).collect(),
    ))
}

fn finite_option(value: Option<f32>) -> Option<f32> {
    value.filter(|value| value.is_finite())
}

/// Builds the override map a scene's `PBSkybox` / `PBSun` / `PBEnvironmentFog`
/// (all read from the ROOT entity) asks for. Absent fields stay absent so the
/// Godot side keeps the time-of-day default; an empty map means nothing to change.
pub fn build_skybox_config(
    skybox: Option<&PbSkybox>,
    sun: Option<&PbSun>,
    fog: Option<&PbEnvironmentFog>,
) -> serde_json::Map<String, serde_json::Value> {
    let mut config = serde_json::Map::new();

    if let Some(skybox) = skybox {
        if let Some(texture) = skybox.skybox_texture.as_deref() {
            let texture = texture.trim();
            if texture.is_empty() {
                tracing::warn!("PBSkybox.skybox_texture is an empty string, ignoring it");
            } else {
                config.insert("skybox_texture".to_owned(), serde_json::json!(texture));
            }
        }

        if let Some(colors) = skybox.sky_colors.as_ref() {
            let mut gradient_map = serde_json::Map::new();
            for (name, gradient) in [
                ("zenith", &colors.zenith),
                ("horizon", &colors.horizon),
                ("nadir", &colors.nadir),
                ("rim", &colors.rim),
            ] {
                if let Some(json) = gradient_json(gradient) {
                    gradient_map.insert(name.to_owned(), json);
                }
            }
            if !gradient_map.is_empty() {
                config.insert(
                    "sky_colors".to_owned(),
                    serde_json::Value::Object(gradient_map),
                );
            }
        }

        if let Some(clouds) = skybox.clouds.as_ref() {
            let mut clouds_map = serde_json::Map::new();
            if let Some(opacity) = finite_option(clouds.opacity) {
                clouds_map.insert(
                    "opacity".to_owned(),
                    serde_json::json!(opacity.clamp(0.0, 1.0)),
                );
            }
            if let Some(speed) = finite_option(clouds.speed) {
                clouds_map.insert("speed".to_owned(), serde_json::json!(speed));
            }
            if let Some(json) = gradient_json(&clouds.color) {
                clouds_map.insert("color".to_owned(), json);
            }
            if !clouds_map.is_empty() {
                config.insert("clouds".to_owned(), serde_json::Value::Object(clouds_map));
            }
        }

        if let Some(stars) = skybox.stars.as_ref() {
            if let Some(brightness) = finite_option(stars.brightness) {
                config.insert(
                    "stars".to_owned(),
                    serde_json::json!({ "brightness": brightness.max(0.0) }),
                );
            }
        }
    }

    if let Some(sun) = sun {
        let mut sun_map = serde_json::Map::new();
        if let Some(json) = gradient_json(&sun.color) {
            sun_map.insert("color".to_owned(), json);
        }
        if let Some(visible) = sun.visible {
            sun_map.insert("visible".to_owned(), serde_json::json!(visible));
        }
        if let Some(intensity) = finite_option(sun.intensity) {
            sun_map.insert(
                "intensity".to_owned(),
                serde_json::json!(intensity.max(0.0)),
            );
        }
        if let Some(direction) = sun.direction.as_ref() {
            let direction = [direction.x, direction.y, direction.z];
            // A zero or non-finite direction means the time-of-day path.
            let length_squared = direction.iter().map(|v| v * v).sum::<f32>();
            if direction.iter().all(|v| v.is_finite()) && length_squared > 1e-12 {
                let length = length_squared.sqrt();
                sun_map.insert(
                    "direction".to_owned(),
                    serde_json::json!([
                        direction[0] / length,
                        direction[1] / length,
                        direction[2] / length
                    ]),
                );
            }
        }
        if !sun_map.is_empty() {
            config.insert("sun".to_owned(), serde_json::Value::Object(sun_map));
        }
    }

    if let Some(fog) = fog {
        let mut fog_map = serde_json::Map::new();
        if let Some(json) = gradient_json(&fog.color) {
            fog_map.insert("color".to_owned(), json);
        }
        if let Some(density) = finite_option(fog.density) {
            fog_map.insert("density".to_owned(), serde_json::json!(density.max(0.0)));
        }
        if !fog_map.is_empty() {
            config.insert("fog".to_owned(), serde_json::Value::Object(fog_map));
        }
    }

    config
}

/// Publishes the current scene's `PBSkybox` / `PBSun` / `PBEnvironmentFog` overrides
/// to `DclGlobal`, for the Godot side to apply to its environment.
///
/// Only the current parcel scene is honored: neighbouring scenes keep the sky the
/// player already has until the player walks into them (entering resets
/// `sdk_skybox_scene_id`, so the components are re-published on entry). Global
/// scenes carry no parcel sky, so they are skipped.
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

    // Nothing changed since the last publish for this scene: re-publishing would
    // only churn the JSON and the signal.
    if global.bind().sdk_skybox_scene_id == scene.scene_id.0 && !is_dirty {
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
    let is_active = !config.is_empty();
    let config_json = if is_active {
        serde_json::Value::Object(config).to_string()
    } else {
        String::new()
    };

    let mut global_bind = global.bind_mut();
    let changed = global_bind.sdk_skybox_active != is_active
        || global_bind.sdk_skybox_config.to_string() != config_json;
    global_bind.sdk_skybox_active = is_active;
    global_bind.sdk_skybox_config = GString::from(&config_json);
    global_bind.sdk_skybox_scene_id = scene.scene_id.0;
    drop(global_bind);

    if changed {
        tracing::debug!(
            "Skybox SDK control changed: active={}, scene={}, config={}",
            is_active,
            scene.scene_id.0,
            config_json
        );
        // Deferred: this runs inside the scene tick, where SceneManager is borrowed mutably.
        // A handler that resolves a sky texture calls back into it (get_scene_content_mapping)
        // and would panic on the re-entrant bind.
        let signal_args = [
            "sdk_skybox_changed".to_variant(),
            GString::from(&config_json).to_variant(),
            scene.scene_id.0.to_variant(),
        ];
        global
            .bind()
            .scene_runner
            .clone()
            .call_deferred("emit_signal", &signal_args);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dcl::components::proto_components::{
        common::{Color4, ColorKey},
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
        assert!(gradient_json(&Some(gradient(vec![]))).is_none());
        assert!(gradient_json(&None).is_none());
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
        let sky_colors = config["sky_colors"].as_object().unwrap();

        assert!(sky_colors.contains_key("horizon"));
        assert!(!sky_colors.contains_key("zenith"));
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

        assert_eq!(config["sun"]["intensity"], serde_json::json!(0.0));
        assert_eq!(config["fog"]["density"], serde_json::json!(0.0));
    }

    #[test]
    fn sun_direction_is_normalized_and_zero_is_ignored() {
        let sun = PbSun {
            direction: Some(crate::dcl::components::proto_components::common::Vector3 {
                x: 0.0,
                y: -4.0,
                z: 0.0,
            }),
            ..Default::default()
        };
        let config = build_skybox_config(None, Some(&sun), None);
        assert_eq!(
            config["sun"]["direction"],
            serde_json::json!([0.0, -1.0, 0.0])
        );

        let sun = PbSun {
            direction: Some(crate::dcl::components::proto_components::common::Vector3 {
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

        assert_eq!(config["clouds"]["opacity"], serde_json::json!(1.0));
        assert!(
            (config["clouds"]["speed"].as_f64().unwrap() + 0.02).abs() < 1e-6,
            "speed is passed through in radians per second"
        );
        assert!(config["clouds"]["color"].is_array());
        assert_eq!(config["stars"]["brightness"], serde_json::json!(0.0));
    }

    #[test]
    fn config_serializes_to_parseable_json() {
        let skybox = PbSkybox {
            skybox_texture: Some("models/sky.png".to_owned()),
            sky_colors: Some(SkyColors {
                horizon: Some(gradient(vec![key(0.0, 1.0, 0.5, 0.25)])),
                ..Default::default()
            }),
            ..Default::default()
        };
        let json =
            serde_json::Value::Object(build_skybox_config(Some(&skybox), None, None)).to_string();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(
            parsed["skybox_texture"],
            serde_json::json!("models/sky.png")
        );
        assert_eq!(
            parsed["sky_colors"]["horizon"][0],
            serde_json::json!([0.0, 1.0, 0.5, 0.25, 1.0])
        );
    }
}
