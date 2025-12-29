use crate::{
    dcl::{
        components::{material::DclMaterial, proto_components, SceneComponentId, SceneEntityId},
        SceneId,
    },
    godot_classes::{
        dcl_node_entity_3d::DclNodeEntity3d, dcl_scene_node::DclSceneNode,
        dcl_ui_control::DclUiControl,
    },
    realm::scene_definition::SceneEntityDefinition,
};
use godot::prelude::*;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::Arc,
};

use super::components::ui::{scene_ui::UiResults, style::UiTransform};
use crate::av::backend::{AudioSink, BackendType};
use godot::classes::ImageTexture;

pub struct GodotDclScene {
    pub entities: HashMap<SceneEntityId, GodotEntityNode>,

    pub root_node_3d: Gd<DclSceneNode>,
    pub hierarchy_dirty_3d: bool,
    pub hierarchy_changed_3d: bool,
    pub unparented_entities_3d: HashSet<SceneEntityId>,

    pub parent_node_ui: Gd<DclUiControl>,
    pub root_node_ui: Gd<DclUiControl>,
    pub ui_entities: HashSet<SceneEntityId>,
    pub hidden_dirty: HashMap<SceneComponentId, HashSet<SceneEntityId>>,
    pub ui_visible: bool,

    pub ui_results: Rc<RefCell<UiResults>>,
}

/// Video player data for CRDT event tracking.
/// State is polled from the video player node each frame.
pub struct VideoPlayerData {
    /// The video source URL
    pub source: String,
    /// The backend type being used
    pub backend_type: BackendType,
    /// Texture for LiveKit video frames (only used for LiveKit backend)
    pub texture: Option<Gd<ImageTexture>>,
    /// Event timestamp counter for CRDT events
    pub timestamp: u32,

    // Last known state - used to detect changes and generate events
    /// Last known video state (matches VIDEO_STATE_* constants in dcl_video_player.rs)
    pub last_state: i32,
    /// Last known video position in seconds
    pub last_position: f64,
    /// Last known video length in seconds
    pub last_length: f64,

    // Last requested values from CRDT - used to detect changes
    /// Last requested position (for seeking)
    pub last_requested_position: f32,
    /// Last requested playback rate
    pub last_playback_rate: f32,
}

impl VideoPlayerData {
    pub fn new(source: String, backend_type: BackendType) -> Self {
        Self {
            source,
            backend_type,
            texture: None,
            timestamp: 0,
            last_state: 0, // VIDEO_STATE_NONE
            last_position: 0.0,
            last_length: -1.0,
            last_requested_position: 0.0,
            last_playback_rate: 1.0,
        }
    }

    /// Create VideoPlayerData with a texture (for LiveKit backend)
    pub fn new_with_texture(
        source: String,
        backend_type: BackendType,
        texture: Gd<ImageTexture>,
    ) -> Self {
        Self {
            source,
            backend_type,
            texture: Some(texture),
            timestamp: 0,
            last_state: 0, // VIDEO_STATE_NONE
            last_position: 0.0,
            last_length: -1.0,
            last_requested_position: 0.0,
            last_playback_rate: 1.0,
        }
    }
}

pub struct UiNode {
    pub base_control: Gd<DclUiControl>,
    pub ui_transform: UiTransform,
    pub computed_parent: SceneEntityId,
    pub has_background: bool,
    pub text_size: Option<Vector2>,
}

impl UiNode {
    pub fn control_offset(&self) -> i32 {
        (if self.has_background { 1 } else { 0 }) + (if self.text_size.is_some() { 1 } else { 0 })
    }
}

pub struct GodotEntityNode {
    pub base_3d: Option<Gd<Node3D>>,
    pub desired_parent_3d: SceneEntityId,
    pub computed_parent_3d: SceneEntityId,
    pub material: Option<DclMaterial>,
    pub pointer_events: Option<proto_components::sdk::components::PbPointerEvents>,
    pub video_player_data: Option<VideoPlayerData>,
    pub audio_stream: Option<(String, AudioSink)>,

    pub base_ui: Option<UiNode>,
}

impl GodotEntityNode {
    fn new(base_3d: Option<Gd<Node3D>>, base_ui: Option<UiNode>) -> Self {
        Self {
            base_3d,
            desired_parent_3d: SceneEntityId::ROOT,
            computed_parent_3d: SceneEntityId::ROOT,

            base_ui,

            material: None,
            pointer_events: None,
            video_player_data: None,
            audio_stream: None,
        }
    }
}

impl GodotDclScene {
    pub fn new(
        scene_entity_definition: Arc<SceneEntityDefinition>,
        scene_id: &SceneId,
        parent_node_ui: Gd<DclUiControl>,
    ) -> Self {
        let mut root_node_3d =
            DclSceneNode::new_alloc(scene_id.0, scene_entity_definition.is_global);

        root_node_3d.set_position(scene_entity_definition.get_godot_3d_position());

        let mut root_node_ui_control = DclUiControl::new_alloc();
        root_node_ui_control.set_name(&format!("ui_scene_id_{:?}", scene_id.0));

        let root_node_ui = UiNode {
            base_control: root_node_ui_control.clone(),
            ui_transform: UiTransform::default(),
            computed_parent: SceneEntityId::ROOT,
            has_background: false,
            text_size: None,
        };

        let entities = HashMap::from([(
            SceneEntityId::new(0, 0),
            GodotEntityNode::new(
                Some(root_node_3d.clone().upcast::<Node3D>()),
                Some(root_node_ui),
            ),
        )]);

        GodotDclScene {
            entities,

            root_node_3d,
            hierarchy_dirty_3d: false,
            hierarchy_changed_3d: false,
            unparented_entities_3d: HashSet::new(),

            root_node_ui: root_node_ui_control,
            ui_entities: HashSet::new(),
            hidden_dirty: HashMap::new(),
            ui_visible: false,
            parent_node_ui,
            ui_results: UiResults::new_shared(),
        }
    }

    pub fn get_godot_entity_node(&self, entity: &SceneEntityId) -> Option<&GodotEntityNode> {
        self.entities.get(entity)
    }

    pub fn get_godot_entity_node_mut(
        &mut self,
        entity: &SceneEntityId,
    ) -> Option<&mut GodotEntityNode> {
        self.entities.get_mut(entity)
    }

    pub fn ensure_godot_entity_node(&mut self, entity: &SceneEntityId) -> &mut GodotEntityNode {
        if !self.entities.contains_key(entity) {
            self.entities
                .insert(*entity, GodotEntityNode::new(None, None));
        }

        self.entities.get_mut(entity).unwrap()
    }

    pub fn get_node_or_null_ui(&self, entity: &SceneEntityId) -> Option<&UiNode> {
        self.entities.get(entity)?.base_ui.as_ref()
    }

    pub fn get_node_or_null_ui_mut(&mut self, entity: &SceneEntityId) -> Option<&mut UiNode> {
        self.entities.get_mut(entity)?.base_ui.as_mut()
    }

    pub fn get_node_or_null_3d(&self, entity: &SceneEntityId) -> Option<&Gd<Node3D>> {
        self.entities.get(entity)?.base_3d.as_ref()
    }

    pub fn get_node_or_null_3d_mut(&mut self, entity: &SceneEntityId) -> Option<&mut Gd<Node3D>> {
        self.entities.get_mut(entity)?.base_3d.as_mut()
    }

    pub fn ensure_node_3d(&mut self, entity: &SceneEntityId) -> (&mut GodotEntityNode, Gd<Node3D>) {
        if !self.entities.contains_key(entity) {
            self.entities
                .insert(*entity, GodotEntityNode::new(None, None));
        }

        let godot_entity_node = self.entities.get_mut(entity).unwrap();
        if godot_entity_node.base_3d.is_none() {
            let mut new_node_3d = DclNodeEntity3d::new_alloc(*entity);
            self.root_node_3d
                .add_child(&new_node_3d.clone().upcast::<Node>());

            if entity == &SceneEntityId::PLAYER || entity == &SceneEntityId::CAMERA {
                let mut player_collider_filter = godot::tools::load::<PackedScene>(
                    "res://src/decentraland_components/player_collider_filter.tscn",
                )
                .instantiate()
                .expect("player_collider_filter scene is valid")
                .cast::<Node>();
                player_collider_filter.set_name("PlayerColliderFilter");

                new_node_3d.add_child(&player_collider_filter.clone());
                player_collider_filter.call("init_player_collider_filter", &[]);
            }
            godot_entity_node.base_3d = Some(new_node_3d.upcast());
        }

        let node_3d = godot_entity_node.base_3d.as_ref().unwrap().clone();

        (godot_entity_node, node_3d)
    }

    pub fn ensure_node_ui(&mut self, entity: &SceneEntityId) -> &mut GodotEntityNode {
        if !self.entities.contains_key(entity) {
            self.entities
                .insert(*entity, GodotEntityNode::new(None, None));
        }

        let godot_entity_node = self.entities.get_mut(entity).unwrap();
        if godot_entity_node.base_ui.is_none() {
            let mut new_node_ui = DclUiControl::new_alloc();
            new_node_ui.set_name(&format!("e{:?}_{:?}", entity.number, entity.version));
            new_node_ui
                .bind_mut()
                .set_pointer_events(&godot_entity_node.pointer_events);
            new_node_ui
                .bind_mut()
                .set_ui_result(self.ui_results.clone());
            new_node_ui.bind_mut().set_dcl_entity_id(entity.as_i32());

            self.root_node_ui
                .add_child(&new_node_ui.clone().upcast::<Node>());
            godot_entity_node.base_ui = Some(UiNode {
                base_control: new_node_ui,
                ui_transform: UiTransform::default(),
                computed_parent: SceneEntityId::ROOT,
                has_background: false,
                text_size: None,
            });
            self.ui_entities.insert(*entity);
        }

        godot_entity_node
    }

    pub fn ensure_node_ui_with_control(
        &mut self,
        entity: &SceneEntityId,
    ) -> (&mut GodotEntityNode, Gd<DclUiControl>) {
        let godot_entity_node = self.ensure_node_ui(entity);
        let control = godot_entity_node
            .base_ui
            .as_ref()
            .unwrap()
            .base_control
            .clone();
        (godot_entity_node, control)
    }

    #[allow(dead_code)]
    pub fn exist_node(&self, entity: &SceneEntityId) -> bool {
        self.entities.contains_key(entity)
    }
}
