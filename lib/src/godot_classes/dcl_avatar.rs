use crate::godot_classes::dcl_hashing::get_hash_number;
use godot::prelude::*;

use crate::avatars::avatar_type::DclAvatarWireFormat;
use crate::dcl::SceneId;

use super::dcl_global::DclGlobal;

// Global counter for unique avatar IDs (non-atomic since init is always on main thread)
static mut AVATAR_ID_COUNTER: u32 = 0;

#[derive(Var, GodotConvert, Export)]
#[godot(via = i32)]
pub enum AvatarMovementType {
    ExternalController = 0,
    LerpTwoPoints = 1,
}

/// Buffered remote-avatar interpolation. Renders the avatar in the PAST along
/// a small ring of recent packets — the classic networked-avatar delay
/// buffer. The previous design (lerp toward the latest packet, zero buffer)
/// cannot absorb arrival jitter: Pulse fan-out quantizes to the server tick
/// grid and UDP adds its own, so measured gaps swing 30-190 ms around the
/// nominal cadence. A zero-buffer lerp freezes on every late packet and cuts
/// short on every early one. The delay is a fixed 2x the measured interval
/// (smooth by construction) and the render clock never extrapolates past the
/// newest packet (extrapolation snap-back reads as ping-pong).
#[derive(Default)]
struct LerpState {
    /// Recent packets, oldest first. Capacity 8: the render delay tracks the
    /// worst gap in the ring, so the ring must span several minimum-sized
    /// gaps (captured minimum ~30 ms) to always cover it.
    buffer: std::collections::VecDeque<Packet>,
    /// Local clock in seconds; accumulates in `process`, never resets. All
    /// arrival timestamps are on this clock (local, monotonic — wire
    /// timestamps mix sender/server epochs and can't be compared).
    clock: f32,
    /// Parcel-update throttle for ExternalController avatars.
    factor: f32,
    /// Measured seconds between movement packets (EMA). LiveKit streams at
    /// 10 Hz, but Pulse's server tick is 50 ms with spatial LOD: tier-0 peers
    /// (<=20 m) arrive at 20 Hz, tier-2 at 5 Hz.
    packet_interval: f32,
    /// Slow EMA of the packet interval (alpha 0.15), feeds the delay TARGET.
    /// The classification EMA (alpha 0.3) is intentionally responsive, but a
    /// delay derived from it lurches the render clock back and forth on
    /// jittery streams — measured as visible ~0.13 m frame steps on the
    /// captured gap pattern.
    delay_interval: f32,
    /// Actual render delay in seconds; slewed toward the target in `process`
    /// (grow 0.2 s/s, shrink 0.02 s/s) so delay changes themselves never
    /// lurch the render clock. Asymmetric: growing late risks brief holds,
    /// shrinking fast produced visible forward jumps.
    delay: f32,
    /// Seconds since the last movement packet; sampled + reset in
    /// `set_target_position`, accumulated in `process`.
    since_last_packet: f32,
    /// EMA of per-packet horizontal speed (m/s). Smoothing kills the flicker
    /// from wire quantization (Pulse position steps are ~6.3 cm, so a slow
    /// walk arrives as alternating 0 / step-sized jumps).
    smoothed_speed: f32,
}

/// One buffered movement packet: world position, yaw, local arrival time.
#[derive(Default, Clone, Copy)]
struct Packet {
    position: Vector3,
    rotation_y: f32,
    arrival: f32,
}

const PACKET_BUFFER_CAPACITY: usize = 8;

impl LerpState {
    /// Arrival interval in seconds; defaults to the LiveKit 10 Hz cadence
    /// until two packets have been seen.
    fn interval(&self) -> f32 {
        if self.packet_interval > 0.0 {
            self.packet_interval
        } else {
            0.1
        }
    }

    /// Latest buffered position, or world origin before the first packet.
    fn target_position(&self) -> Vector3 {
        self.buffer.back().map(|p| p.position).unwrap_or_default()
    }

    /// Update both interval EMAs from a measured inter-packet gap.
    fn note_packet_gap(&mut self, measured: f32) {
        if measured <= 0.01 {
            // Same-frame duplicate packets (dual-room broadcast) would
            // collapse the EMAs.
            return;
        }
        let clamped = measured.clamp(0.03, 0.5);
        self.packet_interval = if self.packet_interval <= 0.0 {
            clamped
        } else {
            self.packet_interval * 0.7 + clamped * 0.3
        };
        self.delay_interval = if self.delay_interval <= 0.0 {
            clamped
        } else {
            self.delay_interval * 0.85 + clamped * 0.15
        };
    }

    /// Push a packet onto the ring, stamped with the current local clock.
    fn push_packet(&mut self, position: Vector3, rotation_y: f32) {
        self.buffer.push_back(Packet {
            position,
            rotation_y,
            arrival: self.clock,
        });
        while self.buffer.len() > PACKET_BUFFER_CAPACITY {
            self.buffer.pop_front();
        }
    }

    /// Delay target in seconds: 2x the slow interval EMA, covering up to a
    /// 2x late packet without starving.
    fn delay_target(&self) -> f32 {
        let interval = if self.delay_interval > 0.0 {
            self.delay_interval
        } else {
            self.interval()
        };
        (2.0 * interval).clamp(0.05, 0.5)
    }

    /// Slew the actual delay toward the target; call once per frame.
    fn advance_delay(&mut self, dt: f32) {
        let target = self.delay_target();
        if self.delay <= 0.0 {
            self.delay = target;
        } else if target > self.delay {
            self.delay = (self.delay + 0.2 * dt).min(target);
        } else {
            self.delay = (self.delay - 0.02 * dt).max(target);
        }
    }

    /// Interpolated (position, yaw) for the current render clock.
    fn render_transform(&self) -> (Vector3, f32) {
        let Some(newest) = self.buffer.back().copied() else {
            return (Vector3::ZERO, 0.0);
        };
        if self.buffer.len() == 1 {
            return (newest.position, newest.rotation_y);
        }
        let render_t = self.clock - self.delay.max(0.05);
        // Newest-first search for the segment containing render_t; falls back
        // to the newest segment (starvation beyond the ring) or the oldest
        // (render_t older than the whole ring).
        let mut i = self.buffer.len() - 2;
        while i > 0 && self.buffer[i].arrival > render_t {
            i -= 1;
        }
        let a = self.buffer[i];
        let b = self.buffer[i + 1];
        let seg_dt = (b.arrival - a.arrival).max(0.005);
        // Hard clamp: NO extrapolation. Extrapolating past the newest packet
        // renders future positions that snap back when the next packet arrives
        // (visible ping-pong, worse than a brief hold at the last known pose).
        let t = ((render_t - a.arrival) / seg_dt).clamp(0.0, 1.0);
        (
            a.position.lerp(b.position, t),
            lerp_angle(a.rotation_y, b.rotation_y, t),
        )
    }
}

#[derive(GodotClass)]
#[class(base=Node3D)]
pub struct DclAvatar {
    #[var(get)]
    unique_id: u32,

    #[var]
    avatar_data: Gd<DclAvatarWireFormat>,

    #[var]
    avatar_name: GString,

    #[var]
    pub blocked: bool,

    #[export]
    movement_type: AvatarMovementType,

    #[var]
    current_parcel_scene_id: i32,

    #[var]
    current_parcel_position: Vector2i,

    #[export]
    walk: bool,
    #[export]
    run: bool,
    #[export]
    jog: bool,
    #[export]
    rise: bool,
    #[export]
    fall: bool,
    #[export]
    land: bool,

    // Multi-jump + gliding state, driven by player.gd (local) or the remote-
    // movement decoder. Consumed by avatar.gd edge detection. glide_state
    // values: 0 CLOSED, 1 OPENING, 2 GLIDING, 3 CLOSING.
    #[export]
    jump_count: i32,
    #[export]
    glide_state: i32,
    #[export]
    is_grounded: bool,

    lerp_state: LerpState,
    base: Base<Node3D>,
}

#[godot_api]
impl INode3D for DclAvatar {
    fn init(base: Base<Node3D>) -> Self {
        // Increment and get the next unique ID (safe since init is always on main thread)
        let unique_id = unsafe {
            let id = AVATAR_ID_COUNTER;
            AVATAR_ID_COUNTER += 1;
            id
        };

        Self {
            unique_id,
            movement_type: AvatarMovementType::ExternalController,
            current_parcel_scene_id: SceneId::INVALID.0,
            current_parcel_position: Vector2i::new(i32::MAX, i32::MAX),
            lerp_state: Default::default(),
            base,
            walk: false,
            run: false,
            jog: false,
            rise: false,
            fall: false,
            land: false,
            jump_count: 0,
            glide_state: 0,
            is_grounded: true,
            avatar_data: DclAvatarWireFormat::from_gd(Default::default()),
            avatar_name: "".into(),
            blocked: false,
        }
    }
}

// Taken from https://github.com/decentraland/unity-explorer/blob/2ec0987a4c880f8723478329a3f2f71e373db288/Explorer/Assets/Scenes/Main.unity#L563
const NICKNAME_COLORS: [Color; 23] = [
    Color::from_rgb(0.67138505, 0.38714847, 0.9433962),
    Color::from_rgb(0.8324557, 0.6273585, 1.0),
    Color::from_rgb(0.8716914, 0.3820755, 1.0),
    Color::from_rgb(1.0, 0.2028302, 0.9783837),
    Color::from_rgb(1.0, 0.3537736, 0.92354745),
    Color::from_rgb(1.0, 0.5235849, 0.79682314),
    Color::from_rgb(1.0, 0.7019608, 0.9433204),
    Color::from_rgb(1.0, 0.28773582, 0.30953965),
    Color::from_rgb(1.0, 0.4292453, 0.46791336),
    Color::from_rgb(1.0, 0.6367924, 0.66624165),
    Color::from_rgb(1.0, 0.5053185, 0.08018869),
    Color::from_rgb(1.0, 0.65705246, 0.0),
    Color::from_rgb(1.0, 0.8548728, 0.0),
    Color::from_rgb(1.0, 0.9431928, 0.6084906),
    Color::from_rgb(0.51564926, 0.8679245, 0.0),
    Color::from_rgb(0.6194137, 0.9607843, 0.121568605),
    Color::from_rgb(0.858401, 1.0, 0.5613208),
    Color::from_rgb(0.0, 1.0, 0.7287984),
    Color::from_rgb(0.5330188, 1.0, 0.9353978),
    Color::from_rgb(0.60784316, 0.8391339, 1.0),
    Color::from_rgb(0.60784316, 0.6527446, 1.0),
    Color::from_rgb(0.48584908, 0.7057166, 1.0),
    Color::from_rgb(0.2783019, 0.7820757, 1.0),
];

#[godot_api]
impl DclAvatar {
    #[signal]
    fn change_parcel_position(parcel_position: Vector2);

    #[signal]
    fn change_scene_id(new_scene_id: i32, prev_scene_id: i32);

    // `mask` uses the internal convention shared with GDScript: -1 = full body
    // (absent on the wire), 0 = AvatarMask.AM_UPPER_BODY.
    #[signal]
    fn emote_triggered(id: GString, looping: bool, mask: i64);

    // Emitted once per started emote when playback ends: `interrupted` is false for
    // the natural end of a non-looping emote, true for movement cancel, explicit
    // stop, or being superseded by another emote.
    #[signal]
    fn emote_finished(id: GString, interrupted: bool, mask: i64);

    #[func]
    pub fn set_target_position(&mut self, new_target: Transform3D) {
        // Measure the real arrival cadence instead of assuming 10 Hz (see
        // LerpState::packet_interval).
        let measured = self.lerp_state.since_last_packet;
        self.lerp_state.since_last_packet = 0.0;
        self.lerp_state.note_packet_gap(measured);
        let interval = self.lerp_state.interval();

        let mut diff_xz_plane = new_target.origin - self.lerp_state.target_position();
        let y_velocity = diff_xz_plane.y / interval;
        diff_xz_plane.y = 0.0;
        let target_forward_distance = diff_xz_plane.length();
        let instant_speed = target_forward_distance / interval;
        self.lerp_state.smoothed_speed = if self.lerp_state.smoothed_speed <= 0.0 {
            instant_speed
        } else {
            self.lerp_state.smoothed_speed * 0.6 + instant_speed * 0.4
        };
        let speed = self.lerp_state.smoothed_speed;

        // Classify by SPEED, not per-packet distance: the old thresholds
        // (0.4 m / 0.65 m) were calibrated for 100 ms packets and misclassified
        // everything one class down at Pulse tier-0's 50 ms cadence.
        // Same bounds as before, expressed in m/s (distance / 0.1 s).
        self.walk = speed < 4.0 && speed > 0.1;
        self.run = speed >= 6.5;
        self.jog = !(self.walk || self.run) && speed > 0.1;
        self.rise = y_velocity > 1.0;
        self.fall = y_velocity < -1.0;
        self.land = !self.rise && !self.fall;
        self.is_grounded = self.land && self.glide_state == 0;

        // Shift the buffer: the old latest packet becomes the segment start.
        // The render clock interpolates between the two in `process`.
        let new_yaw = new_target.basis.get_euler().y;
        let first = self.lerp_state.buffer.is_empty();
        self.lerp_state.push_packet(new_target.origin, new_yaw);
        if first {
            // First packet: no segment yet — snap, interpolating from a zeroed
            // state would drag the avatar across the world.
            self.base_mut().set_global_position(new_target.origin);
            self.base_mut()
                .set_global_rotation(new_target.basis.get_euler());
        }

        self.update_parcel_position(self.lerp_state.target_position());
    }

    /// Instant reposition (teleport): place the avatar at the target with no interpolation —
    /// lerping across a discontinuous jump would drag the avatar through the world.
    #[func]
    pub fn snap_to_position(&mut self, new_target: Transform3D) {
        self.walk = false;
        self.run = false;
        self.jog = false;
        self.rise = false;
        self.fall = false;
        self.land = true;
        self.is_grounded = self.glide_state == 0;

        let target_rotation_y = new_target.basis.get_euler().y;
        self.lerp_state.buffer.clear();
        self.lerp_state
            .push_packet(new_target.origin, target_rotation_y);
        self.lerp_state.smoothed_speed = 0.0;
        self.lerp_state.since_last_packet = 0.0;

        self.base_mut()
            .set_global_rotation(new_target.basis.get_euler());
        self.base_mut().set_global_position(new_target.origin);

        self.update_parcel_position(new_target.origin);
    }

    // This function is called when a parcel scene is created,
    //  it handles the corner case where the avatar is already in the parcel
    //  that is being created
    pub fn on_parcel_scenes_changed(&mut self) {
        let godot_parcel_position = self.base().get_global_position() / 16.0;
        let parcel_position = Vector2i::new(
            f32::floor(godot_parcel_position.x) as i32,
            f32::floor(-godot_parcel_position.z) as i32,
        );

        let scene_runner = DclGlobal::singleton().bind().get_scene_runner();
        let scene_id: i32 = scene_runner
            .bind()
            .get_scene_id_by_parcel_position(parcel_position);
        let prev_scene_id = self.current_parcel_scene_id;

        if prev_scene_id != scene_id {
            self.current_parcel_scene_id = scene_id;
            self.base_mut().call_deferred(
                "emit_signal",
                &[
                    "change_scene_id".to_variant(),
                    scene_id.to_variant(),
                    prev_scene_id.to_variant(),
                ],
            );
        }
    }

    /// Drive air state from the sender's velocity: `grounded_gate` suppresses
    /// rise/fall while grounded (wire value when available, local `land`
    /// otherwise). The previous local estimate (per-packet dy / measured
    /// arrival interval) spiked on wire position quantization + arrival
    /// jitter and replayed Jump_Fall -> Jump_End on grounded peers — the
    /// remote "bounce".
    pub fn apply_wire_air_state(&mut self, grounded_gate: bool, velocity_y: f32) {
        self.rise = !grounded_gate && velocity_y > 1.0;
        self.fall = !grounded_gate && velocity_y < -1.0;
        self.land = !self.rise && !self.fall;
    }

    // Applies authoritative movement state from the wire (remote avatars) to
    // the DclAvatar fields consumed by avatar.gd's animation edge detection.
    // Air state uses the sender's PHYSICS velocity (see apply_wire_air_state).
    pub fn apply_wire_movement_state(
        &mut self,
        jump_count: i32,
        glide_state: i32,
        is_grounded: bool,
        velocity_y: f32,
    ) {
        self.jump_count = jump_count;
        self.glide_state = glide_state;
        self.is_grounded = is_grounded;
        self.apply_wire_air_state(is_grounded, velocity_y);
    }

    #[func]
    pub fn update_parcel_position(&mut self, position: Vector3) -> bool {
        let godot_parcel_position = position / 16.0;
        let parcel_position = Vector2i::new(
            f32::floor(godot_parcel_position.x) as i32,
            f32::floor(-godot_parcel_position.z) as i32,
        );

        if self.current_parcel_position != parcel_position {
            self.current_parcel_position = parcel_position;
            self.base_mut().call_deferred(
                "emit_signal",
                &[
                    "change_parcel_position".to_variant(),
                    parcel_position.to_variant(),
                ],
            );

            let scene_runner = DclGlobal::singleton().bind().get_scene_runner();
            let scene_id: i32 = scene_runner
                .bind()
                .get_scene_id_by_parcel_position(parcel_position);

            if self.current_parcel_scene_id != scene_id {
                let prev_scene_id = self.current_parcel_scene_id;
                self.current_parcel_scene_id = scene_id;
                self.base_mut().call_deferred(
                    "emit_signal",
                    &[
                        "change_scene_id".to_variant(),
                        scene_id.to_variant(),
                        prev_scene_id.to_variant(),
                    ],
                );
                return true;
            }
        } else if self.current_parcel_scene_id == SceneId::INVALID.0 {
            let scene_runner = DclGlobal::singleton().bind().get_scene_runner();
            let scene_id: i32 = scene_runner
                .bind()
                .get_scene_id_by_parcel_position(parcel_position);

            if scene_id != SceneId::INVALID.0 {
                let prev_scene_id = self.current_parcel_scene_id;
                self.current_parcel_scene_id = scene_id;
                self.base_mut().call_deferred(
                    "emit_signal",
                    &[
                        "change_scene_id".to_variant(),
                        scene_id.to_variant(),
                        prev_scene_id.to_variant(),
                    ],
                );
                return true;
            }
        }
        false
    }

    #[func]
    fn process(&mut self, dt: f64) {
        match self.movement_type {
            AvatarMovementType::ExternalController => {
                self.lerp_state.factor += dt as f32;
                if self.lerp_state.factor > 0.1 {
                    self.update_parcel_position(self.base().get_global_position());
                }
            }
            AvatarMovementType::LerpTwoPoints => {
                let dt = dt as f32;
                self.lerp_state.clock += dt;
                self.lerp_state.since_last_packet += dt;
                self.lerp_state.advance_delay(dt);

                if !self.lerp_state.buffer.is_empty() {
                    // Render ~1.25 packet intervals in the past (see LerpState docs).
                    let (new_position, new_rotation_y) = self.lerp_state.render_transform();

                    self.base_mut().set_global_position(new_position);
                    self.base_mut()
                        .set_global_rotation(Vector3::new(0.0, new_rotation_y, 0.0));
                }

                if self.lerp_state.since_last_packet > (1.5 * self.lerp_state.interval()).max(0.3)
                    && (self.walk || self.jog || self.run)
                    && !self.rise
                    && !self.fall
                {
                    // Decay ground locomotion to idle on stream silence. ABSOLUTE time,
                    // not factor-scaled: a LiveKit peer standing still keeps a 1 Hz
                    // keepalive that inflates the interval EMA — a factor-scaled decay
                    // (3 x interval) would land past the keepalive period and never fire,
                    // leaving the avatar walking in place (~7 s of moonwalk) while the
                    // speed EMA drains one packet at a time. 1.5x interval fires before
                    // the 1 s keepalive; the 300 ms floor keeps pre-change behavior at
                    // 10-20 Hz streams and tolerates one late/lost packet.
                    //
                    // Ground locomotion ONLY, matching Unity (RemotePlayerAnimationSystem):
                    // air state (rise/fall) latches on silence — a mid-air peer keeps its air
                    // pose during a packet-loss gap instead of popping to a grounded stance.
                    self.walk = false;
                    self.jog = false;
                    self.run = false;
                    self.land = true;
                    // Drain the speed EMA too: otherwise the next 1 Hz keepalive
                    // (zero-distance packet) still reads a stale speed > walk threshold
                    // and re-latches walk — the moonwalk would come back.
                    self.lerp_state.smoothed_speed = 0.0;
                }
            }
        }
    }

    #[func]
    pub fn get_nickname_color(nickname: GString) -> Color {
        let hash = get_hash_number(nickname.to_string(), 0, NICKNAME_COLORS.len() as i32 - 1);
        NICKNAME_COLORS[hash as usize]
    }

    #[func]
    pub fn set_blocked_and_hidden(&mut self, value: bool) {
        self.blocked = value;
        // Call the GDScript set_hidden method
        self.base_mut().call("set_hidden", &[value.to_variant()]);
    }
}

/// Interpolate an angle in radians along the shortest arc.
fn lerp_angle(from: f32, to: f32, weight: f32) -> f32 {
    let mut diff = to - from;
    while diff < -std::f32::consts::PI {
        diff += 2.0 * std::f32::consts::PI;
    }
    while diff > std::f32::consts::PI {
        diff -= 2.0 * std::f32::consts::PI;
    }
    from + diff * weight
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    #[test]
    fn lerp_angle_takes_shortest_arc() {
        // 10° -> 350° must go backwards through 0°, not the long way around.
        let from = 10.0_f32.to_radians();
        let to = 350.0_f32.to_radians();
        let mid = lerp_angle(from, to, 0.5);
        assert!(mid.abs() < 1e-6, "mid was {mid}");
        assert_eq!(lerp_angle(from, to, 0.0), from);
        let end = lerp_angle(from, to, 1.0);
        let mut delta = end - to;
        while delta > PI {
            delta -= 2.0 * PI;
        }
        while delta < -PI {
            delta += 2.0 * PI;
        }
        assert!(delta.abs() < 1e-6);
    }

    #[test]
    fn interval_defaults_to_livekit_cadence() {
        assert_eq!(LerpState::default().interval(), 0.1);
        let mut state = LerpState::default();
        state.packet_interval = 0.05; // Pulse tier-0
        assert_eq!(state.interval(), 0.05);
    }
}

// godot::test itests must live in a non-cfg(test) module so they compile into
// the extension and register with the Godot test runner (pattern: billboard.rs).
mod itest {
    use super::*;

    /// Regression test for the REMOTE landing bounce: air state must come from
    /// wire data (sender velocity + is_grounded), never from the local
    /// dy/interval estimate. The key case: a grounded peer whose landing-tick
    /// sample still carries downward velocity must NOT flicker `fall` — that
    /// replayed Jump_Fall -> Jump_End every packet (the bounce).
    #[godot::test::itest]
    fn itest_wire_air_state_no_landing_replay(ctx: &crate::framework::TestContext) {
        let mut avatar = DclAvatar::new_alloc();
        ctx.scene_tree
            .clone()
            .add_child(&avatar.clone().upcast::<Node>());

        // Grounded with a downward-velocity sample: no air state at all.
        avatar
            .bind_mut()
            .apply_wire_movement_state(0, 0, true, -8.0);
        assert!(!avatar.bind().fall, "grounded peer flickered fall");
        assert!(!avatar.bind().rise);
        assert!(avatar.bind().land);
        assert!(avatar.bind().is_grounded);

        // Airborne: rise, fall, and apex (near-zero velocity stays airborne,
        // no grounded pop).
        avatar
            .bind_mut()
            .apply_wire_movement_state(1, 0, false, 6.0);
        assert!(avatar.bind().rise);
        assert!(!avatar.bind().fall);
        assert!(!avatar.bind().land);
        avatar
            .bind_mut()
            .apply_wire_movement_state(1, 0, false, -6.0);
        assert!(avatar.bind().fall);
        assert!(!avatar.bind().rise);
        avatar
            .bind_mut()
            .apply_wire_movement_state(1, 0, false, 0.0);
        assert!(!avatar.bind().rise);
        assert!(!avatar.bind().fall);
        assert!(!avatar.bind().is_grounded);
    }

    /// E2E regression test for #2734 (remote avatars rendering as discrete
    /// frames): a real `DclAvatar` node in the scene tree receives a jittered
    /// 10 Hz packet stream (gap pattern captured live from Pulse) while
    /// `process` runs at 30 fps. The pre-fix zero-buffer lerp visibly froze /
    /// cut short on this exact input; the buffered interpolator must keep the
    /// per-frame render step close to the ideal continuous step.
    #[godot::test::itest]
    fn itest_remote_avatar_smooth_under_pulse_jitter(ctx: &crate::framework::TestContext) {
        let mut avatar = DclAvatar::new_alloc();
        ctx.scene_tree
            .clone()
            .add_child(&avatar.clone().upcast::<Node>());
        avatar
            .bind_mut()
            .set_movement_type(AvatarMovementType::LerpTwoPoints as i32);
        // Keep update_parcel_position away from DclGlobal (not available in
        // the test runner): pretend we're settled in parcel (0,0), scene 1.
        avatar.set("current_parcel_position", &Vector2i::ZERO.to_variant());
        avatar.set("current_parcel_scene_id", &1_i32.to_variant());

        // Peer walks a circle (radius 2 m at 1 rad/s => 2 m/s) centered inside
        // parcel (0,0); packets sample it with the captured jitter.
        let gaps = [
            0.13f64, 0.07, 0.16, 0.04, 0.09, 0.19, 0.03, 0.12, 0.11, 0.06,
        ];
        let frame = 1.0f64 / 30.0;
        let mut clock = 0.0f64;
        let mut next_packet_in = 0.0f64;
        let mut gap_idx = 0usize;
        let mut prev = Vector3::ZERO;
        let mut max_step = 0.0f32;
        let mut rendered_path = 0.0f32;

        for step in 0..300usize {
            clock += frame;
            next_packet_in -= frame;
            if next_packet_in <= 0.0 {
                next_packet_in = gaps[gap_idx % gaps.len()];
                gap_idx += 1;
                let angle = clock as f32;
                let pos = Vector3::new(8.0 + 2.0 * angle.cos(), 0.0, -8.0 + 2.0 * angle.sin());
                avatar
                    .bind_mut()
                    .set_target_position(Transform3D::new(Basis::IDENTITY, pos));
            }
            avatar.bind_mut().process(frame);
            let pos = avatar.get_global_position();
            if step > 0 {
                let step_len = pos.distance_to(prev);
                max_step = max_step.max(step_len);
                rendered_path += step_len;
            }
            prev = pos;
        }

        // Ideal continuous step at 2 m/s and 30 fps is ~0.067 m. The old
        // zero-buffer lerp produced freeze-then-jump steps several times that;
        // tolerate 1.8x for interpolation edge effects.
        assert!(
            max_step < 0.12,
            "per-frame render step {max_step} m exceeds smoothness budget"
        );
        // And the avatar actually tracks the peer's 20 m path (lag, not stall).
        assert!(
            rendered_path > 12.0,
            "rendered path {rendered_path} m — avatar stalled"
        );
    }

    /// Same jitter pattern as the itest, but driving the pure LerpState with
    /// the REAL EMA update path (note_packet_gap). Debug-friendly: runs under
    /// plain cargo test without Godot.
    #[test]
    fn buffered_lerp_smooths_jittered_circle_with_emas() {
        let mut state = LerpState::default();
        let gaps = [
            0.13f32, 0.07, 0.16, 0.04, 0.09, 0.19, 0.03, 0.12, 0.11, 0.06,
        ];
        let frame = 1.0f32 / 30.0;
        let mut clock = 0.0f32;
        let mut next_packet_in = 0.0f32;
        let mut gap_idx = 0usize;
        let mut prev = Vector3::ZERO;
        let mut max_step = 0.0f32;

        for step in 0..300usize {
            clock += frame;
            state.clock += frame;
            state.since_last_packet += frame;
            state.advance_delay(frame);
            next_packet_in -= frame;
            if next_packet_in <= 0.0 {
                let gap = gaps[gap_idx % gaps.len()];
                gap_idx += 1;
                next_packet_in = gap;
                state.note_packet_gap(state.since_last_packet);
                state.since_last_packet = 0.0;
                let angle = clock;
                let pos = Vector3::new(8.0 + 2.0 * angle.cos(), 0.0, -8.0 + 2.0 * angle.sin());
                state.push_packet(pos, 0.0);
            }
            let (render, _) = state.render_transform();
            if step > 0 {
                let step_len = render.distance_to(prev);
                if step_len > max_step {
                    max_step = step_len;
                }
            }
            prev = render;
        }
        assert!(max_step < 0.12, "max_step {max_step}");
    }

    /// Feed a jittered 10 Hz packet stream (real captured gaps: 30-190 ms)
    /// through the buffered interpolator and assert the rendered motion is
    /// continuous: no teleport-backs, no freeze tails beyond the extrapolation
    /// budget, and bounded frame-to-frame speed error.
    #[test]
    fn buffered_lerp_smooths_jittered_stream() {
        let mut state = LerpState {
            packet_interval: 0.1,
            ..Default::default()
        };
        // Nominal 100 ms cadence with captured-style jitter (sums to ~100 ms
        // on average, swings like the real Pulse fan-out + UDP).
        let gaps = [
            0.13f32, 0.07, 0.16, 0.04, 0.09, 0.19, 0.03, 0.12, 0.11, 0.06,
        ];
        let peer_speed = 2.0f32; // m/s along +x
        let frame = 1.0f32 / 30.0; // 30 fps render (iOS thermal cap)

        // Peer truth: moves continuously; packets sample it at jittered times.
        let mut peer_t = 0.0f32;
        let mut next_packet_in = 0.0f32;
        let mut gap_idx = 0usize;
        let mut prev_render = Vector3::ZERO;
        let mut max_speed_err = 0.0f32;
        let mut max_backtrack = 0.0f32;

        for step in 0..300 {
            state.clock += frame;
            state.advance_delay(frame);
            peer_t += frame;
            next_packet_in -= frame;
            if next_packet_in <= 0.0 {
                let gap = gaps[gap_idx % gaps.len()];
                gap_idx += 1;
                next_packet_in = gap;
                let pos = Vector3::new(peer_t * peer_speed, 0.0, 0.0);
                state.push_packet(pos, 0.0);
            }
            let (render, _) = state.render_transform();
            if step > 0 {
                let dx = render.x - prev_render.x;
                max_backtrack = max_backtrack.max(-dx);
                let render_speed = dx / frame;
                max_speed_err = max_speed_err.max((render_speed - peer_speed).abs());
            }
            prev_render = render;
        }

        assert!(
            max_backtrack < 0.01,
            "rendered position backtracked {max_backtrack} m"
        );
        // Dead-reckoning extrapolation during the long gaps can overshoot, but
        // steady-state speed error must stay well under visible-stutter range.
        assert!(
            max_speed_err < peer_speed * 1.5,
            "render speed error {max_speed_err} m/s"
        );
        // And the avatar actually tracks the peer (lag only, no stall).
        let lag = peer_t * peer_speed - prev_render.x;
        assert!(lag > 0.1 && lag < 2.0, "final lag {lag} m");
    }
}
