//! Emote ids: the built-in base emotes, the scene-triggered utility emotes and the URNs.

pub const BASE_EMOTES_URN_PREFIX: &str = "urn:decentraland:off-chain:base-emotes:";

/// Base emotes, loaded remotely as `base-emotes` URNs.
const DEFAULT_EMOTES: [&str; 21] = [
    "handsair",
    "wave",
    "fistpump",
    "dance",
    "raiseHand",
    "clap",
    "money",
    "kiss",
    "shrug",
    "headexplode",
    "cry",
    "dab",
    "disco",
    "dontsee",
    "hammer",
    "hohoho",
    "robot",
    "snowfall",
    "tektonik",
    "tik",
    "confettipopper",
];

/// Utility emotes: local animations from the `default_actions` library.
const UTILITY_EMOTES: [&str; 17] = [
    "buttonDown",
    "buttonFront",
    "crafting",
    "getHit",
    "knockOut",
    "lever",
    "openChest",
    "openDoor",
    "punch",
    "push",
    "sittingChair1",
    "sittingChair2",
    "sittingGround1",
    "sittingGround2",
    "swingWeaponOneHand",
    "swingWeaponTwoHands",
    "throw",
];

pub fn is_emote_default(id: &str) -> bool {
    DEFAULT_EMOTES.contains(&id)
}

pub fn is_emote_utility(id: &str) -> bool {
    UTILITY_EMOTES.contains(&id)
}

pub fn base_emote_urn(id: &str) -> String {
    format!("{BASE_EMOTES_URN_PREFIX}{id}")
}

pub fn is_base_emote_urn(urn: &str) -> bool {
    urn.starts_with(BASE_EMOTES_URN_PREFIX)
}

/// `urn:decentraland:off-chain:scene-emote:{scene}-{hash}-{loop}`, parsed from the end because
/// scene ids can carry dashes (preview content: `b64-<base64>`).
pub struct SceneEmoteUrn {
    pub scene_id: String,
    pub glb_hash: String,
    pub looping: bool,
}

impl SceneEmoteUrn {
    pub fn parse(urn: &str) -> Option<Self> {
        let parts: Vec<&str> = urn.split(':').collect();
        if parts.len() != 5 {
            return None;
        }
        let payload = parts[4];
        let last = payload
            .rfind('-')
            .filter(|i| *i > 0 && *i + 1 < payload.len())?;
        let looping = &payload[last + 1..] == "true";
        let rest = &payload[..last];
        let second = rest.rfind('-').filter(|i| *i > 0 && *i + 1 < rest.len())?;
        Some(Self {
            scene_id: rest[..second].to_string(),
            glb_hash: rest[second + 1..].to_string(),
            looping,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_ids() {
        assert!(is_emote_default("wave") && !is_emote_utility("wave"));
        assert!(is_emote_utility("punch") && !is_emote_default("punch"));
        assert_eq!(
            base_emote_urn("clap"),
            "urn:decentraland:off-chain:base-emotes:clap"
        );
        assert!(is_base_emote_urn(&base_emote_urn("clap")));
    }

    #[test]
    fn scene_urn_parses_from_the_end() {
        let urn = "urn:decentraland:off-chain:scene-emote:b64-abc-bafyhash-true";
        let parsed = SceneEmoteUrn::parse(urn).unwrap();
        assert_eq!(parsed.scene_id, "b64-abc");
        assert_eq!(parsed.glb_hash, "bafyhash");
        assert!(parsed.looping);
        assert!(SceneEmoteUrn::parse("urn:decentraland:wearables:x").is_none());
        assert!(SceneEmoteUrn::parse("urn:a:b:c:nodash").is_none());
    }
}
