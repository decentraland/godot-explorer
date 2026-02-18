use godot::classes::AnimationPlayer;
use godot::prelude::*;

use crate::dcl::components::proto_components::sdk::components::{PbAnimationState, PbAnimator};
use crate::dcl::components::SceneEntityId;
use crate::dcl::SceneId;

use super::animator_controller::{apply_anims, DUMMY_ANIMATION_NAME};
use super::dcl_global::DclGlobal;

#[derive(Clone, Var, GodotConvert, Export, PartialEq, Debug)]
#[godot(via=i32)]
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

    #[export]
    dcl_pending_node: Option<Gd<Node>>,

    base: Base<Node3D>,
}

#[godot_api]
impl DclGltfContainer {
    /// Signal emitted when the entity has moved enough times to require
    /// switching colliders from STATIC to KINEMATIC mode
    #[signal]
    fn switch_to_kinematic();

    #[func]
    pub fn get_gltf_resource(&self) -> Option<Gd<Node3D>> {
        let child_count = self.base().get_child_count();
        if child_count == 0 {
            return None;
        }

        for i in 0..child_count {
            if let Some(child) = self.base().get_child(i) {
                if let Ok(node) = child.try_cast::<Node3D>() {
                    return Some(node);
                }
            }
        }

        None
    }

    #[func]
    fn check_animations(&mut self) {
        if self.dcl_gltf_loading_state != GltfContainerLoadingState::Finished {
            return;
        }

        let Some(gltf_container_node) = self.get_gltf_resource() else {
            return;
        };

        let Some(animation_player) =
            gltf_container_node.try_get_node_as::<AnimationPlayer>("AnimationPlayer")
        else {
            return;
        };

        let entity_id = SceneEntityId::from_i32(self.dcl_entity_id);

        let global = DclGlobal::singleton();
        let scene_runner = global.bind().get_scene_runner();
        let dcl_scene_runner = scene_runner.bind();
        if let Some(scene) = dcl_scene_runner.get_scene(&SceneId(self.dcl_scene_id)) {
            if let Some(pending_animator_value) = scene.dup_animator.get(&entity_id) {
                apply_anims(gltf_container_node, pending_animator_value);
            } else {
                let animation_list = animation_player.get_animation_list();
                let animation_name = if animation_list.len() > 1 {
                    let value = animation_list.get(0).as_ref().unwrap().to_string();
                    if value == DUMMY_ANIMATION_NAME {
                        animation_list.get(1).as_ref().unwrap().to_string()
                    } else {
                        value
                    }
                } else if !animation_list.is_empty() {
                    animation_list.get(0).as_ref().unwrap().to_string()
                } else {
                    return;
                };

                apply_anims(
                    gltf_container_node,
                    &PbAnimator {
                        states: vec![PbAnimationState {
                            clip: animation_name,
                            playing: Some(true),
                            r#loop: Some(true),
                            ..Default::default()
                        }],
                    },
                );
            }
        }
    }

    pub fn get_state(&self) -> GltfContainerLoadingState {
        self.dcl_gltf_loading_state.clone()
    }
}

#[godot_api]
impl INode3D for DclGltfContainer {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            dcl_gltf_src: "".into(),
            dcl_scene_id: SceneId::INVALID.0,
            dcl_visible_cmask: 0,
            dcl_invisible_cmask: 3,
            dcl_entity_id: SceneEntityId::INVALID.as_i32(),
            dcl_gltf_loading_state: GltfContainerLoadingState::Unknown,
            dcl_pending_node: None,
            base,
        }
    }
}
