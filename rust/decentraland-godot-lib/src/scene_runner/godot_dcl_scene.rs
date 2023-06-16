use crate::dcl::{components::SceneEntityId, crdt::SceneCrdtState, SceneDefinition, SceneId};
use godot::{
    engine::{node::InternalMode, StandardMaterial3D},
    prelude::*,
};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

pub struct GodotDclScene {
    pub scene_id: SceneId,
    pub scene_crdt: Arc<Mutex<SceneCrdtState>>,
    pub definition: SceneDefinition,

    // godot
    pub entities: HashMap<SceneEntityId, Node3DEntity>,
    pub root_node: Gd<Node3D>,

    pub hierarchy_dirty: bool,
    pub unparented_entities: HashSet<SceneEntityId>,
}

pub struct Node3DEntity {
    pub base: Gd<Node3D>,
    pub desired_parent: SceneEntityId,
    pub computed_parent: SceneEntityId,
    pub material: Option<Gd<StandardMaterial3D>>,
}

impl SceneDefinition {
    pub fn from_dict(dict: Dictionary) -> Result<Self, String> {
        let Some(main_crdt_path) = dict.get("main_crdt_path") else { return Err("main_crdt_path not found".to_string()) };
        let Some(path) = dict.get("path") else { return Err("path not found".to_string()) };
        let Some(base) = dict.get("base") else { return Err("base not found".to_string()) };
        let Some(parcels) = dict.get("parcels") else { return Err("parcels not found".to_string()) };
        let Some(visible) = dict.get("visible") else { return Err("visible not found".to_string()) };
        let Some(is_global) = dict.get("is_global") else { return Err("is_global not found".to_string()) };

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

        let parcels = parcels.map(|v| v.unwrap()).collect();

        Ok(Self {
            main_crdt_path: main_crdt_path.to::<GodotString>().to_string(),
            path: path.to::<GodotString>().to_string(),
            base,
            visible: visible.to::<bool>(),
            parcels,
            is_global: is_global.to::<bool>(),
        })
    }
}

impl Node3DEntity {
    fn new() -> Self {
        let base = Node3D::new_alloc();

        Self {
            base,
            desired_parent: SceneEntityId::new(0, 0),
            computed_parent: SceneEntityId::new(0, 0),
            material: None,
        }
    }
}

impl GodotDclScene {
    pub fn new(
        definition: SceneDefinition,
        scene_crdt: Arc<Mutex<SceneCrdtState>>,
        scene_id: SceneId,
    ) -> Self {
        let mut root_node = Node3D::new_alloc();
        root_node.set_position(Vector3 {
            x: 16.0 * definition.base.x as f32,
            y: 0.0,
            z: 16.0 * -definition.base.y as f32,
        });
        root_node.set_name(GodotString::from(format!("scene_id_{:?}", scene_id.0)));

        let entities = HashMap::from([(
            SceneEntityId::new(0, 0),
            Node3DEntity {
                base: root_node.share(),
                desired_parent: SceneEntityId::new(0, 0),
                computed_parent: SceneEntityId::new(0, 0),
                material: None,
            },
        )]);

        GodotDclScene {
            scene_id,
            scene_crdt,
            definition,

            entities,
            root_node,

            hierarchy_dirty: false,
            unparented_entities: HashSet::new(),
        }
    }

    pub fn ensure_node_mut(&mut self, entity: &SceneEntityId) -> &mut Node3DEntity {
        let maybe_node = self.entities.get(entity);
        if maybe_node.is_none() {
            let mut new_node = Node3DEntity::new();

            new_node.base.set_name(GodotString::from(format!(
                "e{:?}_{:?}",
                entity.number, entity.version
            )));

            self.root_node.add_child(
                new_node.base.share().upcast(),
                false,
                InternalMode::INTERNAL_MODE_DISABLED,
            );

            self.entities.insert(*entity, new_node);
        }

        self.entities.get_mut(entity).unwrap()
    }

    pub fn get_node(&self, entity: &SceneEntityId) -> Option<&Node3DEntity> {
        self.entities.get(entity)
    }

    #[allow(dead_code)]
    pub fn exist_node(&self, entity: &SceneEntityId) -> bool {
        self.entities.get(entity).is_some()
    }
}
