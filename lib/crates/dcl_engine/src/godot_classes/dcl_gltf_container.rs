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
    #[var(pub)]
    pub(crate) dcl_gltf_src: GString,

    #[export]
    #[var(pub)]
    pub(crate) dcl_scene_id: i32,

    #[export]
    #[var(pub)]
    pub(crate) dcl_entity_id: i32,

    #[export]
    #[var(pub)]
    pub(crate) dcl_visible_cmask: i32,

    #[export]
    #[var(pub)]
    pub(crate) dcl_invisible_cmask: i32,

    #[export(enum = (Unknown, Loading, NotFound, FinishedWithError, Finished))]
    #[var(pub)]
    pub(crate) dcl_gltf_loading_state: GltfContainerLoadingState,

    #[export]
    #[var(pub)]
    pub(crate) dcl_pending_node: Option<Gd<Node>>,

    /// Content hash of the source, empty before it resolves.
    #[var(pub)]
    pub(crate) dcl_gltf_hash: GString,
    pub(crate) optimized: bool,
    /// Set by `force_reload_gltf`: the next request bypasses ResourceLoader's cache, which would
    /// serve the pre-edit model on a reused .scn path.
    pub(crate) force_fresh_load: bool,
    /// Once the entity is known to move, every collider spawns KINEMATIC.
    pub(crate) kinematic_requested: bool,
    /// The hash this container is registered under in the loading coordinator.
    pub(crate) requested_hash: String,

    base: Base<Node3D>,
}

#[godot_api]
impl DclGltfContainer {
    /// Signal emitted when the entity has moved enough times to require
    /// switching colliders from STATIC to KINEMATIC mode
    #[signal]
    pub fn switch_to_kinematic();

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
    pub fn check_animations(&mut self) {
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
            dcl_gltf_hash: GString::new(),
            optimized: false,
            force_fresh_load: false,
            kinematic_requested: false,
            requested_hash: String::new(),
            base,
        }
    }

    fn ready(&mut self) {
        // Rust flagged the entity as kinematic (an active tween).
        if self.base().has_meta("kinematic_requested") {
            self.kinematic_requested = true;
        }
        let this = self.to_gd();
        self.signals()
            .switch_to_kinematic()
            .connect_other(&this, |this: &mut Self| this.on_switch_to_kinematic());
        self.defer_load();
    }

    fn enter_tree(&mut self) {
        // Re-parenting an entity (a normal SDK operation) takes its GltfContainer out of the tree
        // and back in, and `ready` only runs once: without this the load is never re-requested,
        // the container sits in LOADING behind no group, Rust keeps counting it in
        // `scene.gltf_loading`, and the loading screen stays pinned until the 120 s timeout.
        // Only re-request when genuinely mid-load and detached: a FINISHED or errored container
        // must not reload.
        if self.dcl_gltf_loading_state == GltfContainerLoadingState::Loading
            && self.requested_hash.is_empty()
        {
            self.defer_load();
        }
    }

    fn exit_tree(&mut self) {
        // Detaches from the shared load group. Leaving the tree is not finishing the load: the
        // state stays LOADING so a re-parented container resumes rather than reporting a false
        // FINISHED. Clearing `requested_hash` marks it as attached to no group: `enter_tree`
        // uses exactly that to re-request.
        if !self.requested_hash.is_empty() {
            let hash = std::mem::take(&mut self.requested_hash);
            crate::scene_runner::gltf_loading::unregister(&self.to_gd(), &hash);
        }
        // A pending orphan node (legacy path).
        if let Some(mut pending) = self.dcl_pending_node.take() {
            pending.queue_free();
        }
    }
}
