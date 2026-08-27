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

/// Buffered remote-avatar interpolation: renders ~2 packet intervals in the
/// past along a ring of recent packets (classic delay buffer). A zero-buffer
/// lerp cannot absorb the 30-190 ms arrival jitter Pulse fan-out + UDP
/// produce; never extrapolate past the newest packet (snap-back = ping-pong).
#[derive(Default)]
struct LerpState {
    /// Recent packets, oldest first. Capacity 8 spans the delay even at the
    /// captured minimum gap (~30 ms).
    buffer: std::collections::VecDeque<Packet>,
    /// Local monotonic clock (s). Wire timestamps mix sender/server epochs.
    clock: f32,
    /// Parcel-update throttle for ExternalController avatars.
    factor: f32,
    /// Inter-packet interval EMA (alpha 0.3). Pulse tier cadences: 20/10/5 Hz.
    packet_interval: f32,
    /// Slow interval EMA (alpha 0.15) feeding the delay target — the fast one
    /// lurches the render clock on jittery streams (visible frame steps).
    delay_interval: f32,
    /// Render delay, slewed in `process` (grow 0.2 s/s, shrink 0.02 s/s) so
    /// delay changes never lurch the render clock.
    delay: f32,
    /// Seconds since the last packet.
    since_last_packet: f32,
    /// Horizontal speed EMA (m/s); smooths wire-quantization flicker.
    smoothed_speed: f32,
}

/// One buffered packet: position, yaw, arrival, and the anim state that was
/// authoritative on arrival — rendered together so the AnimationTree follows
/// what the player SEES, not the network truth ~1 delay ahead.
#[derive(Default, Clone, Copy)]
struct Packet {
    position: Vector3,
    rotation_y: f32,
    arrival: f32,
    anim: AnimState,
}

/// Animation flags consumed by avatar.gd's AnimationTree conditions.
#[derive(Default, Clone, Copy)]
struct AnimState {
    walk: bool,
    jog: bool,
    run: bool,
    rise: bool,
    fall: bool,
    land: bool,
    is_grounded: bool,
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
    fn push_packet(&mut self, position: Vector3, rotation_y: f32, anim: AnimState) {
        self.buffer.push_back(Packet {
            position,
            rotation_y,
            arrival: self.clock,
            anim,
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

    /// Render-frame data: interpolated position, segment yaw, the anim state
    /// of the visible segment, and whether render_t is inside the timeline
    /// (false = starved; the silence decay owns the anim flags there).
    fn render(&self) -> (Vector3, f32, AnimState, bool) {
        let Some(newest) = self.buffer.back().copied() else {
            return (Vector3::ZERO, 0.0, AnimState::default(), false);
        };
        if self.buffer.len() == 1 {
            return (newest.position, newest.rotation_y, newest.anim, true);
        }
        let render_t = self.clock - self.delay.max(0.05);
        // Newest-first search for the segment containing render_t.
        let mut i = self.buffer.len() - 2;
        while i > 0 && self.buffer[i].arrival > render_t {
            i -= 1;
        }
        let a = self.buffer[i];
        let b = self.buffer[i + 1];
        let seg_dt = (b.arrival - a.arrival).max(0.005);
        // Hard clamp: no extrapolation past the newest packet (snap-back).
        let t = ((render_t - a.arrival) / seg_dt).clamp(0.0, 1.0);
        (
            a.position.lerp(b.position, t),
            lerp_angle(a.rotation_y, b.rotation_y, t),
            // Segment a->b's displacement produced packet b: its anim state
            // belongs to this motion.
            b.anim,
            render_t <= newest.arrival,
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
        let measured = self.lerp_state.since_last_packet;
        self.lerp_state.since_last_packet = 0.0;
        let first = self.lerp_state.buffer.is_empty();
        if !first {
            // Skipped on the first packet: the spawn-to-arrival time would
            // seed both EMAs at the 0.5 s clamp.
            self.lerp_state.note_packet_gap(measured);
        }
        let interval = self.lerp_state.interval();

        if first {
            // No previous target: the delta vs the zeroed default would read
            // as a huge speed spike in the EMA.
            self.walk = false;
            self.jog = false;
            self.run = false;
            self.rise = false;
            self.fall = false;
            self.land = true;
            self.is_grounded = self.glide_state == 0;
            self.lerp_state.smoothed_speed = 0.0;
        } else {
            let mut diff_xz_plane = new_target.origin - self.lerp_state.target_position();
            let y_velocity = diff_xz_plane.y / interval;
            diff_xz_plane.y = 0.0;
            let target_forward_distance = diff_xz_plane.length();
            let instant_speed = target_forward_distance / interval;
            // Unseeded EMA: one fast packet after idle reads 0.4x instant
            // (quantization spikes stay under the idle floor).
            self.lerp_state.smoothed_speed =
                self.lerp_state.smoothed_speed * 0.6 + instant_speed * 0.4;
            let speed = self.lerp_state.smoothed_speed;

            // Classify by SPEED: per-distance thresholds assume a fixed
            // cadence; Pulse tiers stream at 20/10/5 Hz.
            self.classify_locomotion(speed);
            self.rise = y_velocity > 1.0;
            self.fall = y_velocity < -1.0;
            self.land = !self.rise && !self.fall;
            self.is_grounded = self.land && self.glide_state == 0;
        }

        // Position, rotation and anim state all render off this ring on one
        // clock — what the player sees, not the network truth ~1 delay ahead.
        let new_yaw = new_target.basis.get_euler().y;
        let anim = self.anim_snapshot();
        self.lerp_state
            .push_packet(new_target.origin, new_yaw, anim);
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
        let anim = self.anim_snapshot();
        self.lerp_state
            .push_packet(new_target.origin, target_rotation_y, anim);
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

    /// Snapshot of the anim flags as they are RIGHT NOW (post classification
    /// and/or wire state) — stored into the packet being buffered.
    fn anim_snapshot(&self) -> AnimState {
        AnimState {
            walk: self.walk,
            jog: self.jog,
            run: self.run,
            rise: self.rise,
            fall: self.fall,
            land: self.land,
            is_grounded: self.is_grounded,
        }
    }

    /// Apply a buffered packet's anim flags to the live fields consumed by
    /// avatar.gd — called from `process` for the segment being rendered.
    fn apply_anim(&mut self, anim: &AnimState) {
        self.walk = anim.walk;
        self.jog = anim.jog;
        self.run = anim.run;
        self.rise = anim.rise;
        self.fall = anim.fall;
        self.land = anim.land;
        self.is_grounded = anim.is_grounded;
    }

    /// Re-snapshot the newest packet's anim after wire-authoritative state
    /// (apply_wire_*) updates the flags post-push.
    pub fn sync_newest_packet_anim(&mut self) {
        let anim = self.anim_snapshot();
        if let Some(newest) = self.lerp_state.buffer.back_mut() {
            newest.anim = anim;
        }
    }

    /// Classify walk/jog/run from horizontal speed (m/s). Idle floor 0.5:
    /// below it the anim plays with no visible displacement.
    fn classify_locomotion(&mut self, speed: f32) {
        const IDLE_SPEED_FLOOR: f32 = 0.5;
        self.walk = speed < 4.0 && speed > IDLE_SPEED_FLOOR;
        self.run = speed >= 6.5;
        self.jog = !(self.walk || self.run) && speed > IDLE_SPEED_FLOOR;
    }

    /// Air state from the sender's velocity; `grounded_gate` (wire when
    /// available, local `land` otherwise) suppresses rise/fall while grounded.
    /// The old dy/interval estimate spiked on quantization + jitter and
    /// replayed the landing anim (remote "bounce").
    pub fn apply_wire_air_state(&mut self, grounded_gate: bool, velocity_y: f32) {
        self.rise = !grounded_gate && velocity_y > 1.0;
        self.fall = !grounded_gate && velocity_y < -1.0;
        self.land = !self.rise && !self.fall;
    }

    // Applies wire-authoritative movement state (remote avatars). Locomotion
    // and air state use the sender's PHYSICS velocity — the local dy/interval
    // estimate spiked (bounce) and missed slow walks / short bursts.
    pub fn apply_wire_movement_state(
        &mut self,
        jump_count: i32,
        glide_state: i32,
        is_grounded: bool,
        velocity: Vector3,
    ) {
        self.jump_count = jump_count;
        self.glide_state = glide_state;
        self.is_grounded = is_grounded;
        self.apply_wire_air_state(is_grounded, velocity.y);
        let wire_speed = Vector2::new(velocity.x, velocity.z).length();
        self.apply_wire_locomotion(wire_speed);
    }

    /// Locomotion from the sender's wire velocity (EMA-smoothed, see
    /// classify_locomotion for thresholds).
    pub fn apply_wire_locomotion(&mut self, wire_speed: f32) {
        self.lerp_state.smoothed_speed = self.lerp_state.smoothed_speed * 0.6 + wire_speed * 0.4;
        let speed = self.lerp_state.smoothed_speed;
        self.classify_locomotion(speed);
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
                    // Position, rotation and anim state all render off one clock.
                    let (new_position, new_yaw, anim, on_timeline) = self.lerp_state.render();
                    self.base_mut().set_global_position(new_position);
                    self.base_mut()
                        .set_global_rotation(Vector3::new(0.0, new_yaw, 0.0));
                    if on_timeline {
                        self.apply_anim(&anim);
                    }
                }

                if self.lerp_state.since_last_packet > (1.5 * self.lerp_state.interval()).max(0.3)
                    && (self.walk || self.jog || self.run)
                    && !self.rise
                    && !self.fall
                {
                    // Decay ground locomotion to idle on stream silence.
                    // Absolute time, not interval-scaled: LiveKit's 1 Hz idle
                    // keepalive would push an interval-scaled decay past the
                    // keepalive period and it would never fire (moonwalk).
                    // Ground-only, matching Unity: air state latches on silence.
                    self.walk = false;
                    self.jog = false;
                    self.run = false;
                    self.land = true;
                    // Drain the speed EMA too or the next keepalive re-latches walk.
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

    /// Slow packets (quantization steps / threshold crossings) must NOT
    /// engage the walk anim.
    #[godot::test::itest]
    fn itest_slow_speed_stays_idle(ctx: &crate::framework::TestContext) {
        let mut avatar = DclAvatar::new_alloc();
        ctx.scene_tree
            .clone()
            .add_child(&avatar.clone().upcast::<Node>());
        avatar
            .bind_mut()
            .set_movement_type(AvatarMovementType::LerpTwoPoints as i32);
        avatar.set("current_parcel_position", &Vector2i::ZERO.to_variant());
        avatar.set("current_parcel_scene_id", &1_i32.to_variant());

        // ~3 cm per 55 ms packet ≈ 0.55 m/s instantaneous — but alternate
        // zero-movement packets like the 6.3 cm quantization does, averaging
        // ~0.27 m/s. Must never read as walking.
        let frame = 1.0f64 / 30.0;
        let mut last_pos = Vector3::new(8.0, 0.0, -8.0);
        for step in 0..120i32 {
            if step % 2 == 0 {
                let n = step / 2;
                if n % 3 == 0 && n > 0 {
                    last_pos = Vector3::new(8.0 + 0.063 * (n / 3) as f32, 0.0, -8.0);
                }
                avatar
                    .bind_mut()
                    .set_target_position(Transform3D::new(Basis::IDENTITY, last_pos));
            }
            avatar.bind_mut().process(frame);
        }
        let avatar = avatar.bind();
        assert!(!avatar.walk, "slow quantization steps read as walk");
        assert!(!avatar.jog, "slow quantization steps read as jog");
        assert!(!avatar.run, "slow quantization steps read as run");
    }

    /// Coherence: the walk anim starts when the RENDERED body moves, not when
    /// the packet arrives.
    #[godot::test::itest]
    fn itest_anim_state_tracks_rendered_motion(ctx: &crate::framework::TestContext) {
        let mut avatar = DclAvatar::new_alloc();
        ctx.scene_tree
            .clone()
            .add_child(&avatar.clone().upcast::<Node>());
        avatar
            .bind_mut()
            .set_movement_type(AvatarMovementType::LerpTwoPoints as i32);
        avatar.set("current_parcel_position", &Vector2i::ZERO.to_variant());
        avatar.set("current_parcel_scene_id", &1_i32.to_variant());

        let frame = 1.0f64 / 30.0;
        // 1 s of stationary packets (peer idle), then continuous walking.
        let mut first_move_frame: i32 = -1;
        let mut first_walk_frame: i32 = -1;
        let mut prev = Vector3::ZERO;
        for step in 0..120i32 {
            let t = step as f64 * frame;
            if step % 3 == 0 {
                // ~18 Hz packet cadence
                let pos = if t < 1.0 {
                    Vector3::new(8.0, 0.0, -8.0)
                } else {
                    Vector3::new(8.0 + 2.0 * (t - 1.0) as f32, 0.0, -8.0)
                };
                avatar
                    .bind_mut()
                    .set_target_position(Transform3D::new(Basis::IDENTITY, pos));
            }
            avatar.bind_mut().process(frame);
            let pos = avatar.get_global_position();
            if step > 0 {
                if first_move_frame < 0 && pos.distance_to(prev) > 0.005 {
                    first_move_frame = step;
                }
                if first_walk_frame < 0 && avatar.bind().walk {
                    first_walk_frame = step;
                }
            }
            prev = pos;
        }

        assert!(first_move_frame > 0, "avatar never moved");
        assert!(first_walk_frame > 0, "walk anim never engaged");
        let gap_frames = (first_walk_frame - first_move_frame).abs();
        assert!(
            gap_frames <= 2,
            "anim/motion skew {gap_frames} frames (walk@{first_walk_frame} move@{first_move_frame})"
        );
    }

    /// Remote bounce regression: air state from wire data only. A grounded
    /// peer with a downward landing-tick sample must NOT flicker `fall`.
    #[godot::test::itest]
    fn itest_wire_air_state_no_landing_replay(ctx: &crate::framework::TestContext) {
        let mut avatar = DclAvatar::new_alloc();
        ctx.scene_tree
            .clone()
            .add_child(&avatar.clone().upcast::<Node>());

        // Grounded with a downward-velocity sample: no air state at all.
        avatar
            .bind_mut()
            .apply_wire_movement_state(0, 0, true, Vector3::new(0.0, -8.0, 0.0));
        assert!(!avatar.bind().fall, "grounded peer flickered fall");
        assert!(!avatar.bind().rise);
        assert!(avatar.bind().land);
        assert!(avatar.bind().is_grounded);

        // Airborne: rise, fall, and apex (near-zero velocity stays airborne,
        // no grounded pop).
        avatar
            .bind_mut()
            .apply_wire_movement_state(1, 0, false, Vector3::new(0.0, 6.0, 0.0));
        assert!(avatar.bind().rise);
        assert!(!avatar.bind().fall);
        assert!(!avatar.bind().land);
        avatar
            .bind_mut()
            .apply_wire_movement_state(1, 0, false, Vector3::new(0.0, -6.0, 0.0));
        assert!(avatar.bind().fall);
        assert!(!avatar.bind().rise);
        avatar
            .bind_mut()
            .apply_wire_movement_state(1, 0, false, Vector3::ZERO);
        assert!(!avatar.bind().rise);
        assert!(!avatar.bind().fall);
        assert!(!avatar.bind().is_grounded);
    }

    /// Wire-velocity locomotion: slow walks animate; only sub-0.5 m/s sway
    /// is idle.
    #[godot::test::itest]
    fn itest_wire_locomotion_from_velocity(ctx: &crate::framework::TestContext) {
        let mut avatar = DclAvatar::new_alloc();
        ctx.scene_tree
            .clone()
            .add_child(&avatar.clone().upcast::<Node>());

        // Idle sway: no anim.
        avatar
            .bind_mut()
            .apply_wire_movement_state(0, 0, true, Vector3::new(0.3, 0.0, 0.0));
        assert!(!avatar.bind().walk && !avatar.bind().jog && !avatar.bind().run);
        // Genuine slow walk (1 m/s): engages — wire velocity needs no
        // interval math, so it crosses the floor on the first packets.
        avatar
            .bind_mut()
            .apply_wire_movement_state(0, 0, true, Vector3::new(1.0, 0.0, 0.0));
        avatar
            .bind_mut()
            .apply_wire_movement_state(0, 0, true, Vector3::new(1.0, 0.0, 0.0));
        assert!(avatar.bind().walk, "slow walk read as idle");
        // Jog and run boundaries.
        for _ in 0..3 {
            avatar
                .bind_mut()
                .apply_wire_movement_state(0, 0, true, Vector3::new(5.0, 0.0, 0.0));
        }
        assert!(avatar.bind().jog, "5 m/s not jog");
        for _ in 0..5 {
            avatar
                .bind_mut()
                .apply_wire_movement_state(0, 0, true, Vector3::new(7.0, 0.0, 0.0));
        }
        assert!(avatar.bind().run, "7 m/s not run");
    }

    /// #2734 regression: a real DclAvatar fed the captured Pulse gap pattern
    /// (30-190 ms) at 30 fps must keep per-frame render steps near the
    /// continuous ideal.
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

    /// Same jitter pattern as the itest through the pure LerpState + real
    /// EMA path; runs under plain cargo test.
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
                state.push_packet(pos, 0.0, AnimState::default());
            }
            let (render, _, _, _) = state.render();
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

    /// Jittered 10 Hz stream through the buffered interpolator: no
    /// teleport-backs, bounded speed error.
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
                state.push_packet(pos, 0.0, AnimState::default());
            }
            let (render, _, _, _) = state.render();
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
