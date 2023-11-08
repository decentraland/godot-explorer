use crate::{
    av::{audio_context::AudioSink, video_stream::VideoSink},
    dcl::{
        components::{
            material::DclMaterial,
            proto_components::{self},
            SceneComponentId, SceneEntityId,
        },
        SceneDefinition, SceneId,
    },
    godot_classes::{dcl_scene_node::DclSceneNode, dcl_ui_control::DclUiControl},
};
use godot::prelude::*;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

use super::components::ui::{scene_ui::UiResults, style::UiTransform};

pub struct GodotDclScene {
    pub entities: HashMap<SceneEntityId, GodotEntityNode>,

    pub root_node_3d: Gd<DclSceneNode>,
    pub hierarchy_dirty_3d: bool,
    pub unparented_entities_3d: HashSet<SceneEntityId>,

    pub parent_node_ui: Gd<DclUiControl>,
    pub root_node_ui: Gd<DclUiControl>,
    pub ui_entities: HashSet<SceneEntityId>,
    pub hidden_dirty: HashMap<SceneComponentId, HashSet<SceneEntityId>>,
    pub ui_visible: bool,

    pub ui_results: Rc<RefCell<UiResults>>,
}

pub struct VideoPlayerData {
    pub video_sink: VideoSink,
    pub audio_sink: AudioSink,
}

pub struct UiNode {
    pub base_control: Gd<DclUiControl>,
    pub ui_transform: UiTransform,
    pub computed_parent: SceneEntityId,
    pub has_background: bool,
    pub has_text: bool,
}

impl UiNode {
    pub fn control_offset(&self) -> i32 {
        (if self.has_background { 1 } else { 0 }) + (if self.has_text { 1 } else { 0 })
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

impl SceneDefinition {
    pub fn from_dict(dict: Dictionary) -> Result<Self, String> {
        let Some(main_crdt_path) = dict.get("main_crdt_path") else {
            return Err("main_crdt_path not found".to_string());
        };
        let Some(path) = dict.get("path") else {
            return Err("path not found".to_string());
        };
        let Some(base) = dict.get("base") else {
            return Err("base not found".to_string());
        };
        let Some(parcels) = dict.get("parcels") else {
            return Err("parcels not found".to_string());
        };
        let Some(visible) = dict.get("visible") else {
            return Err("visible not found".to_string());
        };
        let Some(is_global) = dict.get("is_global") else {
            return Err("is_global not found".to_string());
        };
        let Some(title) = dict.get("title") else {
            return Err("title not found".to_string());
        };
        let Some(entity_id) = dict.get("entity_id") else {
            return Err("entity_id not found".to_string());
        };

        let base =
            Vector2i::try_from_variant(&base).map_err(|_op| "couldn't get offset as Vector2i")?;

        let parcels = VariantArray::try_from_variant(&parcels)
            .map_err(|_op| "couldn't get parcels as array")?;

        let mut parcels = parcels
            .iter_shared()
            .map(|v| Vector2i::try_from_variant(&v));

        if parcels.any(|v| v.is_err()) {
            return Err("couldn't get parcels as Vector2".to_string());
        }

        let mut parcels: Vec<Vector2i> = parcels.map(|v| v.unwrap()).collect();

        if !parcels.contains(&base) {
            parcels.push(base);
        }

        Ok(Self {
            main_crdt_path: main_crdt_path.to::<GodotString>().to_string(),
            path: path.to::<GodotString>().to_string(),
            base,
            visible: visible.to::<bool>(),
            parcels,
            is_global: is_global.to::<bool>(),
            title: title.to::<GodotString>().to_string(),
            entity_id: entity_id.to::<GodotString>().to_string(),
        })
    }
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
        scene_definition: &SceneDefinition,
        scene_id: &SceneId,
        parent_node_ui: Gd<DclUiControl>,
    ) -> Self {
        let mut root_node_3d = DclSceneNode::new_alloc(scene_id.0, scene_definition.is_global);

        root_node_3d.set_position(Vector3 {
            x: 16.0 * scene_definition.base.x as f32,
            y: 0.0,
            z: 16.0 * -scene_definition.base.y as f32,
        });

        let mut root_node_ui_control = DclUiControl::new_alloc();
        root_node_ui_control.set_name(GodotString::from(format!("ui_scene_id_{:?}", scene_id.0)));

        let root_node_ui = UiNode {
            base_control: root_node_ui_control.clone(),
            ui_transform: UiTransform::default(),
            computed_parent: SceneEntityId::ROOT,
            has_background: false,
            has_text: false,
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

    pub fn get_node_ui(&self, entity: &SceneEntityId) -> Option<&UiNode> {
        self.entities.get(entity)?.base_ui.as_ref()
    }

    pub fn get_node_ui_mut(&mut self, entity: &SceneEntityId) -> Option<&mut UiNode> {
        self.entities.get_mut(entity)?.base_ui.as_mut()
    }

    pub fn get_node_3d(&self, entity: &SceneEntityId) -> Option<&Gd<Node3D>> {
        self.entities.get(entity)?.base_3d.as_ref()
    }

    pub fn get_node_3d_mut(&mut self, entity: &SceneEntityId) -> Option<&mut Gd<Node3D>> {
        self.entities.get_mut(entity)?.base_3d.as_mut()
    }

    pub fn ensure_node_3d(&mut self, entity: &SceneEntityId) -> (&mut GodotEntityNode, Gd<Node3D>) {
        if !self.entities.contains_key(entity) {
            self.entities
                .insert(*entity, GodotEntityNode::new(None, None));
        }

        let godot_entity_node = self.entities.get_mut(entity).unwrap();
        if godot_entity_node.base_3d.is_none() {
            let mut new_node_3d = Node3D::new_alloc();
            new_node_3d.set_name(GodotString::from(format!(
                "e{:?}_{:?}",
                entity.number, entity.version
            )));

            self.root_node_3d.add_child(new_node_3d.clone().upcast());
            godot_entity_node.base_3d = Some(new_node_3d);
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
            new_node_ui.set_name(GodotString::from(format!(
                "e{:?}_{:?}",
                entity.number, entity.version
            )));
            new_node_ui
                .bind_mut()
                .set_pointer_events(&godot_entity_node.pointer_events);
            new_node_ui
                .bind_mut()
                .set_ui_result(self.ui_results.clone());
            new_node_ui.bind_mut().set_dcl_entity_id(entity.as_i32());

            self.root_node_ui.add_child(new_node_ui.clone().upcast());
            godot_entity_node.base_ui = Some(UiNode {
                base_control: new_node_ui,
                ui_transform: UiTransform::default(),
                computed_parent: SceneEntityId::ROOT,
                has_background: false,
                has_text: false,
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
        self.entities.get(entity).is_some()
    }
}
