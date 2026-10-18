//! When to ask the store to show its native review card.
//!
//! The rating is self-selecting (people who hit a bug go and rate, engaged players never do), so
//! the ask is implicit, right after something good happened, and never preceded by UI of ours.
//! Three shots per install for life: the first after five sessions, the second seven days after
//! it, the third fourteen days after that. The cooldown is a floor, not a schedule: with no
//! trigger there is no shot. The outcome is not observable, so the call is what gets counted.

use std::{fs, io, path::PathBuf, sync::Mutex};

use serde::{Deserialize, Serialize};

use crate::analytics::data_definition::{SegmentEvent, SegmentEventReviewPrompted};

pub const MAX_SHOTS: u32 = 3;
pub const SESSION_GATE: u32 = 5;
const DAY: i64 = 86_400;
pub const SHOT_2_FLOOR_SECS: i64 = 7 * DAY;
pub const SHOT_3_FLOOR_SECS: i64 = 14 * DAY;
/// Beat between the player's action and the card, so it reads as a considered moment; the shell
/// waits this long and asks again before the card goes up.
pub const PROMPT_DELAY_MS: u64 = 1_200;

/// What can hold a prompt back: an app that is loading, in an error state or mid-dialog is
/// exactly when a rating prompt reads worst.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum Rail {
    Loading,
    Modal,
    DeviceSupport,
    Connection,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// The next shot is `n`; wait the beat and fire.
    Ask(u32),
    /// All shots used.
    Capped,
    /// The session gate or the floor since the last shot is not met yet.
    NotDue,
    Blocked(Rail),
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewState {
    pub sessions: u32,
    pub shots: u32,
    pub last_shot: i64,
}

pub trait StateStore: Send + Sync {
    fn load(&self) -> ReviewState;
    fn save(&self, state: &ReviewState) -> io::Result<()>;
}

pub struct FileStateStore(pub PathBuf);

impl StateStore for FileStateStore {
    fn load(&self) -> ReviewState {
        fs::read_to_string(&self.0)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    fn save(&self, state: &ReviewState) -> io::Result<()> {
        let tmp = self.0.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_string(state)?)?;
        fs::rename(tmp, &self.0)
    }
}

pub struct ReviewPolicy {
    store: Box<dyn StateStore>,
    state: Mutex<ReviewState>,
}

impl ReviewPolicy {
    pub fn new(store: Box<dyn StateStore>) -> Self {
        let state = Mutex::new(store.load());
        Self { store, state }
    }

    fn state(&self) -> std::sync::MutexGuard<'_, ReviewState> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn save(&self, state: &ReviewState) {
        if let Err(error) = self.store.save(state) {
            tracing::warn!("review state not saved: {error}");
        }
    }

    /// One foreground session per launch, the same definition analytics uses.
    pub fn session_started(&self) {
        let mut state = self.state();
        state.sessions += 1;
        self.save(&state);
    }

    /// Launches counted so far, this one included once the shell reported it.
    pub fn sessions(&self) -> u32 {
        self.state().sessions
    }

    pub fn verdict(&self, now: i64, rails: &[Rail]) -> Verdict {
        let state = self.state();
        if state.shots >= MAX_SHOTS {
            return Verdict::Capped;
        }
        let due = match state.shots {
            0 => state.sessions >= SESSION_GATE,
            1 => now - state.last_shot >= SHOT_2_FLOOR_SECS,
            _ => now - state.last_shot >= SHOT_3_FLOOR_SECS,
        };
        if !due {
            return Verdict::NotDue;
        }
        match rails.first() {
            Some(rail) => Verdict::Blocked(*rail),
            None => Verdict::Ask(state.shots + 1),
        }
    }

    /// The beat passed: asks again (the app may have changed), and when it is still fine counts
    /// the shot and returns its event. Counted on the call, not on any outcome, and after the
    /// beat, so a shot the rails cancelled is not burnt.
    pub fn fire(&self, trigger: &str, now: i64, rails: &[Rail]) -> Option<(u32, SegmentEvent)> {
        let Verdict::Ask(shot) = self.verdict(now, rails) else {
            return None;
        };
        let mut state = self.state();
        state.shots = shot;
        state.last_shot = now;
        self.save(&state);
        Some((
            shot,
            SegmentEvent::ReviewPrompted(SegmentEventReviewPrompted {
                shot_id: shot,
                trigger_id: trigger.to_string(),
            }),
        ))
    }
}

/// The process-wide policy, made on first use from the core's data dir.
pub fn global(core: &crate::Core) -> &'static ReviewPolicy {
    static POLICY: std::sync::OnceLock<ReviewPolicy> = std::sync::OnceLock::new();
    POLICY.get_or_init(|| {
        ReviewPolicy::new(Box::new(FileStateStore(
            core.data_dir().join("review_state.json"),
        )))
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    #[derive(Default)]
    struct Memory(Mutex<ReviewState>);

    impl StateStore for Arc<Memory> {
        fn load(&self) -> ReviewState {
            self.0.lock().unwrap().clone()
        }
        fn save(&self, state: &ReviewState) -> io::Result<()> {
            *self.0.lock().unwrap() = state.clone();
            Ok(())
        }
    }

    fn policy(sessions: u32) -> (ReviewPolicy, Arc<Memory>) {
        let memory = Arc::new(Memory(Mutex::new(ReviewState {
            sessions,
            ..Default::default()
        })));
        (ReviewPolicy::new(Box::new(memory.clone())), memory)
    }

    #[test]
    fn the_first_shot_waits_for_the_fifth_session() {
        let (policy, _) = policy(0);
        for _ in 0..4 {
            policy.session_started();
        }
        assert_eq!(policy.verdict(1_000, &[]), Verdict::NotDue);
        policy.session_started();
        assert_eq!(policy.verdict(1_000, &[]), Verdict::Ask(1));
    }

    #[test]
    fn the_floors_are_seven_then_fourteen_days_and_three_shots_is_all() {
        let (policy, _) = policy(5);
        let t0 = 1_000_000;
        assert!(policy.fire("place_favorited", t0, &[]).is_some());
        assert_eq!(
            policy.verdict(t0 + SHOT_2_FLOOR_SECS - 1, &[]),
            Verdict::NotDue
        );
        assert_eq!(policy.verdict(t0 + SHOT_2_FLOOR_SECS, &[]), Verdict::Ask(2));
        let t1 = t0 + SHOT_2_FLOOR_SECS;
        assert!(policy.fire("place_upvoted", t1, &[]).is_some());
        assert_eq!(
            policy.verdict(t1 + SHOT_3_FLOOR_SECS - 1, &[]),
            Verdict::NotDue
        );
        let t2 = t1 + SHOT_3_FLOOR_SECS;
        assert_eq!(policy.verdict(t2, &[]), Verdict::Ask(3));
        assert!(policy.fire("wearable_claimed", t2, &[]).is_some());
        assert_eq!(policy.verdict(t2 + 365 * DAY, &[]), Verdict::Capped);
        assert!(policy.fire("place_upvoted", t2 + 365 * DAY, &[]).is_none());
    }

    #[test]
    fn a_rail_holds_the_prompt_without_burning_a_shot() {
        let (policy, memory) = policy(9);
        assert_eq!(
            policy.verdict(10, &[Rail::Loading, Rail::Modal]),
            Verdict::Blocked(Rail::Loading)
        );
        assert!(policy.fire("place_favorited", 10, &[Rail::Modal]).is_none());
        assert_eq!(memory.load().shots, 0);
        assert!(policy.fire("place_favorited", 10, &[]).is_some());
        assert_eq!(memory.load().shots, 1);
    }

    #[test]
    fn firing_counts_the_call_and_says_which_trigger_and_shot() {
        let (policy, memory) = policy(5);
        let (shot, event) = policy.fire("place_upvoted", 42, &[]).unwrap();
        assert_eq!(shot, 1);
        let SegmentEvent::ReviewPrompted(prompted) = event else {
            panic!("expected review_prompted");
        };
        assert_eq!(
            (prompted.shot_id, prompted.trigger_id.as_str()),
            (1, "place_upvoted")
        );
        assert_eq!(
            memory.load(),
            ReviewState {
                sessions: 5,
                shots: 1,
                last_shot: 42
            }
        );
    }

    #[test]
    fn the_state_survives_a_restart() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("review_state.json");
        let first = ReviewPolicy::new(Box::new(FileStateStore(path.clone())));
        for _ in 0..5 {
            first.session_started();
        }
        first.fire("place_favorited", 7, &[]).unwrap();
        let again = ReviewPolicy::new(Box::new(FileStateStore(path)));
        assert_eq!(again.verdict(8, &[]), Verdict::NotDue);
        assert_eq!(again.state().shots, 1);
    }
}
