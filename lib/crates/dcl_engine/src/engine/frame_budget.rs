//! Shared per-frame budget for heavy main-thread steps (avatar assembly, emote merges, glTF
//! realize, video surface init). `acquire(..).await` right before the step: it returns at once
//! while this frame has room, otherwise in a later frame.
//!
//! Room: the first step of a frame always runs; more run while fewer than MAX_STEPS ran and less
//! than BUDGET_USEC passed since the frame's first step. Priority callers (local player, previews,
//! scene models) skip the step cap and are served first. Waiters alternate between "oldest build
//! first" (avatars finish one after another) and plain arrival order (nothing starves). A waiter
//! whose owner was freed is dropped and resumes at once, so its caller finds the owner gone.

use std::cell::RefCell;
use std::collections::HashMap;

use godot::classes::{Engine, Object, SceneTree, Time};
use godot::prelude::*;
use tokio::sync::oneshot;

use super::local_task::defer;
use crate::godot_classes::dcl_global::DclGlobal;

pub const BUDGET_USEC: u64 = 2_000;
pub const MAX_STEPS: u32 = 1;
/// While the loading screen is up, throughput matters more than frame time.
pub const LOADING_BUDGET_USEC: u64 = 12_000;
pub const LOADING_MAX_STEPS: u32 = 16;

struct Ticket {
    owner: Option<InstanceId>,
    order: u64,
    arrival: u64,
    priority: bool,
    grant: oneshot::Sender<()>,
}

#[derive(Default)]
struct Budget {
    loading: bool,
    queue: Vec<Ticket>,
    frame: Option<u64>,
    frame_start_usec: u64,
    steps: u32,
    seq: u64,
    /// Owner -> order of its current build (see `begin_build`).
    build_order: HashMap<InstanceId, u64>,
    driver: bool,
}

impl Budget {
    fn roll_frame(&mut self, frame: u64, now: u64) {
        if self.frame != Some(frame) {
            self.frame = Some(frame);
            self.steps = 0;
            self.frame_start_usec = now;
        }
    }

    fn has_time(&self, now: u64) -> bool {
        let budget = if self.loading {
            LOADING_BUDGET_USEC
        } else {
            BUDGET_USEC
        };
        now.saturating_sub(self.frame_start_usec) < budget
    }

    fn has_room(&self, priority: bool, now: u64) -> bool {
        if self.steps == 0 {
            return true;
        }
        if !self.has_time(now) {
            return false;
        }
        let cap = if self.loading {
            LOADING_MAX_STEPS
        } else {
            MAX_STEPS
        };
        priority || self.steps < cap
    }

    fn has_waiting(&self, priority: bool) -> bool {
        if priority {
            self.queue.iter().any(|t| t.priority)
        } else {
            !self.queue.is_empty()
        }
    }

    /// A step right away when nobody it must yield to is waiting and the frame has room.
    fn try_take(&mut self, priority: bool, frame: u64, now: u64) -> bool {
        self.roll_frame(frame, now);
        if !self.has_waiting(priority) && self.has_room(priority, now) {
            self.steps += 1;
            return true;
        }
        false
    }

    fn enqueue(&mut self, owner: Option<InstanceId>, priority: bool) -> oneshot::Receiver<()> {
        let (grant, receiver) = oneshot::channel();
        self.seq += 1;
        let arrival = self.seq;
        let order = owner
            .and_then(|id| self.build_order.get(&id).copied())
            .unwrap_or(arrival);
        self.queue.push(Ticket {
            owner,
            order,
            arrival,
            priority,
            grant,
        });
        receiver
    }

    fn begin_build(&mut self, owner: InstanceId) {
        if !self.build_order.contains_key(&owner) {
            self.seq += 1;
            self.build_order.insert(owner, self.seq);
        }
    }

    /// The next waiter this frame has room for, after dropping the ones whose owner is gone.
    fn next_grant(
        &mut self,
        frame: u64,
        now: u64,
        alive: impl Fn(InstanceId) -> bool,
    ) -> Option<Ticket> {
        self.roll_frame(frame, now);
        let mut gone = Vec::new();
        self.queue.retain(|t| match t.owner {
            Some(id) if !alive(id) => {
                gone.push(id);
                false
            }
            _ => true,
        });
        for id in gone {
            self.build_order.remove(&id);
        }
        // Priority first; otherwise odd steps take the oldest arrival, even steps the oldest build.
        let by_arrival = self.steps % 2 == 1;
        let best = (0..self.queue.len()).reduce(|best, i| {
            if before(&self.queue[i], &self.queue[best], by_arrival) {
                i
            } else {
                best
            }
        })?;
        if !self.has_room(self.queue[best].priority, now) {
            return None;
        }
        self.steps += 1;
        Some(self.queue.remove(best))
    }
}

fn before(a: &Ticket, b: &Ticket, by_arrival: bool) -> bool {
    if a.priority != b.priority {
        return a.priority;
    }
    if a.priority || by_arrival || a.order == b.order {
        return a.arrival < b.arrival;
    }
    a.order < b.order
}

thread_local! {
    static BUDGET: RefCell<Budget> = RefCell::new(Budget::default());
}

fn with<R>(f: impl FnOnce(&mut Budget) -> R) -> R {
    BUDGET.with(|b| f(&mut b.borrow_mut()))
}

fn frame() -> u64 {
    Engine::singleton().get_process_frames()
}

fn now_usec() -> u64 {
    Time::singleton().get_ticks_usec()
}

/// Hooks the per-frame drain and the loading-screen signals. False before the tree exists.
pub fn ensure_driver() -> bool {
    if with(|b| b.driver) {
        return true;
    }
    let Some(tree) = Engine::singleton()
        .get_main_loop()
        .and_then(|l| l.try_cast::<SceneTree>().ok())
    else {
        return false;
    };
    let Some(global) = DclGlobal::try_singleton() else {
        return false;
    };
    with(|b| b.driver = true);
    tree.signals().process_frame().connect(on_process_frame);
    global
        .signals()
        .loading_started()
        .connect(|| with(|b| b.loading = true));
    global
        .signals()
        .loading_finished()
        .connect(|| with(|b| b.loading = false));
    true
}

fn on_process_frame() {
    // A load cancelled back out of the world never emits loading_finished.
    if with(|b| b.loading) && crate::world::state::current().is_none() {
        with(|b| b.loading = false);
    }
    drain();
}

/// Grants one waiter; its resume schedules the next grant, so the time its step took counts.
fn drain() {
    loop {
        let alive = |id: InstanceId| Gd::<Object>::try_from_instance_id(id).is_ok();
        let Some(ticket) = with(|b| b.next_grant(frame(), now_usec(), alive)) else {
            return;
        };
        if ticket.grant.send(()).is_ok() {
            return;
        }
        with(|b| b.steps = b.steps.saturating_sub(1));
    }
}

/// Waits for room in a frame. `owner` orders the step within its build and drops it when freed.
pub async fn acquire(owner: Option<InstanceId>, priority: bool) {
    if !ensure_driver() {
        return;
    }
    let receiver =
        with(|b| (!b.try_take(priority, frame(), now_usec())).then(|| b.enqueue(owner, priority)));
    if let Some(receiver) = receiver {
        if receiver.await.is_ok() {
            defer(drain);
        }
    }
}

/// Steps of `owner` queue by when its build began, until `end_build`.
pub fn begin_build(owner: InstanceId) {
    with(|b| b.begin_build(owner));
}

pub fn end_build(owner: InstanceId) {
    with(|b| {
        b.build_order.remove(&owner);
    });
}

/// Whether this frame still has time left, for callers that batch inside one step.
pub fn has_time() -> bool {
    with(|b| {
        let now = now_usec();
        b.roll_frame(frame(), now);
        b.has_time(now)
    })
}

/// True between `loading_started` and `loading_finished`.
pub fn loading() -> bool {
    with(|b| b.loading)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: i64) -> Option<InstanceId> {
        InstanceId::try_from_i64(n)
    }

    fn grants(budget: &mut Budget, frame: u64, now: u64) -> Vec<u64> {
        let mut out = Vec::new();
        while let Some(t) = budget.next_grant(frame, now, |_| true) {
            out.push(t.arrival);
        }
        out
    }

    #[test]
    fn the_first_step_of_a_frame_always_runs_then_the_cap_holds() {
        let mut b = Budget::default();
        assert!(b.try_take(false, 1, 0));
        assert!(!b.try_take(false, 1, 10));
        assert!(b.try_take(true, 1, 10), "priority skips the step cap");
        assert!(!b.try_take(true, 1, BUDGET_USEC + 1), "not the time budget");
        assert!(b.try_take(false, 2, BUDGET_USEC + 2), "a new frame");
    }

    #[test]
    fn loading_allows_more_steps_per_frame() {
        let mut b = Budget {
            loading: true,
            ..Default::default()
        };
        for _ in 0..LOADING_MAX_STEPS {
            assert!(b.try_take(false, 1, 0));
        }
        assert!(!b.try_take(false, 1, 0));
    }

    #[test]
    fn a_waiter_queues_behind_others_and_priority_goes_first() {
        let mut b = Budget::default();
        let _r1 = b.enqueue(id(1), false);
        let _r2 = b.enqueue(None, true);
        assert!(!b.try_take(false, 1, 0), "someone is waiting");
        let granted = grants(&mut b, 1, 0);
        assert_eq!(granted, vec![2], "priority first, then the cap");
        assert_eq!(grants(&mut b, 2, 0), vec![1]);
    }

    #[test]
    fn builds_finish_one_after_another_without_starving_arrivals() {
        let mut b = Budget {
            loading: true,
            ..Default::default()
        };
        b.begin_build(id(1).unwrap());
        b.begin_build(id(2).unwrap());
        let _a = b.enqueue(id(2), false); // arrival 3, build order 2
        let _b = b.enqueue(id(1), false); // arrival 4, build order 1
        let _c = b.enqueue(None, false); // arrival 5
                                         // Even step: oldest build; odd step: oldest arrival.
        assert_eq!(grants(&mut b, 1, 0), vec![4, 3, 5]);
    }

    #[test]
    fn a_freed_owner_is_dropped() {
        let mut b = Budget::default();
        let _r = b.enqueue(id(7), false);
        assert!(b.next_grant(1, 0, |_| false).is_none());
        assert!(b.queue.is_empty());
    }
}
