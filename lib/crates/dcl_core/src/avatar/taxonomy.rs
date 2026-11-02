//! Fixed facts about avatars: the body shapes, the categories that cannot be removed, the
//! colour palettes and the emotes every account has.

use crate::app::ColorTarget;

pub const BASE_AVATARS: &str = "urn:decentraland:off-chain:base-avatars:";
pub const BASE_EMOTES: &str = "urn:decentraland:off-chain:base-emotes:";
pub const FEMALE: &str = "urn:decentraland:off-chain:base-avatars:BaseFemale";
pub const MALE: &str = "urn:decentraland:off-chain:base-avatars:BaseMale";
pub const EMOTE_SLOTS: usize = 10;

/// Body parts the avatar cannot be without; unequipping them is ignored.
const REQUIRED: [&str; 4] = ["body_shape", "eyes", "eyebrows", "mouth"];

pub fn is_removable(category: &str) -> bool {
    !REQUIRED.contains(&category)
}

pub fn is_base_wearable(urn: &str) -> bool {
    urn.to_ascii_lowercase()
        .starts_with(BASE_AVATARS.to_ascii_lowercase().as_str())
}

/// What a missing required category falls back to, by body shape.
pub(super) fn default_for(body_shape: &str, category: &str) -> Option<&'static str> {
    let female = body_shape.eq_ignore_ascii_case(FEMALE);
    Some(match (category, female) {
        ("eyes", false) => "urn:decentraland:off-chain:base-avatars:eyes_00",
        ("eyes", true) => "urn:decentraland:off-chain:base-avatars:f_eyes_00",
        ("eyebrows", false) => "urn:decentraland:off-chain:base-avatars:eyebrows_00",
        ("eyebrows", true) => "urn:decentraland:off-chain:base-avatars:f_eyebrows_00",
        ("mouth", false) => "urn:decentraland:off-chain:base-avatars:mouth_00",
        ("mouth", true) => "urn:decentraland:off-chain:base-avatars:f_mouth_00",
        _ => return None,
    })
}

pub(super) const REQUIRED_WITH_DEFAULT: [&str; 3] = ["eyes", "eyebrows", "mouth"];

/// The swatches the editor offers for a colour.
pub fn palette(target: ColorTarget) -> &'static [&'static str] {
    match target {
        ColorTarget::Skin => &[
            "#ffe4c6", "#ffdcb6", "#f3c9a8", "#d9a982", "#c49168", "#a56f4a", "#8a5a3a", "#6b3f28",
            "#4a2a1a",
        ],
        ColorTarget::Hair => &[
            "#1c1c1c", "#3b2412", "#5a3410", "#7b4c14", "#9a6338", "#8a2515", "#e0952c", "#c9b27a",
            "#d9d9d9",
        ],
        ColorTarget::Eyes => &[
            "#2b2b2b", "#4a3220", "#6b4a2e", "#2f6f4a", "#2f6fb0", "#6a8fa8", "#7a3f8f", "#9a9a9a",
        ],
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct BaseEmote {
    pub id: String,
    pub urn: String,
    pub name: String,
}

/// The emotes the client ships; profiles store them as the bare id or as the full urn.
pub fn base_emotes() -> Vec<BaseEmote> {
    const EMOTES: [(&str, &str); 21] = [
        ("handsair", "Hands Air"),
        ("wave", "Wave"),
        ("fistpump", "Fist Pump"),
        ("dance", "Dance"),
        ("raiseHand", "Raise Hand"),
        ("clap", "Clap"),
        ("money", "Money"),
        ("kiss", "Kiss"),
        ("shrug", "Shrug"),
        ("headexplode", "Head Explode"),
        ("cry", "Cry"),
        ("dab", "Dab"),
        ("disco", "Disco"),
        ("dontsee", "Don't See"),
        ("hammer", "Hammer"),
        ("hohoho", "Ho Ho Ho"),
        ("robot", "Robot"),
        ("snowfall", "Snowfall"),
        ("tektonik", "Tektonik"),
        ("tik", "Tik"),
        ("confettipopper", "Confetti Popper"),
    ];
    EMOTES
        .iter()
        .map(|(id, name)| BaseEmote {
            id: (*id).to_string(),
            urn: format!("{BASE_EMOTES}{id}"),
            name: (*name).to_string(),
        })
        .collect()
}
