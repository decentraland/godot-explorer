//! Ordering of the PlayerEmote copies a peer reaches us with.

use crate::dcl::components::proto_components::kernel::comms::rfc4;

/// Whether `emote` is a copy of one already played from this peer (it arrives once per
/// LiveKit room); otherwise records it as the newest. LiveKit emotes order on the
/// sender-clock `timestamp` the room copies share, not on `incremental_id`: Unity sends 0,
/// or echoes the id of an emote it received. Pulse-bridged emotes have no timestamp and
/// order on the server tick carried in `incremental_id`.
pub fn is_duplicate_emote(
    emote: &rfc4::PlayerEmote,
    last_incremental_id: &mut u32,
    last_timestamp: &mut f32,
) -> bool {
    if emote.timestamp > 0.0 {
        if emote.timestamp <= *last_timestamp {
            return true;
        }
        *last_timestamp = emote.timestamp;
        return false;
    }
    // No usable ordering at all: let it through (the emote cooldown absorbs repeats).
    if emote.incremental_id == 0 {
        return false;
    }
    if emote.incremental_id <= *last_incremental_id {
        return true;
    }
    *last_incremental_id = emote.incremental_id;
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn emote(incremental_id: u32, timestamp: f32) -> rfc4::PlayerEmote {
        rfc4::PlayerEmote {
            incremental_id,
            timestamp,
            urn: "wave".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn livekit_emotes_dedup_on_timestamp() {
        let (mut last_id, mut last_ts) = (0, f32::NEG_INFINITY);
        assert!(!is_duplicate_emote(
            &emote(1, 5.0),
            &mut last_id,
            &mut last_ts
        ));
        // Same emote from the second room.
        assert!(is_duplicate_emote(
            &emote(1, 5.0),
            &mut last_id,
            &mut last_ts
        ));
        assert!(!is_duplicate_emote(
            &emote(2, 6.0),
            &mut last_id,
            &mut last_ts
        ));
        // A late copy of an older emote.
        assert!(is_duplicate_emote(
            &emote(1, 5.0),
            &mut last_id,
            &mut last_ts
        ));
        assert_eq!(last_id, 0);
    }

    #[test]
    fn unity_emotes_play_whatever_id_they_carry() {
        // Unity sends id 0, or echoes the id of an emote it received from this client.
        let (mut last_id, mut last_ts) = (0, f32::NEG_INFINITY);
        for (id, ts) in [(0, 12.5), (1, 14.0), (1, 15.0), (0, 16.0)] {
            assert!(!is_duplicate_emote(
                &emote(id, ts),
                &mut last_id,
                &mut last_ts
            ));
            assert!(is_duplicate_emote(
                &emote(id, ts),
                &mut last_id,
                &mut last_ts
            ));
        }
    }

    #[test]
    fn pulse_emotes_dedup_on_tick() {
        // The Pulse bridge stamps the server tick as incremental_id and no timestamp.
        let (mut last_id, mut last_ts) = (0, f32::NEG_INFINITY);
        assert!(!is_duplicate_emote(
            &emote(100, 0.0),
            &mut last_id,
            &mut last_ts
        ));
        assert!(is_duplicate_emote(
            &emote(100, 0.0),
            &mut last_id,
            &mut last_ts
        ));
        assert!(!is_duplicate_emote(
            &emote(130, 0.0),
            &mut last_id,
            &mut last_ts
        ));
    }

    #[test]
    fn unordered_emotes_are_never_dropped() {
        let (mut last_id, mut last_ts) = (0, f32::NEG_INFINITY);
        assert!(!is_duplicate_emote(
            &emote(0, 0.0),
            &mut last_id,
            &mut last_ts
        ));
        assert!(!is_duplicate_emote(
            &emote(0, 0.0),
            &mut last_id,
            &mut last_ts
        ));
    }
}
