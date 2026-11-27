//! `AvatarModifierArea.excludeIds` matching.

/// Whether `avatar_id` is in `exclude_ids`. Ids compare case-insensitively, as on desktop (comms
/// delivers checksummed addresses, scenes lowercase ones), and an address also matches without
/// its `0x` prefix.
pub fn is_excluded<S: AsRef<str>>(avatar_id: &str, exclude_ids: &[S]) -> bool {
    if avatar_id.is_empty() {
        return false;
    }
    let normalized = normalize(avatar_id);
    exclude_ids
        .iter()
        .any(|raw| normalize(raw.as_ref()) == normalized)
}

/// Whether `id` is an Ethereum address: `0x` and 40 hex digits, any casing.
pub fn is_eth_address(id: &str) -> bool {
    let bytes = id.as_bytes();
    bytes.len() == 42
        && bytes[..2].eq_ignore_ascii_case(b"0x")
        && bytes[2..].iter().all(u8::is_ascii_hexdigit)
}

fn normalize(id: &str) -> String {
    let lower = id.to_lowercase();
    // Only an address gains the prefix: a scene id like "npc-1" is compared as sent.
    if lower.len() == 40 && lower.bytes().all(|b| b.is_ascii_hexdigit()) {
        format!("0x{lower}")
    } else {
        lower
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOWER: &str = "0xabcdef0123456789abcdef0123456789abcdef01";
    const CHECKSUM: &str = "0xAbCdEf0123456789AbCdEf0123456789aBcDeF01";
    const NO_PREFIX: &str = "abcdef0123456789abcdef0123456789abcdef01";
    const OTHER: &str = "0x1111111111111111111111111111111111111111";

    #[test]
    fn matches_across_casing_and_prefix() {
        assert!(is_excluded(LOWER, &[LOWER]));
        assert!(is_excluded(CHECKSUM, &[LOWER]));
        assert!(is_excluded(LOWER, &[CHECKSUM]));
        assert!(is_excluded(LOWER, &[NO_PREFIX]));
        assert!(is_excluded(LOWER, &[OTHER, CHECKSUM]));
    }

    #[test]
    fn does_not_match_others_or_empty() {
        assert!(!is_excluded(LOWER, &[OTHER]));
        assert!(!is_excluded("", &[LOWER]));
    }

    #[test]
    fn matches_scene_avatar_shape_ids() {
        assert!(is_excluded("npc-1", &["npc-1"]));
        assert!(!is_excluded("npc-1", &["npc-2"]));
        // Desktop lowercases both sides too, but never adds a `0x` prefix.
        assert!(is_excluded("NPC-1", &["npc-1"]));
        assert!(!is_excluded("npc-1", &["0xnpc-1"]));
    }

    #[test]
    fn only_0x_and_40_hex_digits_is_an_address() {
        assert!(is_eth_address(LOWER));
        assert!(is_eth_address(CHECKSUM));
        let multibyte = format!("€{}", "0".repeat(39));
        for id in [NO_PREFIX, "0x1234", "npc-1", "", multibyte.as_str()] {
            assert!(!is_eth_address(id), "{id}");
        }
    }
}
