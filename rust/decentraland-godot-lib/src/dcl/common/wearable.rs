use std::str::FromStr;

use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct WearableMeta {
    pub id: String,
    pub description: String,
    pub thumbnail: String,
    pub rarity: Option<String>,
    pub data: WearableData,
}

#[derive(Deserialize, Debug, Clone)]
pub struct WearableData {
    pub tags: Vec<String>,
    pub category: WearableCategory,
    pub representations: Vec<WearableRepresentation>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WearableRepresentation {
    pub body_shapes: Vec<String>,
    pub main_file: String,
    pub override_replaces: Vec<String>,
    pub override_hides: Vec<String>,
    pub contents: Vec<String>,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
pub struct WearableCategory {
    pub slot: &'static str,
    pub is_texture: bool,
}

impl<'de> serde::Deserialize<'de> for WearableCategory {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(WearableCategory::from_str(s.as_str()).unwrap_or(WearableCategory::UNKNOWN))
    }
}

impl WearableCategory {
    const UNKNOWN: WearableCategory = WearableCategory::texture("unknown");

    const EYES: WearableCategory = WearableCategory::texture("eyes");
    const EYEBROWS: WearableCategory = WearableCategory::texture("eyebrows");
    const MOUTH: WearableCategory = WearableCategory::texture("mouth");

    const FACIAL_HAIR: WearableCategory = WearableCategory::model("facial_hair");
    const HAIR: WearableCategory = WearableCategory::model("hair");
    const HEAD: WearableCategory = WearableCategory::model("head");
    const BODY_SHAPE: WearableCategory = WearableCategory::model("body_shape");
    const UPPER_BODY: WearableCategory = WearableCategory::model("upper_body");
    const LOWER_BODY: WearableCategory = WearableCategory::model("lower_body");
    const FEET: WearableCategory = WearableCategory::model("feet");
    const EARRING: WearableCategory = WearableCategory::model("earring");
    const EYEWEAR: WearableCategory = WearableCategory::model("eyewear");
    const HAT: WearableCategory = WearableCategory::model("hat");
    const HELMET: WearableCategory = WearableCategory::model("helmet");
    const MASK: WearableCategory = WearableCategory::model("mask");
    const TIARA: WearableCategory = WearableCategory::model("tiara");
    const TOP_HEAD: WearableCategory = WearableCategory::model("top_head");
    const SKIN: WearableCategory = WearableCategory::model("skin");

    const fn model(slot: &'static str) -> Self {
        Self {
            slot,
            is_texture: false,
        }
    }

    const fn texture(slot: &'static str) -> Self {
        Self {
            slot,
            is_texture: true,
        }
    }
}

impl FromStr for WearableCategory {
    type Err = anyhow::Error;

    fn from_str(slot: &str) -> Result<WearableCategory, Self::Err> {
        match slot {
            "eyebrows" => Ok(Self::EYEBROWS),
            "eyes" => Ok(Self::EYES),
            "facial_hair" => Ok(Self::FACIAL_HAIR),
            "hair" => Ok(Self::HAIR),
            "head" => Ok(Self::HEAD),
            "body_shape" => Ok(Self::BODY_SHAPE),
            "mouth" => Ok(Self::MOUTH),
            "upper_body" => Ok(Self::UPPER_BODY),
            "lower_body" => Ok(Self::LOWER_BODY),
            "feet" => Ok(Self::FEET),
            "earring" => Ok(Self::EARRING),
            "eyewear" => Ok(Self::EYEWEAR),
            "hat" => Ok(Self::HAT),
            "helmet" => Ok(Self::HELMET),
            "mask" => Ok(Self::MASK),
            "tiara" => Ok(Self::TIARA),
            "top_head" => Ok(Self::TOP_HEAD),
            "skin" => Ok(Self::SKIN),
            _ => {
                tracing::warn!("unrecognised wearable category: {slot}");
                Err(anyhow::anyhow!("unrecognised wearable category: {slot}"))
            }
        }
    }
}

impl WearableCategory {
    pub fn iter() -> impl Iterator<Item = &'static WearableCategory> {
        [
            Self::EYES,
            Self::EYEBROWS,
            Self::MOUTH,
            Self::FACIAL_HAIR,
            Self::HAIR,
            Self::HEAD,
            Self::UPPER_BODY,
            Self::LOWER_BODY,
            Self::FEET,
            Self::EARRING,
            Self::EYEWEAR,
            Self::HAT,
            Self::HELMET,
            Self::MASK,
            Self::TIARA,
            Self::TOP_HEAD,
            Self::SKIN,
        ]
        .iter()
    }
}
