use godot::engine::Node3D;
use godot::prelude::*;

use crate::dcl::SceneId;

use super::dcl_global::DclGlobal;

#[derive(Property, Export)]
#[repr(i32)]
pub enum AvatarMovementType {
    ExternalController = 0,
    LerpTwoPoints = 1,
}

#[derive(Default)]
struct LerpState {
    initial_position: Vector3,
    target_position: Vector3,
    factor: f32,
    velocity_y: f32,
    initial_velocity_y: f32,
}

#[derive(GodotClass)]
#[class(base=Node3D)]
pub struct DclAvatar {
    #[export]
    movement_type: AvatarMovementType,

    #[var]
    current_parcel_scene_id: i32,

    #[var]
    current_parcel_position: Vector2i,

    #[export]
    walking: bool,
    #[export]
    running: bool,
    #[export]
    rising: bool,
    #[export]
    falling: bool,

    lerp_state: LerpState,
    #[base]
    base: Base<Node3D>,
}

#[godot_api]
impl INode3D for DclAvatar {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            movement_type: AvatarMovementType::ExternalController,
            current_parcel_scene_id: SceneId::INVALID.0,
            current_parcel_position: Vector2i::new(i32::MAX, i32::MAX),
            lerp_state: Default::default(),
            base,
            walking: false,
            running: false,
            rising: true,
            falling: false,
        }
    }
}

#[godot_api]
impl DclAvatar {
    #[signal]
    fn change_parcel_position(&self, parcel_position: Vector2) {}

    #[signal]
    fn change_scene_id(&self, new_scene_id: i32, prev_scene_id: i32) {}

    #[func]
    pub fn set_target_position(&mut self, new_target: Transform3D) {
        let mut diff_xz_plane = new_target.origin - self.lerp_state.target_position;
        let y_velocity = 10.0 * diff_xz_plane.y;
        diff_xz_plane.y = 0.0;
        let target_forward_distance = diff_xz_plane.length();

        // TODO: define const with these values
        self.walking = target_forward_distance < 0.6 && target_forward_distance > 0.01;
        self.running = target_forward_distance >= 0.6;
        self.rising = y_velocity > 0.25;
        self.falling = y_velocity < -0.25;

        self.lerp_state.initial_position = self.lerp_state.target_position;
        self.lerp_state.target_position = new_target.origin;
        self.lerp_state.factor = 0.0;
        self.lerp_state.velocity_y = y_velocity;
        self.lerp_state.initial_velocity_y = y_velocity;

        // TODO: check euler order
        self.base
            .set_global_rotation(new_target.basis.to_euler(EulerOrder::YXZ));
        self.base
            .set_global_position(self.lerp_state.initial_position);

        self.update_parcel_position(self.lerp_state.target_position);
    }

    // This function is called when a parcel scene is created,
    //  it handles the corner case where the avatar is already in the parcel
    //  that is being created
    pub fn on_parcel_scenes_changed(&mut self) {
        let godot_parcel_position = self.base.get_global_position() / 16.0;
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
            self.base.call_deferred(
                "emit_signal".into(),
                &[
                    "change_scene_id".to_variant(),
                    scene_id.to_variant(),
                    prev_scene_id.to_variant(),
                ],
            );
        }
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
            self.base.call_deferred(
                "emit_signal".into(),
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
                self.base.call_deferred(
                    "emit_signal".into(),
                    &[
                        "change_scene_id".to_variant(),
                        scene_id.to_variant(),
                        prev_scene_id.to_variant(),
                    ],
                );
            }
            return true;
        } else if self.current_parcel_scene_id == SceneId::INVALID.0 {
            let scene_runner = DclGlobal::singleton().bind().get_scene_runner();
            let scene_id: i32 = scene_runner
                .bind()
                .get_scene_id_by_parcel_position(parcel_position);

            if scene_id != SceneId::INVALID.0 {
                let prev_scene_id = self.current_parcel_scene_id;
                self.current_parcel_scene_id = scene_id;
                self.base.call_deferred(
                    "emit_signal".into(),
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
                    self.update_parcel_position(self.base.get_global_position());
                }
            }
            AvatarMovementType::LerpTwoPoints => {
                self.lerp_state.factor += 10.0 * dt as f32;
                if self.lerp_state.factor < 1.0 {
                    if self.lerp_state.factor > 1.0 {
                        self.lerp_state.factor = 1.0;
                    }

                    self.base.set_global_position(
                        self.lerp_state
                            .initial_position
                            .lerp(self.lerp_state.target_position, self.lerp_state.factor),
                    );

                    if self.lerp_state.initial_velocity_y > 0.25 {
                        self.lerp_state.velocity_y -= 55.0 * dt as f32;
                        self.rising = self.lerp_state.velocity_y > 0.25;
                        self.falling = self.lerp_state.velocity_y < -0.25;
                    }
                }
            }
        }
    }
}
