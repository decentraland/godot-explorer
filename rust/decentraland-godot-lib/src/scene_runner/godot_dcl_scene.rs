use crate::dcl::{components::SceneEntityId, crdt::SceneCrdtState, SceneDefinition, SceneId};
use godot::{engine::node::InternalMode, prelude::*};
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
}

impl Node3DEntity {
    fn new() -> Self {
        let base = Node3D::new_alloc();

        Self {
            base,
            desired_parent: SceneEntityId::new(0, 0),
            computed_parent: SceneEntityId::new(0, 0),
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
        root_node.set_position(definition.offset);
        root_node.set_name(GodotString::from(format!("scene_id_{:?}", scene_id.0)));

        let entities = HashMap::from([(
            SceneEntityId::new(0, 0),
            Node3DEntity {
                base: root_node.share(),
                desired_parent: SceneEntityId::new(0, 0),
                computed_parent: SceneEntityId::new(0, 0),
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
