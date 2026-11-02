//! `AvatarModifierArea.excludeIds` matching.

/// Whether `avatar_id` is in `exclude_ids`. Ethereum addresses compare case-insensitively and
/// with or without the `0x` prefix: comms delivers checksummed addresses, scenes lowercase ones.
pub fn is_excluded<S: AsRef<str>>(avatar_id: &str, exclude_ids: &[S]) -> bool {
    if avatar_id.is_empty() {
        return false;
    }
    let normalized = normalize(avatar_id);
    exclude_ids
        .iter()
        .any(|raw| normalize(raw.as_ref()) == normalized)
}

fn normalize(address: &str) -> String {
    let lower = address.to_lowercase();
    if lower.starts_with("0x") {
        lower
    } else {
        format!("0x{lower}")
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
}
