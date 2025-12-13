use godot::classes::{CharacterBody3D, ICharacterBody3D, Input};
use godot::prelude::*;

/// DclPlayer is a Rust-based CharacterBody3D that handles all player movement physics.
/// The GDScript Player class extends this and handles camera, avatar visuals, and UI.
#[derive(GodotClass)]
#[class(base=CharacterBody3D)]
pub struct DclPlayer {
    // Movement speeds (m/s)
    #[export]
    walk_speed: f32,
    #[export]
    jog_speed: f32,
    #[export]
    run_speed: f32,

    // Physics
    #[export]
    gravity: f32,
    #[export]
    jump_gravity_factor: f32,
    #[export]
    long_jump_time: f32,
    #[export]
    long_jump_gravity_scale: f32,

    // Jump heights
    #[export]
    jog_jump_height: f32,
    #[export]
    run_jump_height: f32,

    // Acceleration
    #[export]
    ground_acceleration: f32,
    #[export]
    max_ground_acceleration: f32,
    #[export]
    air_acceleration: f32,
    #[export]
    max_air_acceleration: f32,
    #[export]
    acceleration_time: f32,
    #[export]
    stop_time: f32,

    // Coyote time / jump buffering
    #[export]
    coyote_time: f32,
    #[export]
    jump_cooldown: f32,

    // Slope and step settings
    #[export]
    floor_snap_length: f32,
    #[export]
    floor_max_angle_degrees: f32,

    // Internal state
    jump_time: f32,
    jump_held_time: f32,
    last_on_floor_time: f32,
    acceleration_weight: f32,
    current_direction: Vector3,
    last_position: Vector3,
    actual_velocity_xz: f32,

    // Forced position (for teleport stuck handling)
    #[var]
    forced_position: Vector3,
    #[var]
    has_forced_position: bool,

    // Movement state flags (readable by GDScript for avatar animation)
    #[var]
    is_walking: bool,
    #[var]
    is_jogging: bool,
    #[var]
    is_running: bool,
    #[var]
    is_rising: bool,
    #[var]
    is_falling: bool,
    #[var]
    is_landing: bool,
    #[var]
    is_sprinting_input: bool,

    base: Base<CharacterBody3D>,
}

#[godot_api]
impl ICharacterBody3D for DclPlayer {
    fn init(base: Base<CharacterBody3D>) -> Self {
        Self {
            // Movement speeds (matching Unity explorer)
            walk_speed: 1.5,
            jog_speed: 8.0,
            run_speed: 10.0,

            // Physics
            gravity: 10.0,
            jump_gravity_factor: 4.0,
            long_jump_time: 0.5,
            long_jump_gravity_scale: 0.5,

            // Jump heights (tuned for max reach ~2.25m)
            jog_jump_height: 0.9,
            run_jump_height: 1.35,

            // Acceleration
            ground_acceleration: 20.0,
            max_ground_acceleration: 25.0,
            air_acceleration: 40.0,
            max_air_acceleration: 50.0,
            acceleration_time: 0.5,
            stop_time: 0.0,

            // Coyote time
            coyote_time: 0.15,
            jump_cooldown: 1.5,

            // Slope and step
            floor_snap_length: 0.35,
            floor_max_angle_degrees: 46.0,

            // Internal state
            jump_time: -1.0, // Start negative so first jump works
            jump_held_time: 0.0,
            last_on_floor_time: 0.0,
            acceleration_weight: 0.0,
            current_direction: Vector3::ZERO,
            last_position: Vector3::ZERO,
            actual_velocity_xz: 0.0,

            // Forced position
            forced_position: Vector3::ZERO,
            has_forced_position: false,

            // Movement state
            is_walking: false,
            is_jogging: false,
            is_running: false,
            is_rising: false,
            is_falling: false,
            is_landing: false,
            is_sprinting_input: false,

            base,
        }
    }

    fn ready(&mut self) {
        // Apply floor settings
        let snap_length = self.floor_snap_length;
        let max_angle = self.floor_max_angle_degrees.to_radians();
        self.base_mut().set_floor_snap_length(snap_length);
        self.base_mut().set_floor_max_angle(max_angle);
    }
}

#[godot_api]
impl DclPlayer {
    /// Process physics movement - called from GDScript _physics_process
    #[func]
    pub fn process_movement(&mut self, dt: f64, has_focus: bool) {
        let dt = dt as f32;

        // Get input
        let input = Input::singleton();
        let input_dir = if has_focus {
            input.get_vector(
                StringName::from("ia_left"),
                StringName::from("ia_right"),
                StringName::from("ia_forward"),
                StringName::from("ia_backward"),
            )
        } else {
            Vector2::ZERO
        };

        // Calculate direction in world space
        let transform = self.base().get_transform();
        let direction = if input_dir.length() > 0.0 {
            (transform.basis * Vector3::new(input_dir.x, 0.0, input_dir.y)).normalized()
        } else {
            Vector3::ZERO
        };

        // Acceleration weight for smooth acceleration curve
        let target_accel_weight = if direction.length() > 0.0 { 1.0 } else { 0.0 };
        self.acceleration_weight = move_toward(
            self.acceleration_weight,
            target_accel_weight,
            dt / self.acceleration_time,
        );

        // Ground check - use is_on_floor from last move_and_slide, plus floor at y=0
        let position = self.base().get_position();
        let on_floor = self.base().is_on_floor() || position.y <= 0.01;
        self.jump_time -= dt;

        // Track time since last on floor (for coyote time)
        if on_floor {
            self.last_on_floor_time = 0.0;
        } else {
            self.last_on_floor_time += dt;
        }

        // Track jump button hold time for long jump mechanic
        let jump_pressed = has_focus && input.is_action_pressed(StringName::from("ia_jump"));
        if jump_pressed {
            self.jump_held_time += dt;
        } else {
            self.jump_held_time = 0.0;
        }

        // Can jump if on floor OR within coyote time (and not going up)
        let velocity = self.base().get_velocity();
        let can_coyote_jump = self.last_on_floor_time < self.coyote_time && velocity.y <= 0.0;

        // Sprint input
        self.is_sprinting_input =
            has_focus && input.is_action_pressed(StringName::from("ia_sprint"));
        let is_walking_input = has_focus && input.is_action_pressed(StringName::from("ia_walk"));

        // --- Jump Logic ---
        let mut just_jumped = false;
        let mut new_velocity = velocity;

        if jump_pressed && self.jump_time < 0.0 && (on_floor || can_coyote_jump) {
            // Jump height depends on sprint input
            let jump_height = if self.is_sprinting_input {
                self.run_jump_height
            } else {
                self.jog_jump_height
            };
            let effective_gravity = self.gravity * self.jump_gravity_factor;
            let jump_velocity = (2.0 * jump_height * effective_gravity).sqrt();

            new_velocity.y = jump_velocity;
            self.jump_held_time = 0.0;
            self.last_on_floor_time = self.coyote_time; // Prevent double jump
            self.is_landing = false;
            self.is_rising = true;
            self.is_falling = false;
            self.jump_time = self.jump_cooldown;
            just_jumped = true;
        }

        // --- Gravity Logic ---
        if !on_floor || just_jumped {
            if !just_jumped {
                self.is_rising = new_velocity.y > 0.3;
                self.is_falling = new_velocity.y < -0.3;
                self.is_landing = false;

                let mut effective_gravity = self.gravity;

                // Increase gravity during ascent for snappy jumps
                if new_velocity.y > 0.0 {
                    effective_gravity *= self.jump_gravity_factor;
                }

                // Reduce gravity while holding jump (long jump mechanic)
                if jump_pressed && self.jump_held_time < self.long_jump_time {
                    effective_gravity *= self.long_jump_gravity_scale;
                }

                new_velocity.y -= effective_gravity * dt;
            }
        } else {
            if !self.is_landing {
                self.is_landing = true;
            }
            new_velocity.y = 0.0;
            self.is_rising = false;
            self.is_falling = false;
        }

        // --- Movement with acceleration ---
        // Determine target speed
        let target_speed = if is_walking_input {
            self.walk_speed
        } else if self.is_sprinting_input {
            self.run_speed
        } else {
            self.jog_speed
        };

        // Calculate acceleration based on ground/air state
        let current_accel = if on_floor {
            lerp(
                self.ground_acceleration,
                self.max_ground_acceleration,
                self.acceleration_weight,
            )
        } else {
            lerp(
                self.air_acceleration,
                self.max_air_acceleration,
                self.acceleration_weight,
            )
        };

        // Apply movement
        if direction.length() > 0.0 {
            if on_floor {
                self.current_direction =
                    move_toward_vec3(self.current_direction, direction, current_accel * dt);
                new_velocity.x = self.current_direction.x * target_speed;
                new_velocity.z = self.current_direction.z * target_speed;
            } else {
                // Air control - slower velocity change
                self.current_direction =
                    move_toward_vec3(self.current_direction, direction, current_accel * dt);
                let target_velocity = direction * target_speed;
                let mut horizontal_vel = Vector3::new(new_velocity.x, 0.0, new_velocity.z);
                horizontal_vel =
                    move_toward_vec3(horizontal_vel, target_velocity, current_accel * dt);
                new_velocity.x = horizontal_vel.x;
                new_velocity.z = horizontal_vel.z;
            }
        } else {
            // Deceleration
            if on_floor {
                if self.stop_time <= 0.0 {
                    new_velocity.x = 0.0;
                    new_velocity.z = 0.0;
                    self.current_direction = Vector3::ZERO;
                } else {
                    new_velocity.x =
                        move_toward(new_velocity.x, 0.0, target_speed / self.stop_time * dt);
                    new_velocity.z =
                        move_toward(new_velocity.z, 0.0, target_speed / self.stop_time * dt);
                }
            } else {
                // In air, maintain momentum with slight drag
                new_velocity.x = move_toward(new_velocity.x, 0.0, 0.5 * dt);
                new_velocity.z = move_toward(new_velocity.z, 0.0, 0.5 * dt);
            }
        }

        // Store last position before moving
        self.last_position = self.base().get_global_position();

        // Apply velocity and move
        self.base_mut().set_velocity(new_velocity);
        self.base_mut().move_and_slide();

        // Clamp to floor (can't go below y=0)
        let mut pos = self.base().get_position();
        if pos.y < 0.0 {
            pos.y = 0.0;
            self.base_mut().set_position(pos);
        }

        // Calculate actual XZ velocity for animation state (after move_and_slide)
        let global_position = self.base().get_global_position();
        let pos_xz = Vector2::new(global_position.x, global_position.z);
        let last_pos_xz = Vector2::new(self.last_position.x, self.last_position.z);
        self.actual_velocity_xz = if dt > 0.0 {
            (pos_xz - last_pos_xz).length() / dt
        } else {
            0.0
        };

        // Update movement state for avatar animation
        self.update_movement_state(self.actual_velocity_xz);

        // Handle forced position (teleport stuck state)
        if self.has_forced_position {
            let forced_pos = self.forced_position;
            self.base_mut().set_global_position(forced_pos);
            self.base_mut().set_velocity(Vector3::ZERO);
        }
    }

    /// Get the current movement direction (for avatar facing)
    #[func]
    pub fn get_current_direction(&self) -> Vector3 {
        self.current_direction
    }

    /// Get the actual XZ velocity (for animation)
    #[func]
    pub fn get_actual_velocity_xz(&self) -> f32 {
        self.actual_velocity_xz
    }

    /// Check if sprinting input is active
    #[func]
    pub fn is_sprint_input_active(&self) -> bool {
        self.is_sprinting_input
    }

    /// Teleport player to a position, handling stuck detection
    #[func]
    pub fn async_move_to(&mut self, target: Vector3) {
        // Clear any previous forced position state
        self.has_forced_position = false;

        let original_target = target;
        self.base_mut().set_global_position(target);
        self.base_mut().set_velocity(Vector3::ZERO);

        // Store the target for potential stuck handling
        self.forced_position = original_target;
    }

    /// Called from GDScript after physics frame to check if stuck
    #[func]
    pub fn check_stuck_after_teleport(&mut self, original_target: Vector3) {
        let current_pos = self.base().get_global_position();
        if current_pos.distance_to(original_target) > 0.01 {
            self.forced_position = original_target;
            self.has_forced_position = true;
            self.base_mut().set_global_position(original_target);
        }
    }

    /// Clear forced position state
    #[func]
    pub fn clear_forced_position(&mut self) {
        self.has_forced_position = false;
    }

    fn update_movement_state(&mut self, vel: f32) {
        self.is_walking = false;
        self.is_jogging = false;
        self.is_running = false;

        // Find which movement state is closest to current velocity
        let idle_diff = vel.abs();
        let walk_diff = (vel - self.walk_speed).abs();
        let jog_diff = (vel - self.jog_speed).abs();
        let run_diff = (vel - self.run_speed).abs();

        let min_diff = idle_diff.min(walk_diff).min(jog_diff).min(run_diff);

        if min_diff == walk_diff {
            self.is_walking = true;
        } else if min_diff == jog_diff {
            self.is_jogging = true;
        } else if min_diff == run_diff {
            self.is_running = true;
        }
        // else idle (all false)
    }
}

// Helper functions
fn move_toward(from: f32, to: f32, delta: f32) -> f32 {
    if (to - from).abs() <= delta {
        to
    } else {
        from + (to - from).signum() * delta
    }
}

fn move_toward_vec3(from: Vector3, to: Vector3, delta: f32) -> Vector3 {
    let diff = to - from;
    let len = diff.length();
    if len <= delta || len < f32::EPSILON {
        to
    } else {
        from + diff / len * delta
    }
}

fn lerp(from: f32, to: f32, weight: f32) -> f32 {
    from + (to - from) * weight
}
