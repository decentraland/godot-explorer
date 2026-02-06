use godot::prelude::*;

use crate::dcl::components::proto_components::sdk::components::PbAvatarLocomotionSettings;

/// Default locomotion values (matching current player.gd defaults)
pub const DEFAULT_WALK_SPEED: f32 = 1.5;
pub const DEFAULT_JOG_SPEED: f32 = 8.0;
pub const DEFAULT_RUN_SPEED: f32 = 11.0;
pub const DEFAULT_JUMP_HEIGHT: f32 = 1.8;
pub const DEFAULT_RUN_JUMP_HEIGHT: f32 = 1.8;
pub const DEFAULT_HARD_LANDING_COOLDOWN: f32 = 0.0;

#[derive(GodotClass)]
#[class(base=RefCounted)]
pub struct DclLocomotionSettings {
    #[export]
    walk_speed: f32,

    #[export]
    jog_speed: f32,

    #[export]
    run_speed: f32,

    #[export]
    jump_height: f32,

    #[export]
    run_jump_height: f32,

    #[export]
    hard_landing_cooldown: f32,
}

#[godot_api]
impl IRefCounted for DclLocomotionSettings {
    fn init(_base: Base<RefCounted>) -> Self {
        Self {
            walk_speed: DEFAULT_WALK_SPEED,
            jog_speed: DEFAULT_JOG_SPEED,
            run_speed: DEFAULT_RUN_SPEED,
            jump_height: DEFAULT_JUMP_HEIGHT,
            run_jump_height: DEFAULT_RUN_JUMP_HEIGHT,
            hard_landing_cooldown: DEFAULT_HARD_LANDING_COOLDOWN,
        }
    }
}

#[godot_api]
impl DclLocomotionSettings {
    /// Get a new instance with default locomotion settings
    #[func]
    pub fn create_default() -> Gd<DclLocomotionSettings> {
        Gd::default()
    }
}

impl DclLocomotionSettings {
    /// Update from proto component (uses constants for unset fields)
    pub fn set_from_proto(&mut self, proto: &PbAvatarLocomotionSettings) {
        self.walk_speed = proto.walk_speed.unwrap_or(DEFAULT_WALK_SPEED);
        self.jog_speed = proto.jog_speed.unwrap_or(DEFAULT_JOG_SPEED);
        self.run_speed = proto.run_speed.unwrap_or(DEFAULT_RUN_SPEED);
        self.jump_height = proto.jump_height.unwrap_or(DEFAULT_JUMP_HEIGHT);
        self.run_jump_height = proto.run_jump_height.unwrap_or(DEFAULT_RUN_JUMP_HEIGHT);
        self.hard_landing_cooldown = proto
            .hard_landing_cooldown
            .unwrap_or(DEFAULT_HARD_LANDING_COOLDOWN);
    }

    /// Reset to defaults
    pub fn reset_to_defaults(&mut self) {
        self.walk_speed = DEFAULT_WALK_SPEED;
        self.jog_speed = DEFAULT_JOG_SPEED;
        self.run_speed = DEFAULT_RUN_SPEED;
        self.jump_height = DEFAULT_JUMP_HEIGHT;
        self.run_jump_height = DEFAULT_RUN_JUMP_HEIGHT;
        self.hard_landing_cooldown = DEFAULT_HARD_LANDING_COOLDOWN;
    }
}
