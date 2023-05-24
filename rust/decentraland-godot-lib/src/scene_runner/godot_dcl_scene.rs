use crate::dcl::{components::SceneEntityId, DclScene, SceneDefinition, SceneResponse};
use godot::{
    engine::{node::InternalMode, Mesh},
    prelude::*,
};
use std::collections::HashMap;

pub struct GodotDclScene {
    pub dcl_scene: DclScene,
    pub definition: SceneDefinition,
    pub entities: HashMap<SceneEntityId, Gd<Node3D>>,
    pub root_node: Gd<Node3D>,
    pub waiting_for_updates: bool,
    pub alive: bool,
    pub objs: Vec<Gd<Node>>,
    pub meshes: Vec<Gd<Mesh>>,
}

impl GodotDclScene {
    pub fn new(
        definition: SceneDefinition,
        thread_sender_to_main: std::sync::mpsc::SyncSender<SceneResponse>,
    ) -> Self {
        let dcl_scene = DclScene::spawn_new(definition.clone(), thread_sender_to_main);

        let mut root_node = Node3D::new_alloc();
        root_node.set_position(definition.offset);
        root_node.set_name(GodotString::from(format!(
            "scene_id_{:?}",
            dcl_scene.scene_id.0
        )));

        let entities = HashMap::from([(SceneEntityId::new(0, 0), root_node.share())]);

        GodotDclScene {
            definition,
            dcl_scene,
            entities,
            root_node,
            waiting_for_updates: false,
            alive: true,
            objs: Vec::new(),
            meshes: Vec::new(),
        }
    }

    pub fn ensure_node(&mut self, entity: &SceneEntityId) -> Gd<Node3D> {
        let maybe_node = self.entities.get(entity);
        if maybe_node.is_some() {
            let value = self.entities.get_mut(entity);
            value.unwrap().share()
        } else {
            let mut new_node = Node3D::new_alloc();

            new_node.set_name(GodotString::from(format!(
                "e{:?}_{:?}",
                entity.number, entity.version
            )));

            self.root_node.add_child(
                new_node.share().upcast(),
                false,
                InternalMode::INTERNAL_MODE_DISABLED,
            );

            self.entities.insert(*entity, new_node.share());
            self.objs.push(new_node.share().upcast());

            new_node.share()
        }
    }
}
