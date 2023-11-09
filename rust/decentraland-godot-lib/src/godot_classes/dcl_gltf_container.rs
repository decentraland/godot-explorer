use godot::engine::Node3D;
use godot::prelude::*;

#[repr(i32)]
#[derive(Property, Export, PartialEq, Debug)]
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
    dcl_gltf_src: GodotString,

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
    _base: Base<Node3D>,
}

#[godot_api]
impl DclGltfContainer {}

#[godot_api]
impl NodeVirtual for DclGltfContainer {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            dcl_gltf_src: "".into(),
            dcl_scene_id: -1,
            dcl_visible_cmask: 0,
            dcl_invisible_cmask: 3,
            dcl_entity_id: -1,
            dcl_gltf_loading_state: GltfContainerLoadingState::Unknown,
            _base: base,
        }
    }
}
