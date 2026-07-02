//! Pure de-duplication / "is this a new event?" decisions for incoming comms
//! packets.
//!
//! These functions are deliberately free of any Godot (`Gd<...>`) or protobuf
//! types so they can be unit-tested with plain `cargo test` — the surrounding
//! [`crate::comms::adapter::message_processor::MessageProcessor`] is coupled to
//! the Godot runtime and cannot be. The peer-re-broadcast / dual-room semantics
//! these encode are the historically bug-prone part of the comms layer (e.g. the
//! emote `incremental_id == 0` drop), so they get focused coverage here.

use std::cmp::Ordering;

/// Total-ordering comparison for `f64` timestamps that treats `NaN` as the
/// largest value (sorts last) and distinguishes `-0.0`/`0.0` consistently.
pub fn compare_f64(a: &f64, b: &f64) -> Ordering {
    match (a.is_nan(), b.is_nan()) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        (false, false) => a.total_cmp(b),
    }
}

/// Decide whether an incoming `PlayerEmote` is a NEW emote trigger that should be
/// played, versus a duplicate/stale re-broadcast that should be dropped.
///
/// Peers continuously re-broadcast their *current* emote, and every packet also
/// arrives twice (dual-room broadcasting), so naive "always play" would spam.
/// Clients also disagree on how they signal a new emote:
///  - the godot client bumps `incremental_id` per trigger (starting at 1) and
///    re-broadcasts with the SAME id;
///  - web / Foundation clients leave `incremental_id == 0` and signal a new emote
///    only by changing the `urn`, with an ever-increasing `timestamp`.
///
/// The previous logic (`incremental_id <= last`, default 0) dropped EVERY emote
/// from the second class because `0 <= 0` — the "emotes don't propagate" bug.
///
/// `last` is the last *played* emote for this peer as `(urn, timestamp,
/// incremental_id)`, or `None` if the peer hasn't emoted yet. Play when the urn
/// changes OR the incremental_id advances, rejecting stale/older re-broadcasts by
/// timestamp.
pub fn emote_should_play(
    last: Option<&(String, f32, u32)>,
    incoming_urn: &str,
    incoming_timestamp: f32,
    incoming_incremental_id: u32,
) -> bool {
    match last {
        None => true,
        Some((last_urn, last_ts, last_id)) => {
            incoming_timestamp >= *last_ts
                && (incoming_urn != last_urn.as_str() || incoming_incremental_id > *last_id)
        }
    }
}

/// Movement / MovementCompressed dedup: only strictly-newer timestamps are
/// processed. `last_ts` starts at `f32::NEG_INFINITY` so the first packet from a
/// peer always passes; duplicate dual-room copies (equal timestamp) are dropped.
pub fn movement_is_newer(last_ts: f32, incoming_ts: f32) -> bool {
    incoming_ts > last_ts
}

/// Chat dedup: process only chats strictly newer than the last seen timestamp
/// for that sender (`None` = nothing seen yet).
///
/// A `NaN` incoming timestamp is never "newer". This guards a latent edge bug:
/// [`compare_f64`] sorts `NaN` as the *largest* value, so without this guard a
/// single malformed `NaN`-timestamp chat would be accepted and then stored as
/// `last_ts`, causing every subsequent (real, smaller) timestamp from that
/// sender to compare as older and be dropped — silencing the peer's chat.
pub fn chat_is_newer(last_ts: Option<f64>, incoming_ts: f64) -> bool {
    if incoming_ts.is_nan() {
        return false;
    }
    match last_ts {
        None => true,
        Some(last) => compare_f64(&incoming_ts, &last) == Ordering::Greater,
    }
}

/// Decide whether to fetch a peer's profile after an `AnnounceProfileVersion`:
/// only when the announced version is strictly newer than what we have, we
/// aren't already fetching it, and fetching isn't currently banned (after
/// repeated failures).
pub fn profile_should_fetch(
    current_version: u32,
    announced_version: u32,
    fetch_attempted: bool,
    is_banned: bool,
) -> bool {
    announced_version > current_version && !fetch_attempted && !is_banned
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to build a "last played emote" tuple.
    fn last(urn: &str, ts: f32, id: u32) -> (String, f32, u32) {
        (urn.to_string(), ts, id)
    }

    // ---- emote_should_play: the regression table for the id==0 drop bug ----

    #[test]
    fn emote_first_ever_always_plays_even_with_id_zero() {
        // Web/Foundation clients send id=0; the very first one must play.
        assert!(emote_should_play(None, "wave", 100.0, 0));
        // godot clients start at id=1.
        assert!(emote_should_play(None, "wave", 100.0, 1));
    }

    #[test]
    fn emote_id_zero_same_urn_rebroadcast_is_dropped() {
        // Same emote re-broadcast with a newer timestamp but unchanged urn/id=0
        // must NOT replay (otherwise the avatar restarts the emote constantly).
        let l = last("wave", 100.0, 0);
        assert!(!emote_should_play(Some(&l), "wave", 102.0, 0));
        assert!(!emote_should_play(Some(&l), "wave", 100.0, 0)); // identical dual-room copy
    }

    #[test]
    fn emote_id_zero_urn_switch_replays() {
        // Switching to a different emote (still id=0) must play.
        let l = last("wave", 100.0, 0);
        assert!(emote_should_play(Some(&l), "kiss", 105.0, 0));
    }

    #[test]
    fn emote_id_zero_urn_switch_and_back_replays() {
        // wave -> kiss -> wave (all id=0, increasing ts): each switch replays.
        let l = last("kiss", 110.0, 0);
        assert!(emote_should_play(Some(&l), "wave", 120.0, 0));
    }

    #[test]
    fn emote_id_bump_same_urn_replays() {
        // godot-style: same urn, higher incremental_id => a new trigger, replay.
        let l = last("wave", 100.0, 1);
        assert!(emote_should_play(Some(&l), "wave", 101.0, 2));
    }

    #[test]
    fn emote_id_bump_same_id_same_urn_is_dropped() {
        // godot re-broadcast of the current emote: same id, same urn => drop.
        let l = last("wave", 100.0, 5);
        assert!(!emote_should_play(Some(&l), "wave", 101.0, 5));
    }

    #[test]
    fn emote_stale_older_timestamp_is_dropped() {
        // A late/stale re-broadcast of a previous emote (older ts, different urn,
        // lower id) arriving after a newer one must be ignored.
        let l = last("kiss", 110.0, 10);
        assert!(!emote_should_play(Some(&l), "wave", 100.0, 1));
    }

    // ---- movement_is_newer ----

    #[test]
    fn movement_first_packet_passes() {
        assert!(movement_is_newer(f32::NEG_INFINITY, 0.0));
        assert!(movement_is_newer(f32::NEG_INFINITY, -50.0));
    }

    #[test]
    fn movement_equal_timestamp_dropped_newer_passes() {
        assert!(!movement_is_newer(5.0, 5.0)); // dual-room duplicate
        assert!(!movement_is_newer(5.0, 4.9)); // out of order
        assert!(movement_is_newer(5.0, 5.1));
    }

    // ---- chat_is_newer ----

    #[test]
    fn chat_first_message_passes() {
        assert!(chat_is_newer(None, 1.0));
    }

    #[test]
    fn chat_equal_or_older_dropped_newer_passes() {
        assert!(!chat_is_newer(Some(10.0), 10.0));
        assert!(!chat_is_newer(Some(10.0), 9.0));
        assert!(chat_is_newer(Some(10.0), 11.0));
    }

    #[test]
    fn chat_nan_never_newer() {
        assert!(!chat_is_newer(Some(10.0), f64::NAN));
    }

    #[test]
    fn compare_f64_orders_and_handles_nan() {
        assert_eq!(compare_f64(&1.0, &2.0), Ordering::Less);
        assert_eq!(compare_f64(&2.0, &1.0), Ordering::Greater);
        assert_eq!(compare_f64(&1.0, &1.0), Ordering::Equal);
        assert_eq!(compare_f64(&f64::NAN, &1.0), Ordering::Greater);
        assert_eq!(compare_f64(&1.0, &f64::NAN), Ordering::Less);
        assert_eq!(compare_f64(&f64::NAN, &f64::NAN), Ordering::Equal);
    }

    // ---- profile_should_fetch ----

    #[test]
    fn profile_fetch_only_for_newer_unattempted_unbanned() {
        assert!(profile_should_fetch(0, 1, false, false)); // newer, fresh
        assert!(!profile_should_fetch(5, 5, false, false)); // same version
        assert!(!profile_should_fetch(5, 4, false, false)); // older
        assert!(!profile_should_fetch(0, 1, true, false)); // already fetching
        assert!(!profile_should_fetch(0, 1, false, true)); // banned
    }
}
