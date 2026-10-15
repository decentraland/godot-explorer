use godot::classes::{
    display_server::Feature, input::MouseMode, Camera3D, DisplayServer, ICamera3D, Input,
    InputEvent, InputEventKey, InputEventMouseButton, InputEventMouseMotion,
};
use godot::global::{Key, MouseButton};
use godot::prelude::*;

// Modifier keys' speed multiplier.
const SHIFT_MULTIPLIER: f32 = 2.5;
const ALT_MULTIPLIER: f32 = 1.0 / SHIFT_MULTIPLIER;

/// A fly camera for the offline renderer and the avatar test scene: right mouse captures the
/// pointer, WASD/QE move, the wheel scales the speed.
#[derive(GodotClass)]
#[class(init, base=Camera3D)]
pub struct FreeLookCamera {
    base: Base<Camera3D>,
    #[export(range = (0.0, 1.0))]
    #[init(val = 0.25)]
    sensitivity: f32,

    mouse_position: Vector2,
    total_pitch: f32,

    velocity: Vector3,
    #[init(val = 30.0)]
    acceleration: f32,
    #[init(val = -10.0)]
    deceleration: f32,
    #[init(val = 4.0)]
    vel_multiplier: f32,

    w: bool,
    s: bool,
    a: bool,
    d: bool,
    q: bool,
    e: bool,
    shift: bool,
    alt: bool,
}

#[godot_api]
impl ICamera3D for FreeLookCamera {
    fn input(&mut self, event: Gd<InputEvent>) {
        if let Ok(motion) = event.clone().try_cast::<InputEventMouseMotion>() {
            self.mouse_position = motion.get_relative();
        }
        if let Ok(button) = event.clone().try_cast::<InputEventMouseButton>() {
            match button.get_button_index() {
                // Rotation only while the right button is down.
                MouseButton::RIGHT => {
                    if DisplayServer::singleton().has_feature(Feature::MOUSE) {
                        Input::singleton().set_mouse_mode(if button.is_pressed() {
                            MouseMode::CAPTURED
                        } else {
                            MouseMode::VISIBLE
                        });
                    }
                }
                MouseButton::WHEEL_UP => {
                    self.vel_multiplier = (self.vel_multiplier * 1.1).clamp(0.2, 100.0);
                }
                MouseButton::WHEEL_DOWN => {
                    self.vel_multiplier = (self.vel_multiplier / 1.1).clamp(0.2, 100.0);
                }
                _ => {}
            }
        }
        if let Ok(key) = event.try_cast::<InputEventKey>() {
            let pressed = key.is_pressed();
            match key.get_keycode() {
                Key::W => self.w = pressed,
                Key::S => self.s = pressed,
                Key::A => self.a = pressed,
                Key::D => self.d = pressed,
                Key::Q => self.q = pressed,
                Key::E => self.e = pressed,
                _ => {}
            }
        }
    }

    fn process(&mut self, delta: f64) {
        self.update_mouselook();
        self.update_movement(delta as f32);
    }
}

impl FreeLookCamera {
    fn update_movement(&mut self, delta: f32) {
        let axis = |positive: bool, negative: bool| positive as i32 as f32 - negative as i32 as f32;
        let direction = Vector3::new(
            axis(self.d, self.a),
            axis(self.e, self.q),
            axis(self.s, self.w),
        );

        // The change in velocity due to the desired direction and "drag": a constant
        // acceleration that brings the velocity to 0.
        let offset = super::rig::normalized_or_zero(direction)
            * self.acceleration
            * self.vel_multiplier
            * delta
            + super::rig::normalized_or_zero(self.velocity)
                * self.deceleration
                * self.vel_multiplier
                * delta;

        let mut speed_multi = 1.0;
        if self.shift {
            speed_multi *= SHIFT_MULTIPLIER;
        }
        if self.alt {
            speed_multi *= ALT_MULTIPLIER;
        }

        if direction == Vector3::ZERO && offset.length_squared() > self.velocity.length_squared() {
            // Stop outright to prevent jitter from imperfect deceleration.
            self.velocity = Vector3::ZERO;
        } else {
            let limit = self.vel_multiplier;
            self.velocity.x = (self.velocity.x + offset.x).clamp(-limit, limit);
            self.velocity.y = (self.velocity.y + offset.y).clamp(-limit, limit);
            self.velocity.z = (self.velocity.z + offset.z).clamp(-limit, limit);
            let step = self.velocity * delta * speed_multi;
            self.base_mut().translate(step);
        }
    }

    fn update_mouselook(&mut self) {
        // Only rotates while the mouse is captured.
        if Input::singleton().get_mouse_mode() != MouseMode::CAPTURED {
            return;
        }
        self.mouse_position *= self.sensitivity;
        let yaw = self.mouse_position.x;
        let mut pitch = self.mouse_position.y;
        self.mouse_position = Vector2::ZERO;

        // Prevents looking up/down too far.
        pitch = pitch.clamp(-90.0 - self.total_pitch, 90.0 - self.total_pitch);
        self.total_pitch += pitch;

        self.base_mut().rotate_y((-yaw).to_radians());
        self.base_mut()
            .rotate_object_local(Vector3::RIGHT, (-pitch).to_radians());
    }
}
