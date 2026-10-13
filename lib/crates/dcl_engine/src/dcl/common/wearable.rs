use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct I18N {
    pub code: String,
    pub text: String,
}

#[derive(Default, Serialize, Deserialize, Debug, Clone)]
pub struct BaseItemEntityMetadata {
    pub id: String,
    pub description: String,
    pub thumbnail: String,
    pub rarity: Option<String>,
    pub name: Option<String>,
    pub i18n: Vec<I18N>,

    #[serde(rename = "data")]
    pub wearable_data: Option<WearableData>,

    #[serde(rename = "emoteDataADR74")]
    pub emote_data: Option<EmoteDataADR74>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WearableData {
    pub tags: Vec<String>,
    pub category: WearableCategory,
    pub representations: Vec<WearableRepresentation>,
    pub replaces: Vec<String>,
    pub hides: Vec<String>,
    pub removes_default_hiding: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EmoteDataADR74 {
    pub tags: Vec<String>,
    pub representations: Vec<EmoteADR74Representation>,
    #[serde(rename = "loop")]
    pub r#loop: bool,
    pub category: String, // TODO: should be typed as WearableCategory?
                          // DANCE = 'dance',
                          // STUNT = 'stunt',
                          // GREETINGS = 'greetings',
                          // FUN = 'fun',
                          // POSES = 'poses',
                          // REACTIONS = 'reactions',
                          // HORROR = 'horror',
                          // MISCELLANEOUS = 'miscellaneous'
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WearableRepresentation {
    pub body_shapes: Vec<String>,
    pub main_file: String,
    pub override_replaces: Vec<String>,
    pub override_hides: Vec<String>,
    pub contents: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EmoteADR74Representation {
    pub body_shapes: Vec<String>,
    pub main_file: String,
    pub contents: Vec<String>,
}

#[derive(Serialize, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
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
    pub const UNKNOWN: WearableCategory = WearableCategory::texture("unknown");

    pub const EYES: WearableCategory = WearableCategory::texture("eyes");
    pub const EYEBROWS: WearableCategory = WearableCategory::texture("eyebrows");
    pub const MOUTH: WearableCategory = WearableCategory::texture("mouth");

    pub const FACIAL_HAIR: WearableCategory = WearableCategory::model("facial_hair");
    pub const HAIR: WearableCategory = WearableCategory::model("hair");
    pub const BODY_SHAPE: WearableCategory = WearableCategory::model("body_shape");
    pub const UPPER_BODY: WearableCategory = WearableCategory::model("upper_body");
    pub const LOWER_BODY: WearableCategory = WearableCategory::model("lower_body");
    pub const FEET: WearableCategory = WearableCategory::model("feet");
    pub const EARRING: WearableCategory = WearableCategory::model("earring");
    pub const EYEWEAR: WearableCategory = WearableCategory::model("eyewear");
    pub const HAT: WearableCategory = WearableCategory::model("hat");
    pub const HELMET: WearableCategory = WearableCategory::model("helmet");
    pub const MASK: WearableCategory = WearableCategory::model("mask");
    pub const TIARA: WearableCategory = WearableCategory::model("tiara");
    pub const TOP_HEAD: WearableCategory = WearableCategory::model("top_head");
    pub const SKIN: WearableCategory = WearableCategory::model("skin");
    pub const HANDS_WEAR: WearableCategory = WearableCategory::model("hands_wear");
    // Note: Head is not a wearable category, but it's used as an alias of a group of categories
    pub const HEAD: WearableCategory = WearableCategory::model("head");

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
            "hands_wear" => Ok(Self::HANDS_WEAR),
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
            Self::HANDS_WEAR,
        ]
        .iter()
    }
}
