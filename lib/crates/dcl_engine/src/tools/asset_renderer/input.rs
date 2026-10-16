//! The asset renderer's input file. It is untrusted JSON: any field can hold any type, so reads
//! are coerced to a fallback instead of cast, and a bad entry is reported in `invalid_items` with
//! the reason it actually has instead of taking the whole batch down.

use serde_json::{Map, Value};

pub const VALID_KINDS: [&str; 3] = ["wearable_standalone", "wearable_on_avatar", "emote"];
/// Bounds of a requested render size. Zero would give a dead viewport whose only symptom is the
/// watchdog, and an absurd side would try to allocate gigabytes: both are caller mistakes worth
/// correcting rather than obeying.
pub const MIN_SHOT_SIDE: i64 = 16;
pub const MAX_SHOT_SIDE: i64 = 4096;
const DEFAULT_BODY_SHAPE: &str = "urn:decentraland:off-chain:base-avatars:BaseFemale";

fn float(value: Option<&Value>, fallback: f64) -> f64 {
    value.and_then(Value::as_f64).unwrap_or(fallback)
}

fn int(value: Option<&Value>, fallback: i64) -> i64 {
    value
        .and_then(Value::as_f64)
        .map_or(fallback, |number| number as i64)
}

fn boolean(value: Option<&Value>, fallback: bool) -> bool {
    value.and_then(Value::as_bool).unwrap_or(fallback)
}

fn string(value: Option<&Value>, fallback: &str) -> String {
    value
        .and_then(Value::as_str)
        .unwrap_or(fallback)
        .to_string()
}

fn vector(value: Option<&Value>, fallback: [f64; 3]) -> [f64; 3] {
    let Some(Value::Object(fields)) = value else {
        return fallback;
    };
    ["x", "y", "z"]
        .iter()
        .enumerate()
        .map(|(i, key)| float(fields.get(*key), fallback[i]))
        .collect::<Vec<_>>()
        .try_into()
        .unwrap_or(fallback)
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShotCamera {
    /// Orbits the asset's posed bounds; otherwise `position` and `target` are absolute.
    pub auto_fit: bool,
    pub orbit_yaw_degrees: f64,
    pub orbit_pitch_degrees: f64,
    pub fit_margin: f64,
    pub projection: String,
    pub fov: f64,
    pub ortho_size: f64,
    pub position: [f64; 3],
    pub target: [f64; 3],
}

impl Default for ShotCamera {
    fn default() -> Self {
        Self {
            auto_fit: true,
            orbit_yaw_degrees: 0.0,
            orbit_pitch_degrees: 10.0,
            fit_margin: 1.15,
            projection: "perspective".to_string(),
            fov: 40.0,
            ortho_size: 2.5,
            position: [0.0; 3],
            target: [0.0; 3],
        }
    }
}

impl ShotCamera {
    fn parse(fields: &Map<String, Value>) -> Self {
        let default = Self::default();
        Self {
            auto_fit: boolean(fields.get("autoFit"), default.auto_fit),
            orbit_yaw_degrees: float(fields.get("orbitYawDegrees"), default.orbit_yaw_degrees),
            orbit_pitch_degrees: float(
                fields.get("orbitPitchDegrees"),
                default.orbit_pitch_degrees,
            ),
            fit_margin: float(fields.get("fitMargin"), default.fit_margin),
            projection: string(fields.get("projection"), &default.projection),
            fov: float(fields.get("fov"), default.fov),
            ortho_size: float(fields.get("orthoSize"), default.ortho_size),
            position: vector(fields.get("position"), default.position),
            target: vector(fields.get("target"), default.target),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Shot {
    pub name: String,
    pub dest_path: String,
    pub width: i64,
    pub height: i64,
    /// Emotes only: the normalized animation time (0..1) to freeze at.
    pub at_time: f64,
    pub camera: ShotCamera,
}

impl Shot {
    /// The shot, or the reason it is invalid.
    fn parse(value: &Value) -> Result<Self, String> {
        let Value::Object(fields) = value else {
            return Err("a shot is not an object".to_string());
        };
        let name = string(fields.get("name"), "");
        let camera = match fields.get("camera") {
            None => ShotCamera::default(),
            Some(Value::Object(camera)) => ShotCamera::parse(camera),
            Some(_) => return Err(format!("shot {name} has a camera that is not an object")),
        };
        let dest_path = string(fields.get("destPath"), "");
        if dest_path.is_empty() {
            return Err(format!("shot {name} is missing destPath"));
        }
        // An explicit camera at its own target leaves look_at unable to orient, which would
        // silently reuse the previous shot's framing.
        if !camera.auto_fit && camera.position == camera.target {
            return Err(format!(
                "shot {name} needs a distinct camera position and target"
            ));
        }
        Ok(Self {
            name,
            dest_path,
            width: int(fields.get("width"), 1024).clamp(MIN_SHOT_SIDE, MAX_SHOT_SIDE),
            height: int(fields.get("height"), 1024).clamp(MIN_SHOT_SIDE, MAX_SHOT_SIDE),
            at_time: float(fields.get("atTime"), 0.5),
            camera,
        })
    }
}

#[derive(Clone, Debug)]
pub struct AssetItem {
    pub id: String,
    pub kind: String,
    pub urn: String,
    pub body_shape: String,
    /// Avatar fields merged over the synthesized avatar (`wearable_on_avatar` and `emote`).
    pub avatar_overrides: Map<String, Value>,
    pub shots: Vec<Shot>,
}

impl AssetItem {
    /// The item, or the reason it is invalid: an unusable item is reported, not fatal.
    fn parse(fields: &Map<String, Value>) -> Result<Self, String> {
        let id = string(fields.get("id"), "");
        let kind = string(fields.get("kind"), "wearable_standalone");
        let urn = string(fields.get("urn"), "");
        let avatar_overrides = match fields.get("avatar") {
            Some(Value::Object(overrides)) => overrides.clone(),
            _ => Map::new(),
        };
        // The report is keyed by id: without one its entry cannot be matched back to the request.
        if id.is_empty() {
            return Err("item needs an id".to_string());
        }
        if urn.is_empty() {
            return Err("item needs a urn".to_string());
        }
        if !VALID_KINDS.contains(&kind.as_str()) {
            return Err(format!("invalid kind {kind}"));
        }
        let shots = match fields.get("shots") {
            None => Vec::new(),
            Some(Value::Array(shots)) => shots
                .iter()
                .map(Shot::parse)
                .collect::<Result<Vec<_>, _>>()?,
            Some(_) => return Err("shots has to be an array".to_string()),
        };
        if shots.is_empty() {
            return Err("no shots".to_string());
        }
        Ok(Self {
            id,
            kind,
            urn,
            body_shape: string(fields.get("bodyShape"), DEFAULT_BODY_SHAPE),
            avatar_overrides,
            shots,
        })
    }
}

#[derive(Clone, Debug)]
pub struct InvalidItem {
    pub id: String,
    pub error: String,
}

#[derive(Debug)]
pub struct AssetInput {
    pub base_url: String,
    pub output_json_path: String,
    /// The viewport's `scaling_3d_scale` (clamped to 0.5..2.0 where it is used).
    pub supersample: f64,
    pub items: Vec<AssetItem>,
    pub invalid_items: Vec<InvalidItem>,
}

impl AssetInput {
    /// The whole input file; `None` (with the reason) when it cannot be used at all.
    pub fn parse(text: &str) -> Result<Self, String> {
        let value: Value = serde_json::from_str(text)
            .map_err(|_| "the file has to be a valid json dictionary".to_string())?;
        let Value::Object(file) = value else {
            return Err("the file has to be a valid json dictionary".to_string());
        };
        let payload = match (
            file.get("baseUrl"),
            file.get("outputJsonPath"),
            file.get("payload"),
        ) {
            (Some(_), Some(_), Some(payload)) => payload,
            _ => {
                return Err(
                    "baseUrl, outputJsonPath and payload have to be included in the file"
                        .to_string(),
                )
            }
        };
        let Value::Array(payload) = payload else {
            return Err("payload has to be an array".to_string());
        };
        let base_url = string(file.get("baseUrl"), "");
        let output_json_path = string(file.get("outputJsonPath"), "");
        if base_url.is_empty() || output_json_path.is_empty() {
            return Err("baseUrl and outputJsonPath have to be strings".to_string());
        }

        let mut input = Self {
            base_url,
            output_json_path,
            supersample: float(file.get("supersample"), 1.0),
            items: Vec::new(),
            invalid_items: Vec::new(),
        };
        for (index, entry) in payload.iter().enumerate() {
            let mut id = format!("payload[{index}]");
            let parsed = match entry {
                Value::Object(fields) => {
                    let declared = string(fields.get("id"), "");
                    if !declared.is_empty() {
                        id = declared;
                    }
                    AssetItem::parse(fields)
                }
                _ => Err("entry is not an object".to_string()),
            };
            match parsed {
                Ok(item) => input.items.push(item),
                Err(error) => input.invalid_items.push(InvalidItem { id, error }),
            }
        }
        Ok(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn shot(overrides: Value) -> Value {
        let mut shot =
            json!({"name": "y0", "destPath": "user://a.png", "width": 768, "height": 768});
        for (key, value) in overrides.as_object().unwrap() {
            shot[key] = value.clone();
        }
        shot
    }

    fn parse(payload: Value) -> AssetInput {
        AssetInput::parse(
            &json!({
                "baseUrl": "https://peer.decentraland.org/content",
                "outputJsonPath": "user://report.json",
                "payload": payload,
            })
            .to_string(),
        )
        .expect("a usable file")
    }

    fn invalid(entry: Value) -> InvalidItem {
        let input = parse(json!([entry]));
        assert!(input.items.is_empty());
        assert_eq!(input.invalid_items.len(), 1);
        input.invalid_items[0].clone()
    }

    #[test]
    fn invalid_entries_are_reported_for_the_reason_they_have() {
        let cases = [
            (
                json!("a string, not an object"),
                "payload[0]",
                "entry is not an object",
            ),
            (
                json!({"id": "no-id-urn", "shots": [shot(json!({}))]}),
                "no-id-urn",
                "item needs a urn",
            ),
            (
                json!({"urn": "urn:a", "shots": [shot(json!({}))]}),
                "payload[0]",
                "item needs an id",
            ),
            (
                json!({"id": "bad-kind", "kind": "scene", "urn": "urn:a", "shots": [shot(json!({}))]}),
                "bad-kind",
                "invalid kind scene",
            ),
            (
                json!({"id": "no-shots", "urn": "urn:a", "shots": []}),
                "no-shots",
                "no shots",
            ),
            (
                json!({"id": "shots-not-array", "urn": "urn:a", "shots": "nope"}),
                "shots-not-array",
                "shots has to be an array",
            ),
            (
                json!({"id": "shot-not-object", "urn": "urn:a", "shots": [7]}),
                "shot-not-object",
                "a shot is not an object",
            ),
            (
                json!({"id": "no-dest", "urn": "urn:a", "shots": [shot(json!({"destPath": ""}))]}),
                "no-dest",
                "shot y0 is missing destPath",
            ),
            (
                json!({"id": "camera-not-object", "urn": "urn:a", "shots": [shot(json!({"camera": "front"}))]}),
                "camera-not-object",
                "shot y0 has a camera that is not an object",
            ),
            (
                json!({"id": "degenerate-camera", "urn": "urn:a", "shots": [shot(json!({"camera": {
                    "autoFit": false,
                    "position": {"x": 1, "y": 1, "z": 1},
                    "target": {"x": 1, "y": 1, "z": 1}
                }}))]}),
                "degenerate-camera",
                "shot y0 needs a distinct camera position and target",
            ),
        ];
        for (entry, id, error) in cases {
            let item = invalid(entry);
            assert_eq!(item.id, id);
            assert_eq!(item.error, error, "for {id}");
        }
    }

    #[test]
    fn a_bad_entry_does_not_lose_the_others() {
        let input = parse(json!([
            {"id": "good-one", "urn": "urn:a", "shots": [shot(json!({}))]},
            12345,
            {"id": "good-two", "urn": "urn:b", "shots": [shot(json!({}))]},
        ]));
        assert_eq!(input.items.len(), 2);
        assert_eq!(input.invalid_items.len(), 1);
        assert_eq!(input.items[0].id, "good-one");
        assert_eq!(input.items[1].id, "good-two");
    }

    #[test]
    fn render_sizes_are_bounded() {
        let input = parse(json!([{
            "id": "sizes", "urn": "urn:a",
            "shots": [
                shot(json!({"width": 0, "height": 999999})),
                shot(json!({"name": "y1", "width": "big", "height": null})),
            ]
        }]));
        let shots = &input.items[0].shots;
        assert_eq!(shots[0].width, MIN_SHOT_SIDE);
        assert_eq!(shots[0].height, MAX_SHOT_SIDE);
        assert_eq!((shots[1].width, shots[1].height), (1024, 1024));
    }

    #[test]
    fn wrongly_typed_fields_fall_back() {
        let input = parse(json!([{
            "id": "weird", "urn": "urn:a", "avatar": "not an object",
            "shots": [shot(json!({"camera": {
                "autoFit": "yes", "fov": "wide", "position": "origin", "target": []
            }}))]
        }]));
        let item = &input.items[0];
        assert!(item.avatar_overrides.is_empty());
        let camera = &item.shots[0].camera;
        assert!(camera.auto_fit);
        assert_eq!(camera.fov, 40.0);
        assert_eq!(camera.position, [0.0; 3]);
        assert_eq!(camera.target, [0.0; 3]);
    }

    #[test]
    fn the_file_itself_has_to_be_usable() {
        assert!(AssetInput::parse("not json").is_err());
        assert!(AssetInput::parse("[]").is_err());
        assert!(AssetInput::parse(r#"{"baseUrl": "x"}"#).is_err());
        assert!(
            AssetInput::parse(r#"{"baseUrl": "x", "outputJsonPath": "y", "payload": {}}"#).is_err()
        );
        assert!(
            AssetInput::parse(r#"{"baseUrl": 1, "outputJsonPath": "y", "payload": []}"#).is_err()
        );
    }
}
