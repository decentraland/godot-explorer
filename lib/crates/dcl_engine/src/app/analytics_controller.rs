//! The app's side of the analytics pipeline: the EULA consent gate for returning users, the
//! Firebase `login_success` on a wallet connection, the guest wallet creation outcome, and the
//! one-shot Firebase `first_move_in_world` after the player's first real movement.

use godot::classes::{RefCounted, Timer};
use godot::prelude::*;

use crate::fetcher::globals;
use crate::godot_classes::dcl_global::DclGlobal;
use crate::player::body::Player;
use crate::ui::host;

/// Horizontal velocity (m/s) above which the player has really moved: it filters physics drift and
/// small camera corrections.
const MOVE_VELOCITY_THRESHOLD: f32 = 0.5;
/// 10 Hz is plenty for detecting the first move, and cheaper than a per-frame process.
const POLL_INTERVAL: f64 = 0.1;

/// Whether the horizontal speed counts as the player's first real movement.
pub fn counts_as_movement(velocity_xz: f32) -> bool {
    velocity_xz >= MOVE_VELOCITY_THRESHOLD
}

/// Held alive by Global: a signal connection does not own a RefCounted target.
#[derive(GodotClass)]
#[class(base=RefCounted, init)]
pub struct AnalyticsController {
    base: Base<RefCounted>,
    /// Created when armed by loading_finished; freed when the event fires.
    poll_timer: Option<Gd<Timer>>,
}

#[godot_api]
impl AnalyticsController {
    /// The one-time setup: the consent gate for returning users, and the signal wiring.
    #[func]
    pub fn setup(&mut self) {
        let engine = globals::engine();
        // A returning user accepted the EULA on a prior run: open the consent gate at startup so
        // queued events ship. The Segment to Firebase link is Metrics' own business.
        if engine.bind().is_eula_accepted() {
            globals::metrics().run_deferred(|metrics| metrics.set_eula_accepted(true));
        }
        let identity = DclGlobal::singleton().bind().player_identity.clone();
        identity
            .signals()
            .wallet_connected()
            .connect(|address, _chain_id, is_guest| {
                globals::metrics()
                    .bind_mut()
                    .track_login(address.to_string(), is_guest);
            });
        identity.signals().guest_login_outcome().connect(
            |outcome, is_new_user, failure_reason, http_status, duration_ms| {
                globals::metrics().bind_mut().track_guest_wallet_creation(
                    outcome.to_string(),
                    is_new_user,
                    failure_reason.to_string(),
                    http_status,
                    duration_ms,
                );
            },
        );
        let me = self.to_gd();
        host::connect_fn("loading_started", {
            let me = me.clone();
            move |_| me.clone().bind_mut().stop_poller()
        });
        host::connect_fn("loading_finished", move |_| {
            let already_sent = globals::engine()
                .bind()
                .setting("first_move_in_world_sent".into())
                .try_to::<bool>()
                .unwrap_or(false);
            if !already_sent {
                me.clone().bind_mut().start_poller();
            }
        });
    }

    /// Stops polling the player's first move at sign-out, so the timer does not read the player
    /// body after it is freed. It restarts on the next loading.
    #[func]
    pub fn cancel_first_move_poll(&mut self) {
        self.stop_poller();
    }

    #[func]
    fn on_poll_tick(&mut self) {
        let Some(player) = globals::scene_runner()
            .bind()
            .get_player_body_node()
            .and_then(|n| n.try_cast::<Player>().ok())
        else {
            return;
        };
        if !counts_as_movement(player.bind().actual_velocity_xz) {
            return;
        }
        // Persist the one-shot flag only when Firebase actually logged the event (Android with the
        // EULA accepted). On other platforms or before the EULA, polling still stops for this
        // session but the flag stays false, so a future session retries.
        if globals::metrics().bind_mut().track_first_move_in_world() {
            globals::engine().bind().set_setting(
                "first_move_in_world_sent".into(),
                true.to_variant(),
                true,
            );
        }
        self.stop_poller();
    }
}

impl AnalyticsController {
    /// The timer lives under Global, sharing its lifetime. A no-op while one runs.
    fn start_poller(&mut self) {
        if self.poll_timer.is_some() {
            return;
        }
        let mut timer = Timer::new_alloc();
        timer.set_name("FirstMovePoller");
        timer.set_wait_time(POLL_INTERVAL);
        timer.set_one_shot(false);
        timer.set_autostart(true);
        timer
            .signals()
            .timeout()
            .connect_other(&self.to_gd(), |this: &mut Self| this.on_poll_tick());
        let attached = timer.clone();
        DclGlobal::singleton().run_deferred_gd(move |mut global| global.add_child(&attached));
        self.poll_timer = Some(timer);
    }

    fn stop_poller(&mut self) {
        if let Some(mut timer) = self.poll_timer.take() {
            if timer.is_instance_valid() {
                timer.queue_free();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::counts_as_movement;

    #[test]
    fn drift_is_not_movement() {
        assert!(!counts_as_movement(0.0));
        assert!(!counts_as_movement(0.49));
        assert!(counts_as_movement(0.5));
        assert!(counts_as_movement(6.0));
    }
}
