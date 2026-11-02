//! What a nameplate shows by distance (#2938): the content tier, the crowd cap and the alpha
//! target. The tier bands are in meters from the camera.

use std::collections::HashMap;

use super::nameplate::FADE_END;

/// Full content (name, `#hash`, badge, mic, chat bubble) up to here.
pub const TIER_FULL_END: f32 = 8.0;
/// Name and badge up to here.
pub const TIER_BADGE_END: f32 = 12.0;
/// Name only (smaller, dimmer) up to here; it fades out until FADE_END.
pub const TIER_NAME_END: f32 = 20.0;
pub const TIER_NAME_SCALE: f32 = 0.8;
pub const TIER_NAME_ALPHA: f32 = 0.7;
/// At most this many plates (one more for an incumbent) show FULL content; the rest degrade to
/// the badge tier. The focused one and the nearest win.
pub const CROWD_MAX_FULL: usize = 7;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier {
    Full,
    Badge,
    Name,
}

/// Last frame's distance and focus per visible plate (for the crowd ranking), and the tier and
/// chat visibility last applied to its nodes, keyed by the plate's instance id.
#[derive(Default)]
pub struct TierState {
    dists: HashMap<i64, (f32, bool)>,
    applied: HashMap<i64, (Tier, bool)>,
}

impl TierState {
    /// The tier for plate `id` at `dist`: the distance bands, the crowd cap degrading excess
    /// close plates one tier, and the focused plate always FULL.
    pub fn content_tier(&self, id: i64, dist: f32, focused: bool) -> Tier {
        if focused {
            Tier::Full
        } else if dist > TIER_BADGE_END {
            Tier::Name
        } else if dist > TIER_FULL_END {
            Tier::Badge
        } else if self.crowd_allows_full(id, dist) {
            Tier::Full
        } else {
            Tier::Badge
        }
    }

    /// Whether the plate is within the CROWD_MAX_FULL best: focused plates first, then the
    /// closest; equal distances break by id. A plate already FULL gets one extra slot so
    /// borderline crowds do not flicker as people walk. Ranks against last frame's distances.
    fn crowd_allows_full(&self, id: i64, dist: f32) -> bool {
        let better = self
            .dists
            .iter()
            .filter(|&(&other_id, &(other_dist, focused))| {
                // Only close or focused plates compete for FULL slots.
                other_id != id
                    && (focused || other_dist <= TIER_FULL_END)
                    && (focused || other_dist < dist || (other_dist == dist && other_id < id))
            })
            .count();
        let was_full = matches!(self.applied.get(&id), Some((Tier::Full, _)));
        better < CROWD_MAX_FULL + usize::from(was_full)
    }

    pub fn record(&mut self, id: i64, dist: f32, focused: bool) {
        self.dists.insert(id, (dist, focused));
    }

    pub fn applied(&self, id: i64) -> Option<(Tier, bool)> {
        self.applied.get(&id).copied()
    }

    /// Stores what is applied to the plate's nodes; false when it already was.
    pub fn set_applied(&mut self, id: i64, tier: Tier, show_chat: bool) -> bool {
        self.applied.insert(id, (tier, show_chat)) != Some((tier, show_chat))
    }

    pub fn untrack(&mut self, id: i64) {
        self.dists.remove(&id);
        self.applied.remove(&id);
    }
}

/// Alpha target: full until TIER_NAME_END, fading to FADE_END; the name tier is also dimmed.
/// The focused plate ignores the fade.
pub fn tier_alpha(tier: Tier, dist: f32, focused: bool) -> f32 {
    if focused {
        return 1.0;
    }
    let fade = ((FADE_END - dist) / (FADE_END - TIER_NAME_END)).clamp(0.0, 1.0);
    if tier == Tier::Name {
        fade.min(TIER_NAME_ALPHA)
    } else {
        fade
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn crowd(ids: std::ops::Range<i64>, dist: f32) -> TierState {
        let mut state = TierState::default();
        for id in ids {
            state.record(id, dist, false);
        }
        state
    }

    #[test]
    fn bands_follow_distance() {
        let state = TierState::default();
        let tier = |dist| state.content_tier(1, dist, false);
        assert_eq!(tier(5.0), Tier::Full);
        assert_eq!(tier(8.0), Tier::Full);
        assert_eq!(tier(10.0), Tier::Badge);
        assert_eq!(tier(12.0), Tier::Badge);
        assert_eq!(tier(15.0), Tier::Name);
        assert_eq!(tier(22.0), Tier::Name);
    }

    #[test]
    fn focused_is_always_full() {
        let state = TierState::default();
        assert_eq!(state.content_tier(1, 15.0, true), Tier::Full);
        assert_eq!(state.content_tier(1, 24.0, true), Tier::Full);
    }

    #[test]
    fn a_full_crowd_degrades_the_farthest_to_badge() {
        let state = crowd(100..100 + CROWD_MAX_FULL as i64, 1.0);
        assert_eq!(state.content_tier(1, 5.0, false), Tier::Badge);
    }

    #[test]
    fn a_focused_plate_takes_a_slot() {
        let mut state = crowd(100..100 + CROWD_MAX_FULL as i64 - 1, 1.0);
        assert_eq!(state.content_tier(1, 5.0, false), Tier::Full);
        state.record(200, 1.0, true);
        assert_eq!(state.content_tier(1, 5.0, false), Tier::Badge);
    }

    #[test]
    fn distant_plates_take_no_slot() {
        let state = crowd(100..100 + CROWD_MAX_FULL as i64, 15.0);
        assert_eq!(state.content_tier(1, 5.0, false), Tier::Full);
    }

    #[test]
    fn equal_distances_break_by_id_and_the_incumbent_keeps_its_slot() {
        let state = crowd(10..10 + CROWD_MAX_FULL as i64, 5.0);
        assert_eq!(state.content_tier(1, 5.0, false), Tier::Full);
        let mut state = state;
        assert_eq!(state.content_tier(99, 5.0, false), Tier::Badge);
        state.set_applied(99, Tier::Full, true);
        assert_eq!(state.content_tier(99, 5.0, false), Tier::Full);
    }

    #[test]
    fn alpha_targets() {
        assert_eq!(tier_alpha(Tier::Full, 5.0, false), 1.0);
        assert_eq!(tier_alpha(Tier::Name, 15.0, false), TIER_NAME_ALPHA);
        assert_eq!(tier_alpha(Tier::Full, 22.5, false), 0.5);
        assert_eq!(tier_alpha(Tier::Name, 22.5, false), 0.5);
        assert_eq!(tier_alpha(Tier::Full, 24.0, true), 1.0);
        assert_eq!(tier_alpha(Tier::Full, FADE_END, false), 0.0);
    }

    #[test]
    fn set_applied_reports_changes_and_untrack_forgets() {
        let mut state = TierState::default();
        assert!(state.set_applied(1, Tier::Full, true));
        assert!(!state.set_applied(1, Tier::Full, true));
        assert!(state.set_applied(1, Tier::Full, false));
        state.untrack(1);
        assert_eq!(state.applied(1), None);
    }
}
