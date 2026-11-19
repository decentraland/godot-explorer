use std::{collections::HashMap, ops::Range};

use godot::builtin::{Vector2i, Vector3};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SpawnPosition {
    x: serde_json::Value,
    y: serde_json::Value,
    z: serde_json::Value,
}

impl SpawnPosition {
    pub fn bounding_box(&self) -> (Vector3, Vector3) {
        let parse_val = |v: &serde_json::Value| -> Option<Range<f32>> {
            if let Some(val) = v.as_f64() {
                Some(val as f32..val as f32)
            } else if let Some(array) = v.as_array() {
                if let Some(mut start) = array.first().and_then(|s| s.as_f64()) {
                    let mut end = array.get(1).and_then(|e| e.as_f64()).unwrap_or(start);
                    if end < start {
                        (start, end) = (end, start);
                    }
                    Some(start as f32..end as f32)
                } else {
                    None
                }
            } else {
                None
            }
        };

        let x = parse_val(&self.x).unwrap_or(0.0..16.0);
        let y = parse_val(&self.y).unwrap_or(0.0..16.0);
        let z = parse_val(&self.z).unwrap_or(0.0..16.0);

        (
            Vector3::new(x.start, y.start, z.start),
            Vector3::new(x.end, y.end, z.end),
        )
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SpawnPoint {
    pub name: Option<String>,
    // `default` is optional in the SDK scene schema; a spawn point without it is not the default.
    // Without `#[serde(default)]`, a missing field aborts the whole scene-metadata parse, which
    // silently drops the scene (e.g. pixelarcade.dcl.eth has a spawn point with no `default`).
    #[serde(default)]
    pub default: bool,
    pub position: SpawnPosition,
}

// This structs is a wrapper for the useful SceneMetaScene struct
#[derive(Serialize, Deserialize)]
struct OriginalSceneMetaScene {
    pub base: String,
    pub parcels: Vec<String>,
}

#[derive(Default, Debug)]
pub struct SceneMetaScene {
    pub base: Vector2i,
    pub parcels: Vec<Vector2i>,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SceneDisplay {
    pub title: Option<String>,
    pub navmap_thumbnail: Option<String>,
}

#[derive(Default, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SceneEntityMetadata {
    pub display: Option<SceneDisplay>,
    pub main: String,
    pub scene: SceneMetaScene,
    pub runtime_version: Option<String>,
    pub spawn_points: Option<Vec<SpawnPoint>>,
    pub authoritative_multiplayer: Option<bool>,
    pub landscape_terrain: Option<bool>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl<'de> serde::Deserialize<'de> for SceneMetaScene {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: OriginalSceneMetaScene = OriginalSceneMetaScene::deserialize(deserializer)?;

        // Helper function to parse parcel string, returns None for invalid parcels
        let parcel_from_str = |s: &str| -> Option<Vector2i> {
            let base_parcel = s.split(',').collect::<Vec<&str>>();
            if base_parcel.len() != 2 {
                return None;
            }

            let x = match base_parcel[0].parse::<i32>() {
                Ok(val) => val,
                Err(_) => return None,
            };

            let y = match base_parcel[1].parse::<i32>() {
                Ok(val) => val,
                Err(_) => return None,
            };

            Some(Vector2i::new(x, y))
        };

        // Parse base parcel - this must be valid
        let base_parcel = parcel_from_str(s.base.as_str())
            .ok_or_else(|| serde::de::Error::custom(format!("Invalid base parcel: {}", s.base)))?;

        // This list is the scene's declared extent: `getSceneInformation`, the in-scene
        // check and the floors read it, so only entries that are not "x,y" are dropped —
        // dropping reachable parcels reports a smaller scene than the deployed one.
        let mut parcels: Vec<Vector2i> = s
            .parcels
            .iter()
            .filter_map(|parcel_str| parcel_from_str(parcel_str))
            .collect();

        // Ensure base parcel is always included if not already
        if !parcels.contains(&base_parcel) {
            parcels.insert(0, base_parcel);
        }

        Ok(SceneMetaScene {
            base: base_parcel,
            parcels,
        })
    }
}

// Implement serialize for SceneMetaScene
impl serde::Serialize for SceneMetaScene {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let base = format!("{},{}", self.base.x, self.base.y);
        let parcels = self
            .parcels
            .iter()
            .map(|p| format!("{},{}", p.x, p.y))
            .collect::<Vec<String>>();

        let original = OriginalSceneMetaScene { base, parcels };
        original.serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_display_parses_navmap_thumbnail() {
        let json = serde_json::json!({
            "display": {
                "title": "Raft Game",
                "navmapThumbnail": "images/raft_game.png"
            },
            "main": "bin/index.js",
            "scene": { "base": "14,5", "parcels": ["14,5", "14,6"] }
        });

        let meta: SceneEntityMetadata = serde_json::from_value(json).unwrap();
        let display = meta.display.unwrap();
        assert_eq!(display.title.as_deref(), Some("Raft Game"));
        assert_eq!(
            display.navmap_thumbnail.as_deref(),
            Some("images/raft_game.png")
        );
    }

    fn metadata_from_parcels(base: &str, parcels: Vec<String>) -> SceneEntityMetadata {
        serde_json::from_value(serde_json::json!({
            "main": "bin/index.js",
            "scene": { "base": base, "parcels": parcels }
        }))
        .unwrap()
    }

    // #3039: a 100x100 scene e.g. with its base in a corner has parcels 99 away from
    // the base, and they used to be dropped, leaving a 51x51 (2601) scene.
    #[test]
    fn parcels_far_from_base_are_kept() {
        let mut parcels = Vec::new();
        for x in 0..100 {
            for y in 0..100 {
                parcels.push(format!("{x},{y}"));
            }
        }

        let meta = metadata_from_parcels("0,0", parcels);
        assert_eq!(meta.scene.parcels.len(), 10000);
        for corner in [(0, 99), (99, 0), (99, 99)] {
            assert!(meta
                .scene
                .parcels
                .contains(&Vector2i::new(corner.0, corner.1)));
        }

        // Scenes read the extent from `getSceneInformation`, which serializes this back
        // out, so the count has to survive the round trip too.
        let round_trip = serde_json::to_value(&meta).unwrap();
        assert_eq!(
            round_trip["scene"]["parcels"].as_array().unwrap().len(),
            10000
        );
    }

    #[test]
    fn parcels_negative_and_far_from_base_are_kept() {
        let meta = metadata_from_parcels("0,0", vec!["0,0".to_string(), "-80,-80".to_string()]);
        assert_eq!(
            meta.scene.parcels,
            vec![Vector2i::new(0, 0), Vector2i::new(-80, -80)]
        );
    }

    #[test]
    fn unparsable_parcel_entries_are_skipped() {
        let meta = metadata_from_parcels(
            "0,0",
            vec![
                "0,0".to_string(),
                "not-a-parcel".to_string(),
                "1".to_string(),
                "1,1".to_string(),
            ],
        );
        assert_eq!(
            meta.scene.parcels,
            vec![Vector2i::new(0, 0), Vector2i::new(1, 1)]
        );
    }

    #[test]
    fn base_parcel_is_added_when_missing_from_the_list() {
        let meta = metadata_from_parcels("5,5", vec!["5,6".to_string()]);
        assert_eq!(
            meta.scene.parcels,
            vec![Vector2i::new(5, 5), Vector2i::new(5, 6)]
        );
    }

    #[test]
    fn scene_display_without_thumbnail_parses() {
        let json = serde_json::json!({
            "display": { "title": "No Thumb" },
            "main": "bin/index.js",
            "scene": { "base": "0,0", "parcels": ["0,0"] }
        });

        let meta: SceneEntityMetadata = serde_json::from_value(json).unwrap();
        assert_eq!(meta.display.unwrap().navmap_thumbnail, None);
    }
}
