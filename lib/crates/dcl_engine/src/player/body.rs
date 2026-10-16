use godot::classes::{
    tween::EaseType, CharacterBody3D, CollisionObject3D, CollisionShape3D, ICharacterBody3D, Input,
    Node3D, PhysicsRayQueryParameters3D, RayCast3D, SpringArm3D, Tween,
};
use godot::prelude::*;

use super::{
    camera_clamp::CameraCollisionClamp,
    desktop_input::PlayerDesktopInput,
    outline_system::OutlineSystem,
    rig::{self, resolve_is_grounded, rig_targets},
    stuck_detector::StuckDetector,
};
use crate::{
    avatars::{avatar::build::update_from_profile, dcl_user_profile::DclUserProfile},
    engine::local_task::spawn_local,
    fetcher::globals,
    godot_classes::{
        dcl_avatar::DclAvatar, dcl_camera_3d::DclCamera3D, dcl_global::DclGlobal,
        dcl_locomotion_settings::DclLocomotionSettings,
    },
};

pub const DEFAULT_CAMERA_FOV: f32 = 60.0;
pub const SPRINTING_CAMERA_FOV: f32 = 75.0;

// Double-jump and glide tuning (values mirror Unity CharacterControllerSettings.asset).
pub const MAX_AIR_JUMPS: i32 = 1;
pub const JUMP_BUFFER_WINDOW: f32 = 0.15;
pub const JUMP_COOLDOWN: f32 = rig::JUMP_COOLDOWN;
pub const AIR_JUMP_HEIGHT: f32 = 2.0;
pub const AIR_JUMP_DELAY: f32 = 0.2;
pub const AIR_JUMP_DIRECTION_IMPULSE: f32 = 8.0;
pub const GLIDE_MAX_FALL_SPEED: f32 = 1.0;
pub const GLIDE_HORIZONTAL_SPEED: f32 = 6.0;
pub const GLIDE_MIN_GROUND_DISTANCE: f32 = 1.0;
pub const JUMP_TO_GLIDE_INTERVAL: f32 = 0.5;
pub const GLIDE_COOLDOWN: f32 = 0.6;
pub const GLIDE_OPENING_TIME: f32 = 0.5;
pub const GLIDE_CLOSING_TIME: f32 = 0.15;

/// Character mass for scene-driven impulses and forces (Unity parity): dv = impulse / mass,
/// a = force / mass.
pub const CHARACTER_MASS: f32 = 1.0;
/// Viscous drag on `external_velocity` each tick: environment always, plus ground friction.
pub const EXT_ENV_DRAG: f32 = 1.5;
pub const EXT_GROUND_FRICTION: f32 = 4.0;
pub const MAX_EXTERNAL_VELOCITY: f32 = 50.0;
pub const EXT_VELOCITY_EPSILON_SQR: f32 = 0.0001;

// Glide state machine values: DclAvatar.glide_state and rfc4.Movement.GlideState.
pub const GLIDE_CLOSED: i32 = 0;
pub const GLIDE_OPENING: i32 = 1;
pub const GLIDE_GLIDING: i32 = 2;
pub const GLIDE_CLOSING: i32 = 3;

// Unity parity (ApplyCharacterMovementVelocity.cs / ApplyHorizontalAirDrag.cs): the acceleration
// weight ramps 0->1 over 0.5s while input is held; air drag is quadratic.
const ACCELERATION_TIME: f32 = 0.5;
const GROUND_ACCEL: f32 = 20.0;
const GROUND_ACCEL_MAX: f32 = 25.0;
const AIR_ACCEL: f32 = 15.0;
const AIR_ACCEL_MAX: f32 = 20.0;
const AIR_DRAG: f32 = 0.2;

// Unity parity (ApplyJump.cs / ApplyGravity.cs). Seconds, never ticks.
const COYOTE_WINDOW: f32 = 0.15;
pub(super) const GRAVITY_ASCENT_FACTOR: f32 = 4.0;
const LONG_JUMP_TIME: f32 = 0.5;
const LONG_JUMP_GRAVITY_SCALE: f32 = 0.5;

/// What the jump button would do if pressed right now, for the UI to pick its icon.
pub const JUMP_ACTION_NONE: i32 = 0;
pub const JUMP_ACTION_JUMP: i32 = 1;
pub const JUMP_ACTION_GLIDE_TOGGLE: i32 = 2;

/// AvatarRaycast resting target (matches player.tscn): straight ahead, 10m.
const AVATAR_RAYCAST_DEFAULT_TARGET: Vector3 = Vector3::new(0.0, 0.0, -10.0);
const CAMERA_MODE_TWEEN_TIME: f32 = 0.25;
/// Volume of the 1p<->3p transition sound.
const CAMERA_TRANSITION_SOUND_DB: f32 = -3.0;
// Crosshair: first person is screen center. For third person the target tracks the live
// projection of the avatar's on-screen top and side edges.
const CROSSHAIR_PLACEHOLDER_ANCHOR: Vector2 = Vector2::new(0.53, 0.44);
const CROSSHAIR_TOP_GAP_PX: f32 = 20.0;
const CROSSHAIR_SCREEN_MARGIN: f32 = 24.0;
const AVATAR_TOP_HEIGHT: f32 = 1.8;
const AVATAR_HALF_WIDTH: f32 = 0.35;

/// The CharacterBody3D collision mask in player.tscn (layer 2, world/terrain): keeps the ground
/// raycast from pinging avatar wearables, triggers or other non-ground bodies.
pub(super) const GROUND_RAYCAST_MASK: u32 = 2;

// Slope and step constants (Unity parity: ApplySlopeModifier.cs / CharacterObject.prefab).
// CharacterBody3D has no built-in step offset, hence the custom logic in `step_up.rs`.
pub(super) const STEP_MAX_HEIGHT: f32 = 0.4375;
pub(super) const STEP_MIN_RISE: f32 = 0.02;
pub(super) const STEP_PENDING_RISE: f32 = 0.05;
pub(super) const STEP_TALL_RISE: f32 = 0.25;
pub(super) const SLOPE_WALK_NORMAL_Y: f32 = 0.99;
pub(super) const WALKABLE_NORMAL_Y: f32 = 0.695;
pub(super) const REALM_FLOOR_EPS: f32 = 0.005;
const DOWNSLOPE_STICK_JOG: f32 = 0.45;
const DOWNSLOPE_STICK_RUN: f32 = 0.55;
/// Unity serializes 46deg; the engine default 45 is wrong.
const SLOPE_LIMIT_DEG: f32 = 46.0;
pub(super) const CAPSULE_CENTER_Y: f32 = 0.8;
pub(super) const CAPSULE_RADIUS: f32 = 0.3;

const UI_FADE_OUT: &str = "ui_fade_out";
const UI_FADE_IN: &str = "ui_fade_in";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CameraMode {
    FirstPerson,
    ThirdPerson,
    Cinematic,
}

impl CameraMode {
    pub fn from_i32(value: i32) -> Self {
        match value {
            0 => Self::FirstPerson,
            2 => Self::Cinematic,
            _ => Self::ThirdPerson,
        }
    }

    pub fn to_i32(self) -> i32 {
        match self {
            Self::FirstPerson => 0,
            Self::ThirdPerson => 1,
            Self::Cinematic => 2,
        }
    }
}

fn move_toward(from: f32, to: f32, delta: f32) -> f32 {
    if (to - from).abs() <= delta {
        to
    } else {
        from + (to - from).signum() * delta
    }
}

/// Godot's `signf`: 0 for zero.
fn signf(x: f32) -> f32 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        0.0
    }
}

fn to_xz(v: Vector3) -> Vector2 {
    Vector2::new(v.x, v.z)
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The local player: locomotion, the camera rig, scene-driven physics and the crosshair.
#[derive(GodotClass)]
#[class(base=CharacterBody3D)]
pub struct Player {
    base: Base<CharacterBody3D>,

    last_position: Vector3,
    #[var(pub)]
    actual_velocity_xz: f32,

    // Locomotion settings, updated from the current scene's DclLocomotionSettings.
    #[var(pub)]
    walk_speed: f32,
    #[var(pub)]
    jog_speed: f32,
    #[var(pub)]
    run_speed: f32,
    gravity_scale: f32,
    #[var(pub)]
    jump_height: f32,
    #[var(pub)]
    run_jump_height: f32,
    #[var(pub)]
    hard_landing_cooldown: f32,

    #[var(pub)]
    jump_count: i32,
    #[var(pub)]
    glide_state: i32,

    #[var(pub)]
    camera_mode_change_blocked: bool,
    stored_camera_mode_before_block: CameraMode,

    #[var(pub)]
    current_direction: Vector3,
    #[var(pub)]
    time_falling: f32,
    #[var(pub)]
    current_profile_version: i32,

    /// Persistent velocity accumulating scene-driven impulses (full XYZ) and forces (XZ only:
    /// force.y feeds gravity). Decays via drag each tick.
    #[var(pub)]
    external_velocity: Vector3,

    hard_landing_timer: f32,
    locomotion_settings: Option<Gd<DclLocomotionSettings>>,
    jump_buffer: f32,
    accel_weight: f32,
    glide_timer: f32,
    time_since_last_jump: f32,
    time_since_glide_end: f32,
    air_jump_delay_timer: f32,
    air_jump_direction: Vector3,
    ground_distance: f32,
    /// True while AvatarRaycast is aimed at the crosshair (mobile, non-cinematic): gates the
    /// restore-to-default so it does not write constants every tick.
    avatar_raycast_crosshair_active: bool,
    crosshair_screen_pos: Vector2,
    /// 1p -> 3p fades the crosshair in so it never covers the avatar's head.
    crosshair_alpha: f32,
    crosshair_pos_initialized: bool,
    crosshair_transition_from: Vector2,
    crosshair_transition_clock: f32,
    crosshair_transition_active: bool,
    crosshair_prev_mode: CameraMode,
    /// RIDs excluded from the ground and step probes: the body and the avatar's colliders.
    pub(super) raycast_exclude: Array<Rid>,

    // Pinch-to-zoom (mobile).
    pinch_accumulated_delta: f32,
    pinch_start_mode: CameraMode,
    camera_mode_tween: Option<Gd<Tween>>,

    // Step-up arming: a tall rise, or two consecutive rises with no rest between, disarms the
    // step-up until the capsule rests. A ramp face never rests, a staircase tread always does.
    pub(super) step_armed: bool,
    pub(super) step_pending: bool,

    #[var(no_set, pub)]
    mount_camera: Option<Gd<SpringArm3D>>,
    #[var(no_set, pub)]
    camera: Option<Gd<DclCamera3D>>,
    camera_collision_clamp: Option<Gd<CameraCollisionClamp>>,
    avatar_raycast: Option<Gd<RayCast3D>>,
    #[var(no_set, pub)]
    outline_system: Option<Gd<OutlineSystem>>,
    #[var(no_set, pub)]
    avatar: Option<Gd<DclAvatar>>,
    stuck_detector: Option<Gd<StuckDetector>>,
    /// Margin-less clone of the body shape for step-up motion tests (skin width would eat the
    /// step band).
    pub(super) step_test_shape: Option<Gd<godot::classes::CapsuleShape3D>>,
}

#[godot_api]
impl ICharacterBody3D for Player {
    fn init(base: Base<CharacterBody3D>) -> Self {
        Self {
            base,
            last_position: Vector3::ZERO,
            actual_velocity_xz: 0.0,
            walk_speed: 1.5,
            jog_speed: 8.0,
            run_speed: 10.0,
            gravity_scale: 10.0,
            jump_height: 1.0,
            run_jump_height: 1.5,
            hard_landing_cooldown: 0.0,
            jump_count: 0,
            glide_state: GLIDE_CLOSED,
            camera_mode_change_blocked: false,
            stored_camera_mode_before_block: CameraMode::ThirdPerson,
            current_direction: Vector3::ZERO,
            time_falling: 0.0,
            current_profile_version: -1,
            external_velocity: Vector3::ZERO,
            hard_landing_timer: 0.0,
            locomotion_settings: None,
            jump_buffer: 0.0,
            accel_weight: 0.0,
            glide_timer: 0.0,
            time_since_last_jump: 1000.0,
            time_since_glide_end: 1000.0,
            air_jump_delay_timer: 0.0,
            air_jump_direction: Vector3::ZERO,
            ground_distance: f32::INFINITY,
            avatar_raycast_crosshair_active: false,
            crosshair_screen_pos: Vector2::ZERO,
            crosshair_alpha: 1.0,
            crosshair_pos_initialized: false,
            crosshair_transition_from: Vector2::ZERO,
            crosshair_transition_clock: 0.0,
            crosshair_transition_active: false,
            crosshair_prev_mode: CameraMode::ThirdPerson,
            raycast_exclude: Array::new(),
            pinch_accumulated_delta: 0.0,
            pinch_start_mode: CameraMode::ThirdPerson,
            camera_mode_tween: None,
            step_armed: true,
            step_pending: false,
            mount_camera: None,
            camera: None,
            camera_collision_clamp: None,
            avatar_raycast: None,
            outline_system: None,
            avatar: None,
            stuck_detector: None,
            step_test_shape: None,
        }
    }

    fn ready(&mut self) {
        let this = self.to_gd();
        self.mount_camera = self.base().try_get_node_as("Mount");
        self.camera = self.base().try_get_node_as("Mount/CameraArm/Camera3D");
        self.camera_collision_clamp = self.base().try_get_node_as("Mount/CameraCollisionClamp");
        self.avatar_raycast = self
            .base()
            .try_get_node_as("Mount/CameraArm/Camera3D/AvatarRaycast");
        self.outline_system = self
            .base()
            .try_get_node_as("Mount/CameraArm/Camera3D/OutlineSystem");
        self.avatar = self.base().try_get_node_as("Avatar");
        self.stuck_detector = self.base().try_get_node_as("StuckDetector");
        self.step_test_shape = self
            .base()
            .try_get_node_as::<CollisionShape3D>("CollisionShape3D_Body")
            .and_then(|body| body.get_shape())
            .and_then(|shape| shape.try_cast::<godot::classes::CapsuleShape3D>().ok())
            .map(|shape| {
                let mut copy = shape.duplicate_resource();
                copy.set_margin(0.0);
                copy
            });

        let global = DclGlobal::singleton();
        if !global.bind().is_mobile {
            let input = PlayerDesktopInput::create(this.clone());
            self.base_mut().add_child(&input);
        }
        let mut global_node = global.clone();
        global_node.connect(
            "camera_mode_set",
            &Callable::from_fn("player_camera_mode_set", {
                let mut this = this.clone();
                move |args| {
                    let mode = CameraMode::from_i32(args[0].to::<i32>());
                    if mode != CameraMode::Cinematic {
                        this.bind_mut().set_camera_mode_with_sound(mode, true);
                    }
                    Variant::nil()
                }
            }),
        );

        // The outline system follows the main camera.
        if let (Some(outline), Some(camera)) = (&mut self.outline_system, &self.camera) {
            outline.bind_mut().setup(camera.clone().upcast());
        }
        if let Some(camera) = &mut self.camera {
            camera.make_current();
        }

        // Third person on setup, without the transition sound.
        self.set_camera_mode_with_sound(CameraMode::ThirdPerson, false);
        if let Some(avatar) = &mut self.avatar {
            avatar.bind_mut().is_local_player = true;
        }

        // Unity's slopeLimit is exclusive (a 46 deg ramp does not climb); Godot's
        // floor_max_angle is inclusive, so apply a hair under.
        self.base_mut()
            .set_floor_max_angle((SLOPE_LIMIT_DEG - 0.01).to_radians());

        globals::player_identity().connect(
            "profile_changed",
            &Callable::from_fn("player_profile_changed", {
                let mut this = this.clone();
                move |args| {
                    if let Ok(profile) = args[0].try_to::<Gd<Object>>() {
                        this.bind_mut().on_player_profile_changed(profile);
                    }
                    Variant::nil()
                }
            }),
        );

        if let Some(avatar) = &mut self.avatar {
            // The own avatar's click area is removed to avoid self-targeting.
            if let Some(mut click_area) = avatar.get_node_or_null("%ClickArea") {
                click_area.queue_free();
            }
            // Trigger detection for the local avatar (entity_id=1, SceneEntityId::PLAYER).
            avatar.bind_mut().setup_trigger_detection(1);
        }

        let mut runner = globals::scene_runner();
        runner.connect(
            "on_change_scene_id",
            &Callable::from_fn("player_scene_changed", {
                let mut this = this.clone();
                move |_| {
                    this.bind_mut().on_scene_changed();
                    Variant::nil()
                }
            }),
        );
        // A masked (upper-body) emote only plays while its owning scene is the one the player is
        // standing in. Local player only: a remote avatar's emote lifetime is its own client's.
        if let Some(avatar) = &self.avatar {
            runner.connect(
                "on_change_scene_id",
                &avatar.callable("on_current_scene_changed"),
            );
        }
        runner.connect(
            "locomotion_settings_changed",
            &Callable::from_fn("player_locomotion_settings_changed", {
                let mut this = this.clone();
                move |args| {
                    if let Ok(settings) = args[0].try_to::<Gd<DclLocomotionSettings>>() {
                        this.bind_mut().set_locomotion_settings(settings);
                    }
                    Variant::nil()
                }
            }),
        );
        self.on_scene_changed();

        // Reset the pinch zoom on a deliberate transition only.
        global_node.connect(
            "loading_finished",
            &Callable::from_fn("player_loading_finished", {
                let mut this = this.clone();
                move |_| {
                    if DclGlobal::singleton().bind().is_mobile {
                        this.bind_mut().reset_zoom_to_default();
                    }
                    Variant::nil()
                }
            }),
        );

        // RIDs to exclude from ground-distance raycasts (the body and the avatar subtree's
        // colliders, including the TriggerDetector, which would otherwise make the ray report
        // ~0m at all times). The scene pointer raycast excludes the same set: with the
        // third-person avatar centered on screen, the ray from the camera would hit its back.
        self.build_raycast_exclude();
        runner
            .bind_mut()
            .set_pointer_raycast_exclude(self.raycast_exclude.clone());

        // The avatar is top-level: start it where the player is.
        let position = self.base().get_global_position();
        let rotation_y = self.base().get_rotation().y;
        if let Some(avatar) = &mut self.avatar {
            avatar.set_global_position(position);
            avatar.set_rotation(Vector3::new(0.0, rotation_y, 0.0));
        }
    }

    fn physics_process(&mut self, dt: f64) {
        self.physics_tick(dt as f32);
    }
}

#[godot_api]
impl Player {
    #[constant]
    pub const JUMP_ACTION_NONE: i32 = JUMP_ACTION_NONE;
    #[constant]
    pub const JUMP_ACTION_JUMP: i32 = JUMP_ACTION_JUMP;
    #[constant]
    pub const JUMP_ACTION_GLIDE_TOGGLE: i32 = JUMP_ACTION_GLIDE_TOGGLE;
    #[constant]
    pub const GLIDE_CLOSED: i32 = GLIDE_CLOSED;
    #[constant]
    pub const GLIDE_OPENING: i32 = GLIDE_OPENING;
    #[constant]
    pub const GLIDE_GLIDING: i32 = GLIDE_GLIDING;
    #[constant]
    pub const GLIDE_CLOSING: i32 = GLIDE_CLOSING;

    #[func]
    pub fn on_camera_mode_blocked(&mut self, forced_mode: i32) {
        if !self.camera_mode_change_blocked {
            // Already blocked: the state to return to is stored once.
            self.stored_camera_mode_before_block = self.current_camera_mode();
            self.camera_mode_change_blocked = true;
        }
        self.set_camera_mode_with_sound(CameraMode::from_i32(forced_mode), false);
        DclGlobal::singleton().call("set_camera_mode_blocked", &[true.to_variant()]);
    }

    #[func]
    pub fn on_camera_mode_unblocked(&mut self) {
        self.camera_mode_change_blocked = false;
        DclGlobal::singleton().call("set_camera_mode_blocked", &[false.to_variant()]);
        let stored = self.stored_camera_mode_before_block;
        self.set_camera_mode_with_sound(stored, false);
    }

    #[func]
    pub fn set_camera_mode(&mut self, mode: i32, play_sound: bool) {
        self.set_camera_mode_with_sound(CameraMode::from_i32(mode), play_sound);
    }

    #[func]
    pub fn clamp_camera_rotation(&mut self) {
        let mode = self.current_camera_mode();
        let Some(mount) = &mut self.mount_camera else {
            return;
        };
        let mut rotation = mount.get_rotation();
        match mode {
            CameraMode::FirstPerson => {
                rotation.x = rotation
                    .x
                    .clamp((-60.0_f32).to_radians(), 90.0_f32.to_radians());
            }
            CameraMode::ThirdPerson => {
                rotation.x = rotation
                    .x
                    .clamp((-70.0_f32).to_radians(), 35.0_f32.to_radians());
            }
            CameraMode::Cinematic => {}
        }
        mount.set_rotation(rotation);
    }

    /// A relative look delta (screen pixels), shared by the touch handler and the scene-UI swipe
    /// handoff so both rotate the camera identically.
    #[func]
    pub fn apply_look_delta(&mut self, relative: Vector2) {
        use super::mobile_input::{HORIZONTAL_SENS, VERTICAL_SENS};
        self.base_mut()
            .rotate_y((-relative.x).to_radians() * HORIZONTAL_SENS);
        if let Some(mount) = &mut self.mount_camera {
            mount.rotate_x((-relative.y).to_radians() * VERTICAL_SENS);
        }
        self.clamp_camera_rotation();
    }

    /// Pinch-to-zoom (mobile): begin, then apply per frame, then end, around a two-finger pinch.
    #[func]
    pub fn begin_pinch_zoom(&mut self) {
        self.pinch_accumulated_delta = 0.0;
        self.pinch_start_mode = self.current_camera_mode();
    }

    /// `pixel_delta` is the change in distance between the two fingers this frame. Fingers
    /// opening zoom IN (toward first person), closing zoom OUT. After a toggle the accumulator
    /// resets, so swapping back needs a fresh threshold of spread in the opposite direction.
    #[func]
    pub fn apply_pinch_zoom(&mut self, pixel_delta: f32) {
        if self.camera_mode_change_blocked {
            return;
        }
        self.pinch_accumulated_delta += pixel_delta;
        let mode = self.current_camera_mode();
        match rig::toggle_direction(self.pinch_accumulated_delta) {
            1 => {
                if mode == CameraMode::ThirdPerson {
                    set_global_camera_mode(CameraMode::FirstPerson);
                }
                self.pinch_accumulated_delta = 0.0;
            }
            -1 => {
                if mode == CameraMode::FirstPerson {
                    set_global_camera_mode(CameraMode::ThirdPerson);
                }
                self.pinch_accumulated_delta = 0.0;
            }
            _ => {}
        }
    }

    #[func]
    pub fn end_pinch_zoom(&mut self) {
        if self.camera_mode_change_blocked {
            return;
        }
        let mode = self.current_camera_mode();
        if mode == self.pinch_start_mode {
            return; // no net change: nothing to report
        }
        let direction = if mode == CameraMode::ThirdPerson {
            "zoom_out"
        } else {
            "zoom_in"
        };
        globals::metrics().bind_mut().track_click_button(
            "ZOOM".to_string(),
            "IN_WORLD".to_string(),
            serde_json::json!({ "zoom_direction": direction }).to_string(),
        );
    }

    /// The crosshair target in screen pixels, smoothed: the single source for the HUD label,
    /// the scene interaction raycast and AvatarRaycast, so the three never desync.
    #[func]
    pub fn get_crosshair_screen_position(&self) -> Vector2 {
        self.crosshair_screen_pos
    }

    /// Crosshair opacity for the HUD label.
    #[func]
    pub fn get_crosshair_alpha(&self) -> f32 {
        self.crosshair_alpha
    }

    #[func]
    pub fn avatar_look_at(&mut self, target_position: Vector3) {
        let direction =
            rig::normalized_or_zero(target_position - self.base().get_global_position());
        let y_rot = direction.x.atan2(direction.z);
        let x_rot = direction
            .y
            .atan2((direction.x * direction.x + direction.z * direction.z).sqrt());
        // Player body, avatar and camera look at the same target.
        let mut rotation = self.base().get_rotation();
        rotation.y = y_rot + std::f32::consts::PI;
        self.base_mut().set_rotation(rotation);
        if let Some(avatar) = &mut self.avatar {
            avatar.set_rotation(Vector3::new(0.0, y_rot + std::f32::consts::PI, 0.0));
        }
        if let Some(mount) = &mut self.mount_camera {
            let mut mount_rotation = mount.get_rotation();
            mount_rotation.x = x_rot;
            mount.set_rotation(mount_rotation);
        }
        self.clamp_camera_rotation();
    }

    /// Faces the avatar to `target_position` independently of the camera (both targets given).
    #[func]
    pub fn set_avatar_rotation_independent(&mut self, target_position: Vector3) {
        let direction =
            rig::normalized_or_zero(target_position - self.base().get_global_position());
        let y_rot = direction.x.atan2(direction.z);
        if let Some(avatar) = &mut self.avatar {
            // Top-level, so the world-space Y rotation is set directly.
            let mut rotation = avatar.get_rotation();
            rotation.y = y_rot + std::f32::consts::PI;
            avatar.set_rotation(rotation);
        }
    }

    #[func]
    pub fn camera_look_at(&mut self, target_position: Vector3) {
        let direction =
            rig::normalized_or_zero(target_position - self.base().get_global_position());
        let y_rot = direction.x.atan2(direction.z);
        let x_rot = direction
            .y
            .atan2((direction.x * direction.x + direction.z * direction.z).sqrt());
        // Body Y rotation and camera mount X rotation, as the normal controls do.
        let mut rotation = self.base().get_rotation();
        rotation.y = y_rot + std::f32::consts::PI;
        self.base_mut().set_rotation(rotation);
        if let Some(mount) = &mut self.mount_camera {
            let mut mount_rotation = mount.get_rotation();
            mount_rotation.x = x_rot;
            mount.set_rotation(mount_rotation);
        }
        self.clamp_camera_rotation();
    }

    #[func]
    pub fn get_broadcast_position(&self) -> Vector3 {
        self.avatar
            .as_ref()
            .map(|avatar| avatar.get_global_transform().origin)
            .unwrap_or(Vector3::ZERO)
    }

    #[func]
    pub fn get_broadcast_rotation_y(&self) -> f32 {
        let rotation_y = if self.current_camera_mode() == CameraMode::ThirdPerson {
            self.avatar
                .as_ref()
                .map(|avatar| avatar.get_rotation().y)
                .unwrap_or(0.0)
        } else {
            self.base().get_rotation().y
        };
        // Wrapped into [-PI, PI) so it never crosses the discontinuity, then snapped to
        // 1-degree steps.
        const SNAP_STEP: f32 = 0.0174533;
        let wrapped = godot::global::wrapf(
            rotation_y as f64,
            -std::f64::consts::PI,
            std::f64::consts::PI,
        ) as f32;
        godot::global::snappedf(wrapped as f64, SNAP_STEP as f64) as f32
    }

    /// The avatar the crosshair ray hits, if any.
    #[func]
    pub fn get_avatar_under_crosshair(&self) -> Option<Gd<Node3D>> {
        let ray = self.avatar_raycast.as_ref()?;
        if !ray.is_colliding() {
            return None;
        }
        let collider = ray.get_collider()?;
        let is_avatar =
            collider.has_meta("is_avatar") && collider.get_meta("is_avatar").to::<bool>();
        if !is_avatar {
            return None;
        }
        // Walk up to the Avatar node.
        let mut node = collider.try_cast::<Node>().ok();
        while let Some(current) = node {
            if let Ok(avatar) = current.clone().try_cast::<DclAvatar>() {
                return Some(avatar.upcast());
            }
            node = current.get_parent();
        }
        None
    }

    #[func]
    pub fn get_jump_action(&self) -> i32 {
        let global = DclGlobal::singleton();
        let global = global.bind();
        if global.is_jump_disabled() || global.is_all_input_disabled() {
            return JUMP_ACTION_NONE;
        }
        if self.hard_landing_timer > 0.0 {
            return JUMP_ACTION_NONE;
        }
        if self.base().is_on_floor() || self.base().get_position().y <= 0.0 {
            return JUMP_ACTION_JUMP;
        }
        // Airborne. GLIDE_TOGGLE is reported while the glider is open even if the scene
        // disables gliding: the force-close transitions to CLOSING next tick, and NONE here
        // would flicker the icon in between.
        if self.glide_state == GLIDE_OPENING || self.glide_state == GLIDE_GLIDING {
            return JUMP_ACTION_GLIDE_TOGGLE;
        }
        if self.glide_state == GLIDE_CLOSING {
            return JUMP_ACTION_NONE;
        }
        // Closed: an air-jump takes priority over glide-open.
        if self.jump_count >= 1
            && self.jump_count <= MAX_AIR_JUMPS
            && !global.is_double_jump_disabled()
            && self.time_since_last_jump >= JUMP_COOLDOWN
        {
            return JUMP_ACTION_JUMP;
        }
        if !global.is_glide_disabled()
            && self.ground_distance > GLIDE_MIN_GROUND_DISTANCE
            && self.time_since_last_jump >= JUMP_TO_GLIDE_INTERVAL
            && self.time_since_glide_end >= GLIDE_COOLDOWN
        {
            return JUMP_ACTION_GLIDE_TOGGLE;
        }
        JUMP_ACTION_NONE
    }

    /// True while the next jump press would open or close the glider.
    #[func]
    pub fn can_toggle_glide(&self) -> bool {
        if self.glide_state == GLIDE_OPENING || self.glide_state == GLIDE_GLIDING {
            return true;
        }
        if self.glide_state != GLIDE_CLOSED {
            return false;
        }
        let global = DclGlobal::singleton();
        let global = global.bind();
        // jump_count in [1..MAX_AIR_JUMPS] means the next press fires an air-jump, not glide-open.
        let grounded = self.base().is_on_floor()
            || self.base().get_position().y <= 0.0
            || self.time_falling <= 0.0;
        let input_blocked = global.is_jump_disabled()
            || global.is_all_input_disabled()
            || global.is_glide_disabled();
        let air_jump_consumes_press = self.jump_count >= 1 && self.jump_count <= MAX_AIR_JUMPS;
        let too_low = self.ground_distance <= GLIDE_MIN_GROUND_DISTANCE;
        let on_cooldown = self.time_since_last_jump < JUMP_TO_GLIDE_INTERVAL
            || self.time_since_glide_end < GLIDE_COOLDOWN;
        !(grounded
            || input_blocked
            || self.hard_landing_timer > 0.0
            || air_jump_consumes_press
            || too_low
            || on_cooldown)
    }

    #[func]
    pub fn move_to(&mut self, target: Vector3, #[opt(default = true)] check_stuck: bool) {
        self.base_mut().set_global_position(target);
        self.base_mut().set_velocity(Vector3::ZERO);
        // A teleport mid-glide (or mid-air-jump hover) must not carry the glider lift or the
        // frozen gravity into the destination: back to a grounded-idle baseline, which the next
        // tick re-derives.
        self.jump_count = 0;
        self.glide_state = GLIDE_CLOSED;
        self.glide_timer = 0.0;
        self.jump_buffer = 0.0;
        self.air_jump_delay_timer = 0.0;
        if check_stuck {
            if let Some(detector) = &mut self.stuck_detector {
                detector.bind_mut().check_stuck();
            }
        }
    }
}

impl Player {
    pub(super) fn current_camera_mode(&self) -> CameraMode {
        self.camera
            .as_ref()
            .map(|camera| CameraMode::from_i32(camera.bind().get_camera_mode()))
            .unwrap_or(CameraMode::ThirdPerson)
    }

    pub fn set_camera_mode_with_sound(&mut self, mode: CameraMode, play_sound: bool) {
        if let Some(camera) = &mut self.camera {
            camera.bind_mut().set_camera_mode(mode.to_i32());
        }
        if let Some(mut tween) = self.camera_mode_tween.take() {
            if tween.is_running() {
                tween.kill();
            }
        }
        let third_person = match mode {
            CameraMode::ThirdPerson => true,
            CameraMode::FirstPerson => false,
            CameraMode::Cinematic => return,
        };
        let (spring_length, offset_x) = rig_targets(third_person);
        let mut tween = self.base_mut().create_tween();
        tween.set_parallel_ex().parallel(true).done();
        if let Some(mount) = &self.mount_camera {
            tween
                .tween_property(
                    mount,
                    "spring_length",
                    &spring_length.to_variant(),
                    CAMERA_MODE_TWEEN_TIME as f64,
                )
                .set_ease(EaseType::IN_OUT);
        }
        // The X offset lives on the collision clamp, which positions the camera below the arm so
        // the spring-arm pivot stays centered on the player capsule and sweeps a sphere to the
        // real (offset) camera position every physics frame.
        if let Some(clamp) = &self.camera_collision_clamp {
            tween
                .tween_property(
                    clamp,
                    "lateral_offset",
                    &offset_x.to_variant(),
                    CAMERA_MODE_TWEEN_TIME as f64,
                )
                .set_ease(EaseType::IN_OUT);
        }
        self.camera_mode_tween = Some(tween);

        let rotation_y = self.base().get_rotation().y;
        if third_person {
            if let Some(avatar) = &mut self.avatar {
                avatar.bind_mut().apply_hidden(false);
                avatar.set_rotation(Vector3::new(0.0, rotation_y, 0.0));
            }
            if play_sound {
                play_ui_sound(UI_FADE_OUT);
            }
        } else {
            if self
                .camera
                .as_ref()
                .is_some_and(|camera| camera.is_current())
            {
                if let Some(avatar) = &mut self.avatar {
                    avatar.bind_mut().apply_hidden(true);
                }
            }
            if play_sound {
                play_ui_sound(UI_FADE_IN);
            }
        }
    }

    fn on_player_profile_changed(&mut self, profile: Gd<Object>) {
        let Ok(profile) = profile.try_cast::<DclUserProfile>() else {
            return;
        };
        let version = profile.bind().get_profile_version() as i32;
        // Only update the avatar if the profile version changed.
        if version != self.current_profile_version {
            self.current_profile_version = version;
            if let Some(avatar) = &self.avatar {
                spawn_local(update_from_profile(avatar.clone(), profile));
            }
        }
    }

    fn on_scene_changed(&mut self) {
        let settings = globals::scene_runner()
            .bind()
            .get_current_scene_locomotion_settings();
        self.set_locomotion_settings(settings);
    }

    fn set_locomotion_settings(&mut self, settings: Gd<DclLocomotionSettings>) {
        {
            let settings = settings.bind();
            self.walk_speed = settings.get_walk_speed();
            self.jog_speed = settings.get_jog_speed();
            self.run_speed = settings.get_run_speed();
            self.jump_height = settings.get_jump_height();
            self.run_jump_height = settings.get_run_jump_height();
            self.hard_landing_cooldown = settings.get_hard_landing_cooldown();
        }
        self.locomotion_settings = Some(settings);
    }

    /// Back to the default third-person view, on every deliberate transition on mobile (so a
    /// pinch into first person returns after the next loading screen). Skipped while a scene
    /// forces the camera mode.
    fn reset_zoom_to_default(&mut self) {
        if self.camera_mode_change_blocked {
            return;
        }
        if self.current_camera_mode() != CameraMode::ThirdPerson {
            set_global_camera_mode(CameraMode::ThirdPerson);
        }
    }

    fn build_raycast_exclude(&mut self) {
        self.raycast_exclude.clear();
        self.raycast_exclude.push(self.base().get_rid());
        if let Some(avatar) = &self.avatar {
            let mut out = Vec::new();
            collect_collider_rids(&avatar.clone().upcast(), &mut out);
            for rid in out {
                self.raycast_exclude.push(rid);
            }
        }
    }

    /// Distance from feet to ground for glide entry and close gating; infinite beyond 20m. Uses
    /// the terrain layer only, so wearables, triggers and scene gadgets do not collapse the
    /// reading; the exclude list is a belt-and-suspenders for avatar colliders that might
    /// briefly share the world mask.
    fn measure_ground_distance(&mut self) -> f32 {
        let Some(world) = self.base().get_world_3d() else {
            return f32::INFINITY;
        };
        let Some(mut space) = world.get_direct_space_state() else {
            return f32::INFINITY;
        };
        let from = self.base().get_global_position() + Vector3::new(0.0, 0.1, 0.0);
        let to = from + Vector3::new(0.0, -20.0, 0.0);
        let Some(mut query) = PhysicsRayQueryParameters3D::create(from, to) else {
            return f32::INFINITY;
        };
        query.set_collision_mask(GROUND_RAYCAST_MASK);
        query.set_exclude(&self.raycast_exclude);
        query.set_collide_with_bodies(true);
        query.set_collide_with_areas(false);
        let hit = space.intersect_ray(&query);
        match hit.get("position") {
            Some(position) => from.y - position.to::<Vector3>().y,
            None => f32::INFINITY,
        }
    }

    /// Asymmetric gravity (ApplyGravity.cs): the hold window first (applies on ascent and early
    /// descent: 10x0.5=5), then the ascent factor (hold+rise = 20).
    fn current_gravity(&self, vertical_velocity: f32) -> f32 {
        let mut g = self.gravity_scale;
        if Input::singleton().is_action_pressed("ia_jump")
            && self.time_since_last_jump < LONG_JUMP_TIME
        {
            g *= LONG_JUMP_GRAVITY_SCALE;
        }
        if vertical_velocity > 0.0 {
            g *= GRAVITY_ASCENT_FACTOR;
        }
        g
    }

    fn set_avatar_flag(&mut self, name: &str, value: bool) {
        if let Some(avatar) = &mut self.avatar {
            let mut avatar = avatar.bind_mut();
            match name {
                "walk" => avatar.walk = value,
                "jog" => avatar.jog = value,
                "run" => avatar.run = value,
                "rise" => avatar.rise = value,
                "fall" => avatar.fall = value,
                "land" => avatar.land = value,
                "is_grounded" => avatar.is_grounded = value,
                _ => godot_error!("unknown avatar flag `{name}`"),
            }
        }
    }

    fn avatar_flag(&self, name: &str) -> bool {
        self.avatar
            .as_ref()
            .map(|avatar| {
                let avatar = avatar.bind();
                match name {
                    "walk" => avatar.walk,
                    "jog" => avatar.jog,
                    "run" => avatar.run,
                    "rise" => avatar.rise,
                    "fall" => avatar.fall,
                    "land" => avatar.land,
                    "is_grounded" => avatar.is_grounded,
                    _ => false,
                }
            })
            .unwrap_or(false)
    }

    fn update_avatar_movement_state(&mut self, speed: f32) {
        let candidates = [
            ("idle", speed.abs()),
            ("walk", (speed - self.walk_speed).abs()),
            ("jog", (speed - self.jog_speed).abs()),
            ("run", (speed - self.run_speed).abs()),
        ];
        let mut nearest = candidates[0];
        for candidate in candidates {
            if candidate.1 < nearest.1 {
                nearest = candidate;
            }
        }
        self.set_avatar_flag("walk", nearest.0 == "walk");
        self.set_avatar_flag("jog", nearest.0 == "jog");
        self.set_avatar_flag("run", nearest.0 == "run");
    }

    fn physics_tick(&mut self, dt: f32) {
        let input = Input::singleton();
        let global = DclGlobal::singleton();
        let mut runner = globals::scene_runner();

        // Sample scene-driven physics before gravity: force.y feeds the effective gravity below.
        let scene_external_force = runner.bind().get_active_external_force();
        let scene_pending_impulses = runner.bind_mut().consume_pending_impulses();
        let external_acceleration = scene_external_force / CHARACTER_MASS;

        // Keep the top-level avatar co-located with the player (picks up teleports and
        // external position changes, and makes the look_at below use the right origin).
        let player_position = self.base().get_global_position();
        if let Some(avatar) = &mut self.avatar {
            avatar.set_global_position(player_position);
        }

        let mut velocity = self.base().get_velocity();

        // Hard landing cooldown prevents horizontal movement.
        if self.hard_landing_timer > 0.0 {
            self.hard_landing_timer -= dt;
            velocity.x = move_toward(velocity.x, 0.0, 20.0 * dt);
            velocity.z = move_toward(velocity.z, 0.0, 20.0 * dt);
        }

        self.jump_buffer = (self.jump_buffer - dt).max(0.0);
        let has_focus = explorer_has_focus();
        if has_focus && input.is_action_just_pressed("ia_jump") {
            self.jump_buffer = JUMP_BUFFER_WINDOW;
        }

        self.time_since_last_jump = (self.time_since_last_jump + dt).min(1000.0);
        self.time_since_glide_end = (self.time_since_glide_end + dt).min(1000.0);

        if self.glide_state == GLIDE_OPENING {
            self.glide_timer -= dt;
            if self.glide_timer <= 0.0 {
                self.glide_state = GLIDE_GLIDING;
            }
        } else if self.glide_state == GLIDE_CLOSING {
            self.glide_timer -= dt;
            if self.glide_timer <= 0.0 {
                self.glide_state = GLIDE_CLOSED;
                self.time_since_glide_end = 0.0;
            }
        }

        self.ground_distance = self.measure_ground_distance();

        let mut input_dir = input.get_vector("ia_left", "ia_right", "ia_forward", "ia_backward");
        let input_magnitude = input_dir.length().clamp(0.0, 1.0);
        if !has_focus {
            input_dir = Vector2::ZERO; // ignore input
        }

        let (
            all_disabled,
            walk_disabled,
            jog_disabled,
            run_disabled,
            jump_disabled,
            double_jump_disabled,
            glide_disabled,
        ) = {
            let global = global.bind();
            (
                global.is_all_input_disabled(),
                global.is_walk_disabled(),
                global.is_jog_disabled(),
                global.is_run_disabled(),
                global.is_jump_disabled(),
                global.is_double_jump_disabled(),
                global.is_glide_disabled(),
            )
        };

        // All input disabled, or a hard landing in progress: no direction.
        if all_disabled || self.hard_landing_timer > 0.0 {
            input_dir = Vector2::ZERO;
        }

        // Movement basis: the active camera's yaw when a virtual camera is active, otherwise the
        // player's transform.
        let active_camera = self
            .base()
            .get_viewport()
            .and_then(|viewport| viewport.get_camera_3d());
        let own_camera: Option<Gd<godot::classes::Camera3D>> =
            self.camera.as_ref().map(|c| c.clone().upcast());
        let movement_basis = match (&active_camera, &own_camera) {
            (Some(active), Some(own)) if active != own && active.is_instance_valid() => {
                Basis::from_axis_angle(Vector3::UP, active.get_global_rotation().y)
            }
            _ => self.base().get_transform().basis,
        };
        let direction =
            rig::normalized_or_zero(movement_basis * Vector3::new(input_dir.x, 0.0, input_dir.y));

        self.current_direction = self.current_direction.move_toward(direction, 8.0 * dt);

        let on_floor = self.base().is_on_floor() || self.base().get_position().y <= 0.0;
        // One grounded signal for the whole jump/gravity chain: the floor, or walkable support
        // (the margin-cloud rest has no floor contact). Mixing the two across branches left
        // landing unreachable mid-cloud (velocity.y accumulated, jump_count never reset, the
        // glider never closed).
        let supported = on_floor || self.has_walkable_support();
        let was_falling = self.avatar_flag("fall");
        // Fall time up to this tick, captured before the reset below so the landing branch can
        // still read it for the hard-landing check.
        let fall_duration = self.time_falling;

        // is_on_floor() drops before the capsule visually leaves an edge (rounded bottom and
        // speculative margin), which would burn the coyote window early; Unity's CCT
        // IsGrounded holds through the skin width. The fall timer starts only when walkable
        // support is truly gone.
        if !supported {
            self.time_falling += dt;
        } else {
            self.time_falling = 0.0;
        }

        // The ground jump stays reachable this long after leaving the floor, and the glide gate
        // must not eat the press. velocity.y guards it: the window opens only when walking off
        // a ledge, never on the way up (a jump-pad launch must not become a cancellable ground).
        let in_coyote = !supported && self.time_falling <= COYOTE_WINDOW && velocity.y <= 0.0;

        // Air-jump hover phase: freeze gravity, then fire the impulse and horizontal dash when
        // the timer expires. avatar.rise/fall stay untouched on purpose: flipping them mid-hover
        // would trip Jump_Fall -> Jump_End via nfall and strand the state machine away from
        // Double_Jump_Rise when jump_count flips.
        if self.air_jump_delay_timer > 0.0 {
            self.air_jump_delay_timer -= dt;
            velocity.y = 0.0;
            if self.air_jump_delay_timer <= 0.0 {
                velocity.y =
                    (2.0 * AIR_JUMP_HEIGHT * self.gravity_scale * GRAVITY_ASCENT_FACTOR).sqrt();
                let horiz = Vector3::new(self.air_jump_direction.x, 0.0, self.air_jump_direction.z);
                if horiz.length_squared() > 0.0001 {
                    let horiz = horiz.normalized();
                    // max(8, current horizontal speed) (ApplyJump.cs).
                    let impulse = AIR_JUMP_DIRECTION_IMPULSE
                        .max(Vector2::new(velocity.x, velocity.z).length());
                    velocity.x = horiz.x * impulse;
                    velocity.z = horiz.z * impulse;
                }
                self.jump_count += 1;
                self.time_since_last_jump = 0.0;
                self.set_avatar_flag("rise", true);
                self.set_avatar_flag("fall", false);
            }
        } else if !supported && !in_coyote {
            let in_grace_time = self.time_falling < 0.2
                && !input.is_action_pressed("ia_jump")
                && self.time_since_last_jump >= JUMP_COOLDOWN;
            self.set_avatar_flag("land", in_grace_time);
            // rise/fall are suppressed while the glider provides lift (OPENING + GLIDING); during
            // CLOSING normal gravity resumes so Jump_Fall can take over.
            let free_flight = self.glide_state == GLIDE_CLOSED || self.glide_state == GLIDE_CLOSING;
            self.set_avatar_flag("rise", velocity.y > 0.3 && free_flight);
            self.set_avatar_flag("fall", velocity.y < -0.3 && !in_grace_time && free_flight);
            // Scene force.y reduces effective gravity, so an upward wind cancels the fall
            // instead of stacking on velocity.y.
            let gravity = self.current_gravity(velocity.y);
            velocity.y -= (gravity - external_acceleration.y) * dt;

            // Air-jump: 0.2s hover then impulse (matches Unity ApplyJump two-step).
            if self.jump_buffer > 0.0
                && self.jump_count >= 1
                && self.jump_count <= MAX_AIR_JUMPS
                && self.glide_state == GLIDE_CLOSED
                && !jump_disabled
                && !double_jump_disabled
                && self.hard_landing_timer <= 0.0
                && self.time_since_last_jump >= JUMP_COOLDOWN
            {
                self.air_jump_delay_timer = AIR_JUMP_DELAY;
                self.air_jump_direction = self.current_direction;
                self.jump_buffer = 0.0;
            }

            // Glide toggle-open (mobile-friendly). Diverges from Unity's hold-to-glide and the
            // Unity-exact `jump_count > MAX_AIR_JUMPS` entry gate: a player who stepped off a
            // ledge can open the glide without first double-jumping. Air-jump still takes
            // priority (above) because it consumes the buffer first.
            if self.jump_buffer > 0.0 && self.glide_state == GLIDE_CLOSED {
                let gate_enabled = !jump_disabled && !glide_disabled;
                let gate_altitude = self.ground_distance > GLIDE_MIN_GROUND_DISTANCE;
                let gate_jump_interval = self.time_since_last_jump >= JUMP_TO_GLIDE_INTERVAL;
                let gate_cooldown = self.time_since_glide_end >= GLIDE_COOLDOWN;
                if gate_enabled && gate_altitude && gate_jump_interval && gate_cooldown {
                    self.glide_state = GLIDE_OPENING;
                    self.glide_timer = GLIDE_OPENING_TIME;
                    self.jump_buffer = 0.0;
                    self.set_avatar_flag("rise", false);
                    self.set_avatar_flag("fall", false);
                }
            }

            // Glide close: re-press (toggle), altitude too low, or input disabled. glide_disabled
            // covers scene-to-scene transitions where the destination forbids gliding.
            if self.glide_state == GLIDE_OPENING || self.glide_state == GLIDE_GLIDING {
                let exit_toggle = self.jump_buffer > 0.0;
                let exit_altitude = self.ground_distance <= GLIDE_MIN_GROUND_DISTANCE;
                let exit_disabled = jump_disabled || glide_disabled;
                if exit_toggle || exit_altitude || exit_disabled {
                    self.glide_state = GLIDE_CLOSING;
                    self.glide_timer = GLIDE_CLOSING_TIME;
                    if exit_toggle {
                        self.jump_buffer = 0.0;
                    }
                }
            }

            // Fall speed is clamped from OPENING onward so the 0.5s opening window is not a
            // free-fall.
            if (self.glide_state == GLIDE_OPENING || self.glide_state == GLIDE_GLIDING)
                && velocity.y < -GLIDE_MAX_FALL_SPEED
            {
                velocity.y = -GLIDE_MAX_FALL_SPEED;
            }
        } else if self.jump_buffer > 0.0
            && !jump_disabled
            && self.hard_landing_timer <= 0.0
            && self.time_since_last_jump >= JUMP_COOLDOWN
        {
            // Ground jump: consumes the buffer instead of reading the key again. Fires on the
            // floor and inside the coyote window. An exact port of ApplyJump.GetJumpHeight: the
            // run height only while sprinting, lerped by horizontal speed over run speed; v0
            // uses the ascent gravity.
            let h_speed = Vector2::new(velocity.x, velocity.z).length();
            let max_jump_height = if input.is_action_pressed("ia_sprint") {
                self.run_jump_height
            } else {
                self.jump_height
            };
            let effective_jump_height = self.jump_height
                + (max_jump_height - self.jump_height) * (h_speed / self.run_speed).clamp(0.0, 1.0);
            velocity.y =
                (2.0 * effective_jump_height * self.gravity_scale * GRAVITY_ASCENT_FACTOR).sqrt();
            self.jump_count = 1;
            self.jump_buffer = 0.0;
            self.time_since_last_jump = 0.0;
            self.set_avatar_flag("land", false);
            self.set_avatar_flag("rise", true);
            self.set_avatar_flag("fall", false);
        } else if supported {
            if !self.avatar_flag("land") {
                self.set_avatar_flag("land", true);
                if was_falling && self.hard_landing_cooldown > 0.0 && fall_duration > 1.0 {
                    self.hard_landing_timer = self.hard_landing_cooldown;
                }
            }
            velocity.y = 0.0;
            self.set_avatar_flag("rise", false);
            self.set_avatar_flag("fall", false);
            // Landing resets the air-jump budget and force-closes the glider.
            self.jump_count = 0;
            if self.glide_state == GLIDE_OPENING || self.glide_state == GLIDE_GLIDING {
                self.glide_state = GLIDE_CLOSING;
                self.glide_timer = GLIDE_CLOSING_TIME;
            }
        } else {
            // Coyote fall without a buffered jump: gravity applies, no landing state, and the
            // glide gate stays closed for the whole window.
            let gravity = self.current_gravity(velocity.y);
            velocity.y -= (gravity - external_acceleration.y) * dt;
        }

        // Unity port (ApplyCharacterMovementVelocity.cs): the weight ramps over 0.5s while input
        // is held; the accel pair follows the settings curve (keys 0/0.1->0, 0.9/1->1: plateau at
        // min, then ramp). The velocity target uses the RAW input direction; the smoothed
        // current_direction is only for facing.
        let has_move_input = direction != Vector3::ZERO;
        self.accel_weight = move_toward(
            self.accel_weight,
            if has_move_input { 1.0 } else { 0.0 },
            dt / ACCELERATION_TIME,
        );
        let curve_t = ((self.accel_weight - 0.1) / (0.9 - 0.1)).clamp(0.0, 1.0);
        let accel = if on_floor {
            GROUND_ACCEL + (GROUND_ACCEL_MAX - GROUND_ACCEL) * curve_t
        } else {
            AIR_ACCEL + (AIR_ACCEL_MAX - AIR_ACCEL) * curve_t
        };

        if let Some(camera) = &mut self.camera {
            camera.bind_mut().set_target_fov(DEFAULT_CAMERA_FOV);
        }
        if has_move_input {
            let wants_walk = input.is_action_pressed("ia_walk");
            let wants_sprint = input.is_action_pressed("ia_sprint");
            let is_mobile = global.bind().is_mobile;

            // The effective speed depends on the scene's input modifiers.
            let mut effective_speed = 0.0;
            if wants_sprint && !run_disabled {
                if let Some(camera) = &mut self.camera {
                    camera.bind_mut().set_target_fov(SPRINTING_CAMERA_FOV);
                }
                effective_speed = self.run_speed;
            } else if is_mobile {
                // Analog speed: walk->jog by stick displacement.
                if walk_disabled && !jog_disabled {
                    effective_speed = self.jog_speed;
                } else if jog_disabled && !walk_disabled {
                    effective_speed = self.walk_speed;
                } else if !walk_disabled && !jog_disabled {
                    effective_speed =
                        self.walk_speed + (self.jog_speed - self.walk_speed) * input_magnitude;
                }
            } else if wants_walk && !walk_disabled {
                effective_speed = self.walk_speed;
            } else if !jog_disabled {
                effective_speed = self.jog_speed;
            } else if !walk_disabled {
                effective_speed = self.walk_speed;
            }
            // else: no movement allowed.

            // ADAD sign correction: reversing an axis flips the sign, keeping momentum.
            let target_x = direction.x * effective_speed;
            let target_z = direction.z * effective_speed;
            if signf(target_x) != 0.0 && signf(target_x) != signf(velocity.x) {
                velocity.x = -velocity.x;
            }
            if signf(target_z) != 0.0 && signf(target_z) != signf(velocity.z) {
                velocity.z = -velocity.z;
            }
            // Ground and air accel pairs; air is move_toward instead of a direct assignment
            // (reduced air control).
            velocity.x = move_toward(velocity.x, target_x, accel * dt);
            velocity.z = move_toward(velocity.z, target_z, accel * dt);

            let look_target =
                rig::normalized_or_zero(self.current_direction) + self.base().get_position();
            if let Some(avatar) = &mut self.avatar {
                avatar.look_at(look_target);
                let rotation = avatar.get_rotation();
                avatar.set_rotation(Vector3::new(0.0, rotation.y, 0.0));
            }
        } else if on_floor {
            // StopTimeSec=0: the grounded stop is INSTANT in Unity (degenerate SmoothDamp), and
            // instant is tick-rate independent.
            velocity.x = 0.0;
            velocity.z = 0.0;
        } else {
            // Air with no input drifts toward 0 at the air accel rate.
            velocity.x = move_toward(velocity.x, 0.0, accel * dt);
            velocity.z = move_toward(velocity.z, 0.0, accel * dt);
        }

        // Quadratic horizontal air drag, coefficient 0.2 (live Unity value).
        if !on_floor {
            let h_vel = Vector2::new(velocity.x, velocity.z);
            let h_mag = h_vel.length();
            if h_mag > 0.0 {
                let h_vel = h_vel - h_vel.normalized() * (AIR_DRAG * h_mag * h_mag * dt).min(h_mag);
                velocity.x = h_vel.x;
                velocity.z = h_vel.y;
            }
        }

        // While gliding, horizontal speed is capped: this overrides the walk/jog/run speeds above.
        if self.glide_state == GLIDE_GLIDING {
            let horizontal = Vector2::new(velocity.x, velocity.z);
            if horizontal.length() > GLIDE_HORIZONTAL_SPEED {
                let horizontal = horizontal.normalized() * GLIDE_HORIZONTAL_SPEED;
                velocity.x = horizontal.x;
                velocity.z = horizontal.y;
            }
        }

        let global_position = self.base().get_global_position();
        self.actual_velocity_xz =
            (to_xz(global_position) - to_xz(self.last_position)).length() / dt;
        let actual = self.actual_velocity_xz;
        self.update_avatar_movement_state(actual);

        // Mirror the local physics state into DclAvatar so avatar.gd drives the AnimationTree
        // off the same numbers for the local and remote avatars.
        let jump_count = self.jump_count;
        let glide_state = self.glide_state;
        if let Some(avatar) = &mut self.avatar {
            let mut avatar = avatar.bind_mut();
            avatar.jump_count = jump_count;
            avatar.glide_state = glide_state;
        }
        // Debounced ungrounding (see resolve_is_grounded). The jump-pad override below
        // (combined_vy > 0.3) runs after this and still wins.
        let grounded = resolve_is_grounded(
            on_floor,
            self.time_falling,
            input.is_action_pressed("ia_jump"),
            self.time_since_last_jump,
        );
        self.set_avatar_flag("is_grounded", grounded);

        self.base_mut().set_velocity(velocity);
        self.apply_scene_physics(dt, external_acceleration, &scene_pending_impulses, on_floor);
        let mut velocity = self.base().get_velocity();

        // Re-derive rise/fall from the combined vertical velocity: the lift from
        // external_velocity is not visible in velocity.y alone (the Y mix is undone below, so
        // velocity.y reads near zero mid-bounce).
        if self.external_velocity.length_squared() > 0.01 {
            let combined_vy = velocity.y + self.external_velocity.y;
            let free_flight = self.glide_state == GLIDE_CLOSED || self.glide_state == GLIDE_CLOSING;
            if free_flight {
                if combined_vy > 0.3 {
                    self.set_avatar_flag("rise", true);
                    self.set_avatar_flag("fall", false);
                    self.set_avatar_flag("land", false);
                    self.set_avatar_flag("is_grounded", false);
                } else if combined_vy < -0.3 {
                    self.set_avatar_flag("fall", true);
                    self.set_avatar_flag("rise", false);
                }
            }
        }

        // Snapshot the locomotion XZ so it can be restored after the move. Otherwise
        // `move_toward` on the next no-input tick would decelerate from velocity-with-external
        // stacked, and the external add would compound indefinitely.
        let locomotion_x = velocity.x;
        let locomotion_z = velocity.z;
        let external_y_for_move = self.external_velocity.y;
        velocity.x += self.external_velocity.x;
        velocity.y += external_y_for_move;
        velocity.z += self.external_velocity.z;
        self.base_mut().set_velocity(velocity);

        self.last_position = self.base().get_global_position();
        // Downslope stick: ApplySlopeModifier picks by input kind (run when sprinting), not by
        // measured speed. Assigned BEFORE the predictive step-up: a rise zeroes the snap for that
        // frame's move so the lower floor within snap reach cannot re-glue the capsule mid-step.
        // Keeping the snap on every other frame is also what makes is_on_floor() reliable for the
        // re-arm.
        let snap = if input.is_action_pressed("ia_sprint") {
            DOWNSLOPE_STICK_RUN
        } else {
            DOWNSLOPE_STICK_JOG
        };
        self.base_mut().set_floor_snap_length(snap);
        self.try_step_up_predictive(Vector3::new(locomotion_x, 0.0, locomotion_z), dt);
        let vy_before_move = self.base().get_velocity().y;
        // Captured after the predictive pass: a committed rise changes y, and comparing against
        // the pre-rise position would undo legitimate steps.
        let y_before_move = self.base().get_global_position().y;
        self.base_mut().move_and_slide();
        // PhysX slope limiter, positional: if the move itself gained height with no upward
        // velocity (gravity only), the slope slide carried the capsule, and the recovery ratchet
        // climbs steep ramps positionally. Allowed only when a walkable contact exists;
        // jumps/pads set vy > 0 before the move and never match. Without this a 46 deg ramp is
        // climbable (Unity blocks it).
        if self.base().get_global_position().y > y_before_move + 0.0005 && vy_before_move <= 0.0 {
            let mut walkable_contact = false;
            let mut any_contact = false;
            for i in 0..self.base().get_slide_collision_count() {
                any_contact = true;
                if let Some(collision) = self.base().get_slide_collision(i) {
                    if collision.get_normal().y >= WALKABLE_NORMAL_Y {
                        walkable_contact = true;
                    }
                }
            }
            if any_contact && !walkable_contact {
                let mut position = self.base().get_global_position();
                position.y = y_before_move;
                self.base_mut().set_global_position(position);
                let mut v = self.base().get_velocity();
                v.y = v.y.min(0.0);
                self.base_mut().set_velocity(v);
            }
        }
        let moved_xz =
            (to_xz(self.base().get_global_position()) - to_xz(self.last_position)).length();
        self.try_step_up(Vector3::new(locomotion_x, 0.0, locomotion_z), moved_xz);
        // Step-up re-arming: the capsule re-arms when it stands on WALKABLE support, a staircase
        // tread, a step top, flat ground (even resting on Jolt's speculative margin cloud, where
        // is_on_floor() never fires, or pressed against the next riser, which is fine: the tread
        // holds you). On a steep ramp the support under the feet IS the ramp (un-walkable
        // normal), and mid-air there is no support, so ramp faces never re-arm: no band-chaining
        // while pressing, sliding or hopping.
        if (!self.step_armed || self.step_pending) && self.has_walkable_support() {
            self.step_armed = true;
            self.step_pending = false;
        }
        let mut position = self.base().get_position();
        position.y = position.y.max(0.0);
        self.base_mut().set_position(position);
        let final_position = self.base().get_global_position();
        if let Some(avatar) = &mut self.avatar {
            avatar.set_global_position(final_position);
        }

        // Restore the locomotion-only XZ; external_velocity carries its own state and is
        // re-added next frame.
        let mut velocity = self.base().get_velocity();
        velocity.x = locomotion_x;
        velocity.z = locomotion_z;
        // Restore velocity.y unless a floor/ceiling collision already zeroed it.
        if !self.base().is_on_floor() && !self.base().is_on_ceiling() {
            velocity.y -= external_y_for_move;
        }
        self.base_mut().set_velocity(velocity);

        self.update_crosshair_screen_position(dt);
        self.update_avatar_raycast_to_crosshair();
    }

    /// Folds scene-driven force and impulses into `external_velocity`, then drag and clamp.
    /// The runner only emits these for the current parcel scene.
    fn apply_scene_physics(
        &mut self,
        dt: f32,
        external_acceleration: Vector3,
        impulses: &PackedVector3Array,
        on_floor: bool,
    ) {
        // Force XZ accumulates; force Y was already folded into the effective gravity.
        self.external_velocity.x += external_acceleration.x * dt;
        self.external_velocity.z += external_acceleration.z * dt;

        let mut velocity = self.base().get_velocity();
        let mut got_upward_impulse = false;
        for impulse in impulses.as_slice() {
            let delta_v = *impulse / CHARACTER_MASS;
            if delta_v.y > 0.0 {
                got_upward_impulse = true;
                // An upward impulse on a falling player clears gravity so jump pads launch.
                if velocity.y < 0.0 {
                    velocity.y = 0.0;
                }
            }
            self.external_velocity += delta_v;
        }
        self.base_mut().set_velocity(velocity);

        // An upward impulse ungrounds the player for the rest of this tick, or the grounded drag
        // and Y-zero below would cancel a jump-pad launch.
        let effective_on_floor = on_floor && !got_upward_impulse;

        // Viscous drag: v *= (1 - damping * dt).
        let mut damping = EXT_ENV_DRAG;
        if effective_on_floor {
            damping += EXT_GROUND_FRICTION;
        }
        self.external_velocity *= (1.0 - damping * dt).max(0.0);

        // Zero Y when grounded so landings do not micro-bounce.
        if effective_on_floor {
            self.external_velocity.y = 0.0;
        }

        // Snap tiny magnitudes to zero, clamp large ones.
        let sqr_mag = self.external_velocity.length_squared();
        if sqr_mag < EXT_VELOCITY_EPSILON_SQR {
            self.external_velocity = Vector3::ZERO;
        } else if sqr_mag > MAX_EXTERNAL_VELOCITY * MAX_EXTERNAL_VELOCITY {
            self.external_velocity = self.external_velocity.normalized() * MAX_EXTERNAL_VELOCITY;
        }
    }

    /// Live third-person crosshair target (settled-camera phase): CROSSHAIR_TOP_GAP_PX above the
    /// avatar's on-screen top edge, at its side edge's x, clamped inside the screen margin.
    /// Falls back to the placeholder anchor when the head is behind the camera.
    fn live_crosshair_target(&self, viewport_size: Vector2) -> Vector2 {
        let Some(camera) = &self.camera else {
            return CROSSHAIR_PLACEHOLDER_ANCHOR * viewport_size;
        };
        let top_world =
            self.base().get_global_position() + Vector3::new(0.0, AVATAR_TOP_HEIGHT, 0.0);
        if camera.is_position_behind(top_world) {
            return CROSSHAIR_PLACEHOLDER_ANCHOR * viewport_size;
        }
        let edge_world =
            top_world + camera.get_global_transform().basis.col_a() * AVATAR_HALF_WIDTH;
        let top_px = camera.unproject_position(top_world);
        let edge_px = camera.unproject_position(edge_world);
        let tracked = Vector2::new(edge_px.x, top_px.y - CROSSHAIR_TOP_GAP_PX);
        let margin = Vector2::new(CROSSHAIR_SCREEN_MARGIN, CROSSHAIR_SCREEN_MARGIN);
        tracked.clamp(margin, viewport_size - margin)
    }

    fn update_crosshair_screen_position(&mut self, dt: f32) {
        let Some(viewport) = self.base().get_viewport() else {
            return;
        };
        let viewport_size = viewport.get_visible_rect().size;
        let active = DclGlobal::singleton().bind().is_mobile
            && !globals::scene_runner()
                .bind()
                .get_raycast_use_cursor_position();
        let mode = self.current_camera_mode();
        if !active {
            // Desktop and cinematic own the crosshair; park it at center so re-entering mobile
            // gameplay starts from center, never from a stale point.
            self.crosshair_screen_pos = viewport_size * 0.5;
            self.crosshair_alpha = 1.0;
            self.crosshair_pos_initialized = false;
            self.crosshair_transition_active = false;
            self.crosshair_prev_mode = mode;
            return;
        }

        // Timed-lerp model (zero overshoot on device QA). A mode swap captures the current
        // position and plays deterministic smoothstep lerps: no spring chase, no prediction. The
        // timeline is in units of the camera tween (T=0.25s):
        //   3p -> 1p: glide to center over [0, T].
        //   1p -> 3p: track the live point invisibly, then fade in over [1.5T, 2.0T] (the
        //   collision clamp extends the real camera ~0.15s past the arm tween, so the live point
        //   is only stable by then).
        self.crosshair_transition_clock += dt;
        if mode != self.crosshair_prev_mode {
            self.crosshair_prev_mode = mode;
            if self.crosshair_pos_initialized {
                self.crosshair_transition_from = self.crosshair_screen_pos;
                self.crosshair_transition_clock = 0.0;
                self.crosshair_transition_active = true;
            }
        }

        if !self.crosshair_pos_initialized {
            self.crosshair_pos_initialized = true;
            self.crosshair_screen_pos = if mode == CameraMode::FirstPerson {
                viewport_size * 0.5
            } else {
                self.live_crosshair_target(viewport_size)
            };
            return;
        }

        if self.crosshair_transition_active {
            let t = self.crosshair_transition_clock / CAMERA_MODE_TWEEN_TIME;
            if mode == CameraMode::FirstPerson {
                let w = smoothstep(0.0, 1.0, t.min(1.0));
                self.crosshair_screen_pos =
                    self.crosshair_transition_from.lerp(viewport_size * 0.5, w);
                if w >= 1.0 {
                    self.crosshair_transition_active = false;
                }
            } else {
                self.crosshair_screen_pos = self.live_crosshair_target(viewport_size);
                self.crosshair_alpha = smoothstep(0.0, 1.0, ((t - 1.5) / 0.5).min(1.0));
                if t >= 2.0 {
                    self.crosshair_transition_active = false;
                }
            }
        } else if mode == CameraMode::FirstPerson {
            self.crosshair_screen_pos = viewport_size * 0.5;
            self.crosshair_alpha = 1.0;
        } else {
            self.crosshair_screen_pos = self.live_crosshair_target(viewport_size);
            self.crosshair_alpha = 1.0;
        }
    }

    /// Aims the avatar outline / view-profile raycast at the crosshair. The ray keeps its origin
    /// at the camera (the own avatar is excluded via its removed ClickArea) and only the
    /// direction changes. Mobile only; desktop and cinematic keep the tscn default. Writes happen
    /// only on state change, not every physics tick.
    fn update_avatar_raycast_to_crosshair(&mut self) {
        let active = DclGlobal::singleton().bind().is_mobile
            && !globals::scene_runner()
                .bind()
                .get_raycast_use_cursor_position();
        let Some(ray) = &mut self.avatar_raycast else {
            return;
        };
        if !active {
            if self.avatar_raycast_crosshair_active {
                self.avatar_raycast_crosshair_active = false;
                ray.set_position(Vector3::ZERO);
                ray.set_target_position(AVATAR_RAYCAST_DEFAULT_TARGET);
            }
            return;
        }
        self.avatar_raycast_crosshair_active = true;
        let Some(camera) = &self.camera else {
            return;
        };
        let dir = camera.project_ray_normal(self.crosshair_screen_pos);
        ray.set_position(Vector3::ZERO);
        let target = camera.get_global_position() + dir * AVATAR_RAYCAST_DEFAULT_TARGET.length();
        let local = ray.to_local(target);
        ray.set_target_position(local);
    }

    /// Direct support check for step-up re-arming. `is_on_floor()` misses rests on Jolt's
    /// speculative margin cloud (contacts cancel motion without ever registering floor), so when
    /// it reports nothing a short ray confirms: rays return normals, `cast_motion` in this build
    /// does not.
    pub(super) fn has_walkable_support(&mut self) -> bool {
        if self.base().is_on_floor() {
            return true; // floor contacts are within floor_max_angle by definition
        }
        let position = self.base().get_global_position();
        if position.y <= REALM_FLOOR_EPS {
            return true; // clamp-held realm floor
        }
        let Some(world) = self.base().get_world_3d() else {
            return false;
        };
        let Some(mut space) = world.get_direct_space_state() else {
            return false;
        };
        let Some(mut query) = PhysicsRayQueryParameters3D::create(
            position + Vector3::new(0.0, 0.05, 0.0),
            position + Vector3::new(0.0, -0.1, 0.0),
        ) else {
            return false;
        };
        query.set_collision_mask(self.base().get_collision_mask());
        query.set_exclude(&self.raycast_exclude);
        let hit = space.intersect_ray(&query);
        hit.get("normal")
            .is_some_and(|normal| normal.to::<Vector3>().y >= WALKABLE_NORMAL_Y)
    }
}

fn set_global_camera_mode(mode: CameraMode) {
    DclGlobal::singleton().call("set_camera_mode", &[mode.to_i32().to_variant()]);
}

fn explorer_has_focus() -> bool {
    DclGlobal::singleton()
        .call("explorer_has_focus", &[])
        .try_to::<bool>()
        .unwrap_or(false)
}

fn play_ui_sound(name: &str) {
    crate::ui::ui_sounds::play_sound(name, false, CAMERA_TRANSITION_SOUND_DB);
}

fn collect_collider_rids(node: &Gd<Node>, out: &mut Vec<Rid>) {
    if let Ok(body) = node.clone().try_cast::<CollisionObject3D>() {
        out.push(body.get_rid());
    }
    for child in node.get_children().iter_shared() {
        collect_collider_rids(&child, out);
    }
}
