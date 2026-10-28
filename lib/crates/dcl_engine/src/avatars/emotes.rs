//! Emote ids: the built-in base emotes, the scene-triggered utility emotes and the URNs.

pub const BASE_EMOTES_URN_PREFIX: &str = "urn:decentraland:off-chain:base-emotes:";
/// Catalyst collection holding the scene "feedback" emotes (throw, punch, sittingChair1...),
/// deployed 2026-09-17. Bevy puts these URNs on the wire for `triggerEmote` since
/// bevy-explorer#1290; Unity and this client send the bare id.
pub const BASE_SCENE_EMOTES_URN_PREFIX: &str = "urn:decentraland:off-chain:base-scene-emotes:";
/// Emote files shipped inside a scene: `{prefix}{sceneId}-{glbHash}-{loop}`.
pub const SCENE_EMOTE_URN_PREFIX: &str = "urn:decentraland:off-chain:scene-emote:";

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

/// The base emote ids in slot order: the wheel's initial state.
pub fn default_emote_ids() -> &'static [&'static str] {
    &DEFAULT_EMOTES
}

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

/// The id of a base-emote URN; anything else comes back as it is.
pub fn base_emote_id(urn: &str) -> &str {
    urn.strip_prefix(BASE_EMOTES_URN_PREFIX).unwrap_or(urn)
}

pub fn is_scene_emote_urn(urn: &str) -> bool {
    urn.starts_with(SCENE_EMOTE_URN_PREFIX)
}

fn strip_prefix_ignore_case<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    let head = s.get(..prefix.len())?;
    head.eq_ignore_ascii_case(prefix)
        .then(|| &s[prefix.len()..])
}

/// The id as spelled in the built-in tables, whatever its case.
fn embedded_id(id: &str) -> Option<&'static str> {
    DEFAULT_EMOTES
        .iter()
        .chain(UTILITY_EMOTES.iter())
        .find(|known| known.eq_ignore_ascii_case(id))
        .copied()
}

/// Maps every spelling other clients put on the wire for a built-in emote to the one this
/// client plays (#2986):
/// - `...:base-scene-emotes:<id>` and `...:base-emotes:<id>` naming a utility emote -> the bare
///   utility id, played from the bundled `default_actions` library
/// - those URNs naming a base emote -> the `base-emotes` URN with canonical casing
/// - a bare id in any case -> the canonical bare id ("sittingchair2" -> "sittingChair2")
///
/// Anything else (on-chain, scene-emote, ids this client doesn't ship) comes back unchanged, so
/// an unknown `base-(scene-)emotes` URN still resolves through the catalyst.
pub fn normalize_emote_id(urn_or_id: &str) -> String {
    let mut id = urn_or_id;
    let mut is_urn = false;
    if let Some(rest) = strip_prefix_ignore_case(urn_or_id, BASE_SCENE_EMOTES_URN_PREFIX)
        .or_else(|| strip_prefix_ignore_case(urn_or_id, BASE_EMOTES_URN_PREFIX))
    {
        id = rest;
        is_urn = true;
    } else if strip_prefix_ignore_case(urn_or_id, "urn:").is_some() {
        return urn_or_id.to_string();
    }
    match embedded_id(id) {
        None => urn_or_id.to_string(),
        Some(canonical) if is_emote_utility(canonical) || !is_urn => canonical.to_string(),
        Some(canonical) => base_emote_urn(canonical),
    }
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
        assert_eq!(base_emote_id(&base_emote_urn("clap")), "clap");
        assert_eq!(base_emote_id("custom"), "custom");
        assert_eq!(default_emote_ids().len(), 21);
    }

    #[test]
    fn normalizes_other_clients_spellings() {
        let n = normalize_emote_id;
        let scene = |id: &str| format!("{BASE_SCENE_EMOTES_URN_PREFIX}{id}");
        assert_eq!(n(&scene("sittingchair2")), "sittingChair2");
        assert_eq!(n(&scene("throw")), "throw");
        assert_eq!(n(&scene("PUNCH")), "punch");
        assert_eq!(n(&base_emote_urn("buttonfront")), "buttonFront");
        assert_eq!(n(&base_emote_urn("RAISEHAND")), base_emote_urn("raiseHand"));
        assert_eq!(n(&scene("wave")), base_emote_urn("wave"));
        assert_eq!(n("sittingchair2"), "sittingChair2");
        assert_eq!(n("RaiseHand"), "raiseHand");
        assert_eq!(n("unknownid"), "unknownid");
        assert_eq!(n(&scene("brandnew")), scene("brandnew"));
        let onchain = "urn:decentraland:matic:collections-v2:0xabc:0";
        assert_eq!(n(onchain), onchain);
        let scene_file = format!("{SCENE_EMOTE_URN_PREFIX}b64-abc-hash-true");
        assert_eq!(n(&scene_file), scene_file);
        assert!(is_scene_emote_urn(&scene_file) && !is_scene_emote_urn(&scene("throw")));
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
