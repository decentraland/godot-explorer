use godot::engine::{AnimationPlayer, Node3D};
use godot::prelude::*;

use crate::dcl::components::proto_components::sdk::components::{PbAnimationState, PbAnimator};
use crate::dcl::components::SceneEntityId;
use crate::dcl::SceneId;
use crate::scene_runner::components::animator::apply_animator_value;

use super::dcl_global::DclGlobal;

#[repr(i32)]
#[derive(Clone, Property, Export, PartialEq, Debug)]
pub enum GltfContainerLoadingState {
    Unknown = 0,
    #[allow(dead_code)]
    Loading = 1,
    NotFound = 2,
    FinishedWithError = 3,
    Finished = 4,
}

impl GltfContainerLoadingState {
    pub fn to_proto(
        &self,
    ) -> crate::dcl::components::proto_components::sdk::components::common::LoadingState {
        match self {
            Self::Unknown => crate::dcl::components::proto_components::sdk::components::common::LoadingState::Unknown,
            Self::Loading => crate::dcl::components::proto_components::sdk::components::common::LoadingState::Loading,
            Self::NotFound => crate::dcl::components::proto_components::sdk::components::common::LoadingState::NotFound,
            Self::FinishedWithError => crate::dcl::components::proto_components::sdk::components::common::LoadingState::FinishedWithError,
            Self::Finished => crate::dcl::components::proto_components::sdk::components::common::LoadingState::Finished,
        }
    }

    pub fn from_proto(
        proto: crate::dcl::components::proto_components::sdk::components::common::LoadingState,
    ) -> Self {
        match proto {
            crate::dcl::components::proto_components::sdk::components::common::LoadingState::Unknown => Self::Unknown,
            crate::dcl::components::proto_components::sdk::components::common::LoadingState::Loading => Self::Loading,
            crate::dcl::components::proto_components::sdk::components::common::LoadingState::NotFound => Self::NotFound,
            crate::dcl::components::proto_components::sdk::components::common::LoadingState::FinishedWithError => Self::FinishedWithError,
            crate::dcl::components::proto_components::sdk::components::common::LoadingState::Finished => Self::Finished,
        }
    }

    pub fn from_i32(value: i32) -> Self {
        match value {
            0 => Self::Unknown,
            1 => Self::Loading,
            2 => Self::NotFound,
            3 => Self::FinishedWithError,
            4 => Self::Finished,
            _ => Self::Unknown,
        }
    }

    pub fn to_i32(&self) -> i32 {
        match self {
            Self::Unknown => 0,
            Self::Loading => 1,
            Self::NotFound => 2,
            Self::FinishedWithError => 3,
            Self::Finished => 4,
        }
    }
}

#[derive(GodotClass)]
#[class(base=Node3D)]
pub struct DclGltfContainer {
    #[export]
    dcl_gltf_src: GString,

    #[export]
    dcl_scene_id: i32,

    #[export]
    dcl_entity_id: i32,

    #[export]
    dcl_visible_cmask: i32,

    #[export]
    dcl_invisible_cmask: i32,

    #[export(enum = (Unknown, Loading, NotFound, FinishedWithError, Finished))]
    dcl_gltf_loading_state: GltfContainerLoadingState,

    #[base]
    base: Base<Node3D>,
}

fn get_animation_player(godot_entity_node: &Base<Node3D>) -> Option<Gd<AnimationPlayer>> {
    godot_entity_node
        .get_child(0)?
        .try_get_node_as::<AnimationPlayer>("AnimationPlayer")
}

#[godot_api]
impl DclGltfContainer {
    #[func]
    fn check_animations(&mut self) {
        if self.dcl_gltf_loading_state != GltfContainerLoadingState::Finished {
            return;
        }
        let Some(animation_player) = get_animation_player(&self.base) else {
            return;
        };
        let entity_id = SceneEntityId::from_i32(self.dcl_entity_id);

        let global = DclGlobal::singleton();
        let scene_runner = global.bind().get_scene_runner();
        let dcl_scene_runner = scene_runner.bind();
        if let Some(scene) = dcl_scene_runner.get_scene(&SceneId(self.dcl_scene_id)) {
            if let Some(pending_animator_value) = scene.dup_animator.get(&entity_id) {
                apply_animator_value(pending_animator_value, animation_player);
            } else {
                let animation_list = animation_player.get_animation_list();
                if !animation_list.is_empty() {
                    let animation_name = animation_list.get(0).into();
                    apply_animator_value(
                        &PbAnimator {
                            states: vec![PbAnimationState {
                                clip: animation_name,
                                playing: Some(true),
                                r#loop: Some(true),
                                should_reset: Some(true),
                                ..Default::default()
                            }],
                        },
                        animation_player,
                    );
                }
            }
        }
    }

    pub fn get_state(&self) -> GltfContainerLoadingState {
        self.dcl_gltf_loading_state.clone()
    }
}

#[godot_api]
impl INode for DclGltfContainer {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            dcl_gltf_src: "".into(),
            dcl_scene_id: SceneId::INVALID.0,
            dcl_visible_cmask: 0,
            dcl_invisible_cmask: 3,
            dcl_entity_id: SceneEntityId::INVALID.as_i32(),
            dcl_gltf_loading_state: GltfContainerLoadingState::Unknown,
            base,
        }
    }
}
