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
pub struct SceneDisplay {
    pub title: Option<String>,
}

#[derive(Default, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SceneEntityMetadata {
    pub display: Option<SceneDisplay>,
    pub main: String,
    pub scene: SceneMetaScene,
    pub runtime_version: Option<String>,
    pub spawn_points: Option<Vec<SpawnPoint>>,
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

        // Parse all parcels and filter out outliers
        let mut all_parcels: Vec<Vector2i> = Vec::new();
        for parcel_str in s.parcels.iter() {
            if let Some(parcel) = parcel_from_str(parcel_str) {
                all_parcels.push(parcel);
            }
        }

        // Filter out suspicious outlier parcels
        const MAX_DISTANCE_FROM_BASE: i32 = 50; // Maximum reasonable distance from base parcel
        let mut valid_parcels = Vec::new();

        for parcel in all_parcels {
            let distance_from_base = (parcel.x - base_parcel.x)
                .abs()
                .max((parcel.y - base_parcel.y).abs());

            // Filter out parcels too far from base (likely data errors)
            // Also filter (0,0) if it's far from base, as it's likely a default/error value
            if distance_from_base > MAX_DISTANCE_FROM_BASE {
                continue;
            }

            // Parcel is within valid distance, keep it
            valid_parcels.push(parcel);
        }

        // Ensure base parcel is always included if not already
        if !valid_parcels.contains(&base_parcel) {
            valid_parcels.insert(0, base_parcel);
        }

        Ok(SceneMetaScene {
            base: base_parcel,
            parcels: valid_parcels,
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
