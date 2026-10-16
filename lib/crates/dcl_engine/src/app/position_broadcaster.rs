//! Sends the local player's movement to the room on a timer, only when something changed (or as a
//! keepalive), so remote peers can interpolate without being flooded.

use godot::classes::{ITimer, Timer};
use godot::prelude::*;

use crate::fetcher::globals;
use crate::player::body::Player;

/// About 2.75 cm: 0.5 m/s at the 55 ms timer.
const POSITION_THRESHOLD: f32 = 0.0275;
/// About 0.57 degrees, in radians.
const ROTATION_THRESHOLD: f32 = 0.01;
/// About a second of nothing changing between forced sends.
const IDLE_KEEPALIVE_TICKS: u32 = 18;

/// The last values sent.
#[derive(Clone, Copy)]
pub struct Sent {
    pub position: Vector3,
    pub rotation_y: f32,
    pub jump_count: i32,
    pub is_grounded: bool,
    pub glide_state: i32,
}

impl Default for Sent {
    fn default() -> Self {
        Self {
            position: Vector3::ZERO,
            rotation_y: f32::INFINITY,
            jump_count: 0,
            is_grounded: true,
            glide_state: 0,
        }
    }
}

/// The signed shortest distance between two angles, in radians.
fn wrapped_angle_delta(a: f32, b: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    let mut d = (a - b) % tau;
    if d > std::f32::consts::PI {
        d -= tau;
    } else if d < -std::f32::consts::PI {
        d += tau;
    }
    d
}

/// Whether a send is due: something changed (zero tolerance for jump count, grounded and glide
/// state, which must go out on the next tick), or `idle_ticks` reached the keepalive.
pub fn should_send(last: &Sent, now: &Sent, idle_ticks: u32) -> bool {
    let position_changed = now.position.distance_to(last.position) >= POSITION_THRESHOLD;
    let rotation_changed =
        wrapped_angle_delta(now.rotation_y, last.rotation_y).abs() >= ROTATION_THRESHOLD;
    let anim_changed = now.jump_count != last.jump_count
        || now.is_grounded != last.is_grounded
        || now.glide_state != last.glide_state;
    position_changed || rotation_changed || anim_changed || idle_ticks + 1 >= IDLE_KEEPALIVE_TICKS
}

#[derive(GodotClass)]
#[class(base=Timer, init)]
pub struct PositionBroadcaster {
    base: Base<Timer>,
    #[export]
    player_node: Option<Gd<Player>>,
    last: Sent,
    idle_ticks: u32,
}

#[godot_api]
impl ITimer for PositionBroadcaster {
    fn ready(&mut self) {
        self.base_mut().set_wait_time(0.055);
        self.base_mut().set_autostart(true);
        let me = self.to_gd();
        self.base()
            .signals()
            .timeout()
            .connect_other(&me, |this: &mut Self| this.on_timeout());
    }
}

impl PositionBroadcaster {
    fn on_timeout(&mut self) {
        let Some(player) = self.player_node.clone().filter(|p| p.is_instance_valid()) else {
            return;
        };
        let (position, rotation_y, velocity, avatar) = {
            let p = player.bind();
            (
                p.get_broadcast_position(),
                p.get_broadcast_rotation_y(),
                player.get_velocity(),
                p.get_avatar(),
            )
        };
        let Some(avatar) = avatar else {
            godot_error!("No player avatar to broadcast");
            return;
        };
        let (walk, run, jog, rise, fall, land, jump_count, glide_state, is_grounded) = {
            let a = avatar.bind();
            (
                a.walk,
                a.run,
                a.jog,
                a.rise,
                a.fall,
                a.land,
                a.jump_count,
                a.glide_state,
                a.is_grounded,
            )
        };
        let now = Sent {
            position,
            rotation_y,
            jump_count,
            is_grounded,
            glide_state,
        };
        if !should_send(&self.last, &now, self.idle_ticks) {
            self.idle_ticks += 1;
            return;
        }
        self.idle_ticks = 0;
        globals::comms().bind_mut().broadcast_movement(
            false,
            position,
            rotation_y,
            velocity,
            walk,
            run,
            jog,
            rise,
            fall,
            land,
            jump_count,
            glide_state,
            is_grounded,
        );
        self.last = now;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(x: f32) -> Sent {
        Sent {
            position: Vector3::new(x, 0.0, 0.0),
            rotation_y: 0.0,
            ..Sent::default()
        }
    }

    #[test]
    fn nothing_changed_waits_for_the_keepalive() {
        let last = at(1.0);
        assert!(!should_send(&last, &at(1.0), 0));
        assert!(!should_send(&last, &at(1.0), IDLE_KEEPALIVE_TICKS - 2));
        assert!(should_send(&last, &at(1.0), IDLE_KEEPALIVE_TICKS - 1));
    }

    #[test]
    fn movement_and_turning_send_at_once() {
        let last = at(1.0);
        assert!(should_send(&last, &at(1.03), 0));
        assert!(!should_send(&last, &at(1.01), 0));
        let mut turned = at(1.0);
        turned.rotation_y = 0.02;
        assert!(should_send(&last, &turned, 0));
    }

    #[test]
    fn a_changed_animation_state_goes_out_on_the_next_tick() {
        let last = at(1.0);
        let mut jumped = at(1.0);
        jumped.jump_count = 1;
        assert!(should_send(&last, &jumped, 0));
        let mut airborne = at(1.0);
        airborne.is_grounded = false;
        assert!(should_send(&last, &airborne, 0));
        let mut gliding = at(1.0);
        gliding.glide_state = 2;
        assert!(should_send(&last, &gliding, 0));
    }

    #[test]
    fn the_first_send_always_happens() {
        // The initial rotation is infinite, so the wrap gives NaN and the comparison is false:
        // position decides, and a fresh player is away from the origin.
        assert!(should_send(&Sent::default(), &at(5.0), 0));
    }

    #[test]
    fn angles_wrap_the_short_way() {
        assert!(wrapped_angle_delta(3.1, -3.1).abs() < 0.1);
        assert!(wrapped_angle_delta(0.5, 0.4) > 0.0);
    }
}
