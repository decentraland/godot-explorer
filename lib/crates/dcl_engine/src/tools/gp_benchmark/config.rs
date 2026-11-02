//! The Genesis Plaza benchmark's configuration: the committed file plus what the deeplink (or the
//! CLI) overrides, so a matrix of runs needs no re-export.

use std::collections::HashMap;

use serde_json::{json, Map, Value};

const TRUTHY: [&str; 3] = ["true", "1", "yes"];

fn truthy(value: &str) -> bool {
    TRUTHY.contains(&value.to_lowercase().as_str())
}

/// What the overrides ask of the rest of the app, beyond the config itself.
#[derive(Debug, Default, PartialEq)]
pub struct SideEffects {
    pub skip_gltf: Option<bool>,
    pub fixed_skybox_time: Option<bool>,
}

#[derive(Clone, Debug, Default)]
pub struct Config(pub Map<String, Value>);

impl Config {
    /// `None` when the file is not a JSON object.
    pub fn parse(text: &str) -> Option<Self> {
        match serde_json::from_str::<Value>(text).ok()? {
            Value::Object(map) => Some(Self(map)),
            _ => None,
        }
    }

    pub fn string(&self, key: &str, fallback: &str) -> String {
        self.0
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or(fallback)
            .to_string()
    }

    pub fn seconds(&self, key: &str, fallback: i64) -> i64 {
        self.0
            .get(key)
            .and_then(Value::as_f64)
            .map_or(fallback, |s| s as i64)
    }

    pub fn flag(&self, key: &str, fallback: bool) -> bool {
        self.0.get(key).and_then(Value::as_bool).unwrap_or(fallback)
    }

    pub fn toggle(&self, key: &str) -> bool {
        self.0
            .get("toggles")
            .and_then(|t| t.get(key))
            .and_then(Value::as_bool)
            .unwrap_or(false)
    }

    pub fn int(&self, key: &str) -> Option<i64> {
        self.0.get(key).and_then(Value::as_i64)
    }

    pub fn float(&self, key: &str) -> Option<f64> {
        self.0.get(key).and_then(Value::as_f64)
    }

    pub fn output_path(&self) -> String {
        self.string("output_path", "user://output/gp-benchmark/result.json")
    }

    /// Applies the deeplink parameters (`bench-tag`, `bench-warmup`, `force-graphic-profile`, ...).
    pub fn apply_overrides(&mut self, params: &HashMap<String, String>) -> SideEffects {
        let get = |key: &str| params.get(key).filter(|v| !v.is_empty());
        let int = |key: &str| get(key).and_then(|v| v.parse::<i64>().ok());
        let float = |key: &str| get(key).and_then(|v| v.parse::<f64>().ok());

        if let Some(tag) = get("bench-tag") {
            let extension = self
                .string("output_path", "result.json")
                .rsplit_once('.')
                .map(|(_, ext)| ext.to_string())
                .unwrap_or_default();
            self.0.insert("tag".into(), json!(tag));
            self.0.insert(
                "output_path".into(),
                json!(format!("user://output/gp-benchmark/{tag}.{extension}")),
            );
        }

        let mut toggles = self
            .0
            .get("toggles")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        for (param, toggle) in [
            ("bench-disable-tweens", "disable_tweens"),
            ("bench-disable-transforms", "disable_transforms"),
        ] {
            if let Some(value) = get(param) {
                toggles.insert(toggle.into(), json!(truthy(value)));
            }
        }
        self.0.insert("toggles".into(), Value::Object(toggles));

        for (param, key) in [
            ("bench-warmup", "warmup_seconds"),
            ("bench-sample", "sample_seconds"),
            ("bench-settling-timeout", "settling_timeout_seconds"),
        ] {
            if let Some(seconds) = int(param) {
                self.0.insert(key.into(), json!(seconds));
            }
        }
        // `bench-force-settling` disables the early exit on "nothing loading", which fires before
        // the glTF children reach the tree; `bench-keep-running` skips the quit at the end so the
        // captured state can be walked around in.
        for (param, key) in [
            ("bench-force-settling", "force_settling"),
            ("bench-keep-running", "keep_running"),
        ] {
            if let Some(value) = get(param) {
                self.0.insert(key.into(), json!(truthy(value)));
            }
        }
        // Profile index as GraphicSettings names them: 0 very low .. 4 custom.
        if let Some(index) = int("force-graphic-profile").filter(|i| (0..=4).contains(i)) {
            self.0.insert("force_graphic_profile".into(), json!(index));
        }
        if let Some(scale) = float("viewport-scale-3d").filter(|s| *s > 0.1 && *s <= 2.0) {
            self.0.insert("viewport_scale_3d".into(), json!(scale));
        }
        // Raising the mesh LOD threshold picks lower-detail LODs sooner, the point of the LOD bake
        // on mobile; applied after loading so the hardware benchmark cannot clobber it.
        if let Some(threshold) = float("mesh-lod-threshold").filter(|t| (0.0..=64.0).contains(t)) {
            self.0.insert("mesh_lod_threshold".into(), json!(threshold));
        }
        // Per-feature quality overrides (0 off .. 2 high), applied after the forced profile to
        // isolate the cost of one feature.
        for key in [
            "gfx-aa",
            "gfx-shadow",
            "gfx-bloom",
            "gfx-skybox",
            "gfx-texture",
        ] {
            if let Some(level) = int(key) {
                self.0.insert(key.into(), json!(level));
            }
        }
        if let Some(mode) = get("debug-draw") {
            self.0
                .insert("debug_draw".into(), json!(mode.to_lowercase()));
        }
        SideEffects {
            skip_gltf: get("skip-gltf").map(|v| truthy(v)),
            fixed_skybox_time: get("fixed-skybox-time").map(|v| truthy(v)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Config {
        Config::parse(
            r#"{"warmup_seconds": 30, "sample_seconds": 10, "tag": "baseline",
                "output_path": "user://output/gp-benchmark/baseline.json",
                "toggles": {"disable_tweens": false}}"#,
        )
        .unwrap()
    }

    fn params(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn only_an_object_is_a_config() {
        assert!(Config::parse("[]").is_none());
        assert!(Config::parse("nope").is_none());
        assert_eq!(base().seconds("warmup_seconds", 0), 30);
    }

    #[test]
    fn a_tag_renames_the_output() {
        let mut config = base();
        config.apply_overrides(&params(&[("bench-tag", "no-shadow")]));
        assert_eq!(config.string("tag", ""), "no-shadow");
        assert_eq!(
            config.output_path(),
            "user://output/gp-benchmark/no-shadow.json"
        );
    }

    #[test]
    fn toggles_and_durations_follow_the_deeplink() {
        let mut config = base();
        config.apply_overrides(&params(&[
            ("bench-disable-tweens", "YES"),
            ("bench-disable-transforms", "0"),
            ("bench-warmup", "5"),
            ("bench-sample", "oops"),
        ]));
        assert!(config.toggle("disable_tweens"));
        assert!(!config.toggle("disable_transforms"));
        assert_eq!(config.seconds("warmup_seconds", 0), 5);
        assert_eq!(config.seconds("sample_seconds", 0), 10);
    }

    #[test]
    fn graphics_overrides_are_bounded() {
        let mut config = base();
        config.apply_overrides(&params(&[
            ("force-graphic-profile", "9"),
            ("viewport-scale-3d", "3.0"),
            ("mesh-lod-threshold", "8"),
            ("gfx-shadow", "0"),
        ]));
        assert_eq!(config.int("force_graphic_profile"), None);
        assert_eq!(config.float("viewport_scale_3d"), None);
        assert_eq!(config.float("mesh_lod_threshold"), Some(8.0));
        assert_eq!(config.int("gfx-shadow"), Some(0));
    }

    #[test]
    fn side_effects_are_reported_not_stored() {
        let mut config = base();
        let effects = config.apply_overrides(&params(&[
            ("skip-gltf", "true"),
            ("fixed-skybox-time", "no"),
        ]));
        assert_eq!(
            effects,
            SideEffects {
                skip_gltf: Some(true),
                fixed_skybox_time: Some(false)
            }
        );
        assert_eq!(config.apply_overrides(&params(&[])), SideEffects::default());
    }
}
