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

#[derive(Default)]
struct LerpState {
    initial_position: Vector3,
    target_position: Vector3,
    factor: f32,
    initial_velocity_y: f32,
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
    blocked: bool,

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

    #[signal]
    fn emote_triggered(id: GString, looping: bool);

    #[func]
    pub fn set_target_position(&mut self, new_target: Transform3D) {
        let mut diff_xz_plane = new_target.origin - self.lerp_state.target_position;
        let y_velocity = 10.0 * diff_xz_plane.y; // divide by 0.1s
        diff_xz_plane.y = 0.0;
        let target_forward_distance = diff_xz_plane.length();

        // TODO: define const with these values
        self.walk = target_forward_distance < 0.4 && target_forward_distance > 0.01;
        self.run = target_forward_distance >= 0.65;
        self.jog = !(self.walk || self.run) && target_forward_distance > 0.01;
        self.rise = y_velocity > 1.0;
        self.fall = y_velocity < -1.0;
        self.land = !self.rise && !self.fall;

        self.lerp_state.initial_position = self.lerp_state.target_position;
        self.lerp_state.target_position = new_target.origin;
        self.lerp_state.factor = 0.0;
        self.lerp_state.initial_velocity_y = y_velocity;

        let initial_position = self.lerp_state.initial_position;

        // TODO: check euler order
        self.base_mut()
            .set_global_rotation(new_target.basis.get_euler());
        self.base_mut().set_global_position(initial_position);

        self.update_parcel_position(self.lerp_state.target_position);
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
                self.lerp_state.factor += 10.0 * dt as f32;
                if self.lerp_state.factor < 1.0 {
                    if self.lerp_state.factor > 1.0 {
                        self.lerp_state.factor = 1.0;
                    }

                    let new_position = self
                        .lerp_state
                        .initial_position
                        .lerp(self.lerp_state.target_position, self.lerp_state.factor);

                    self.base_mut().set_global_position(new_position);
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
        self.base_mut()
            .call("set_hidden", &[value.to_variant()]);
    }
}
