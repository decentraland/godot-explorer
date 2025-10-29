use godot::prelude::*;

use crate::dcl::components::{
    proto_components::sdk::components::{
        common::camera_transition::TransitionMode, PbVirtualCamera,
    },
    SceneEntityId,
};

#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub struct DclVirtualCamera {
    // Used to mute and restore the volume
    #[export]
    entity_id: i32,

    #[export]
    look_at_entity_id: i32,

    #[export]
    transition_time: f32,

    #[export]
    transition_speed: f32,
}

impl DclVirtualCamera {
    pub fn clear(&mut self) {
        self.entity_id = 0;
        self.look_at_entity_id = 0;
        self.transition_speed = 0.0;
        self.transition_time = 0.0;
    }

    pub fn set_transform(&mut self, entity_id: &SceneEntityId) {
        self.clear();
        self.entity_id = entity_id.as_i32();
    }

    pub fn set_virtual_camera(
        &mut self,
        entity_id: &SceneEntityId,
        virtual_camera_value: &PbVirtualCamera,
    ) {
        self.clear();
        self.entity_id = entity_id.as_i32();
        self.look_at_entity_id = virtual_camera_value.look_at_entity.unwrap_or(0) as i32;

        match &virtual_camera_value
            .default_transition
            .as_ref()
            .and_then(|x| x.transition_mode.as_ref())
        {
            Some(TransitionMode::Time(t)) => {
                self.transition_time = *t;
            }
            Some(TransitionMode::Speed(v)) => {
                self.transition_speed = *v;
            }
            None => {}
        }
    }
}
