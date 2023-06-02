use crate::{
    dcl::{DclScene, RendererResponse, SceneDefinition, SceneId, SceneResponse},
    scene_runner::content::ContentMapping,
};
use godot::{engine::node::InternalMode, prelude::*};
use std::collections::{HashMap, HashSet};

use super::godot_dcl_scene::GodotDclScene;

pub struct Scene {
    pub godot_dcl_scene: GodotDclScene,
    pub dcl_scene: DclScene,
    pub waiting_for_updates: bool,
    pub alive: bool,

    pub content_mapping: Gd<ContentMapping>,
}

// Deriving GodotClass makes the class available to Godot
#[derive(GodotClass)]
#[class(base=Node)]
pub struct SceneManager {
    #[base]
    base: Base<Node>,
    scenes: HashMap<SceneId, Scene>,

    camera_node: Option<Gd<Node3D>>,

    thread_sender_to_main: std::sync::mpsc::SyncSender<SceneResponse>,
    main_receiver_from_thread: std::sync::mpsc::Receiver<SceneResponse>,

    renderering_tick: i64,
}

#[godot_api]
impl SceneManager {
    #[func]
    fn start_scene(
        &mut self,
        path: GodotString,
        offset: godot::prelude::Vector3,
        content_mapping: Gd<ContentMapping>,
    ) -> u32 {
        let scene_definition = SceneDefinition {
            path: path.to_string(),
            offset,
            visible: true,
        };
        let dcl_scene =
            DclScene::spawn_new(scene_definition.clone(), self.thread_sender_to_main.clone());

        let new_scene = Scene {
            godot_dcl_scene: GodotDclScene::new(
                scene_definition,
                dcl_scene.scene_crdt.clone(),
                dcl_scene.scene_id,
            ),
            dcl_scene,
            waiting_for_updates: false,
            alive: true,

            content_mapping,
        };

        self.base.add_child(
            new_scene.godot_dcl_scene.root_node.share().upcast(),
            false,
            InternalMode::INTERNAL_MODE_DISABLED,
        );
        let ret = new_scene.dcl_scene.scene_id.0;
        self.scenes.insert(new_scene.dcl_scene.scene_id, new_scene);

        ret
    }

    #[func]
    fn kill_scene(&mut self, scene_id: u32) -> bool {
        let scene_id = SceneId(scene_id);
        if let Some(scene) = self.scenes.get_mut(&scene_id) {
            if scene.alive {
                scene.alive = false;
                return true;
            }
        }
        false
    }

    #[func]
    fn set_camera_node(&mut self, camera_node: Gd<Node3D>) {
        self.camera_node = Some(camera_node.share());
    }

    #[func]
    fn get_scene_content_mapping(&self, scene_id: i32) -> Gd<ContentMapping> {
        if let Some(scene) = self.scenes.get(&SceneId(scene_id as u32)) {
            return scene.content_mapping.share();
        }
        Gd::new_default()
    }

    fn scene_runner_update(&mut self, delta: f64) {
        self.renderering_tick += 1;

        self.process_scenes(delta);

        let mut scene_to_remove: HashSet<SceneId> = HashSet::new();

        for (id, scene) in self.scenes.iter_mut() {
            if scene.waiting_for_updates && !scene.alive {
                if scene.dcl_scene.thread_join_handle.is_finished() {
                    scene_to_remove.insert(*id);
                }
            } else if scene.alive {
                let crdt = scene.dcl_scene.scene_crdt.clone();
                let mut crdt_state = crdt.lock().unwrap();
                let dirty = crdt_state.take_dirty();
                drop(crdt_state);

                if let Err(_e) = scene
                    .dcl_scene
                    .main_sender_to_thread
                    .blocking_send(RendererResponse::Ok(dirty))
                {
                    // TODO: clean up this scene?
                    // godot_print!("failed to send updates to scene: {e:?}");
                } else {
                    scene.waiting_for_updates = true;
                }
            } else {
                if let Err(_e) = scene
                    .dcl_scene
                    .main_sender_to_thread
                    .blocking_send(RendererResponse::Kill)
                {
                    // TODO: clean up this scene?
                    // godot_print!("failed to send updates to scene: {e:?} after killing it");
                } else {
                    scene.waiting_for_updates = true;
                }
                continue;
            }
        }

        for scene_id in scene_to_remove.iter() {
            let mut scene = self.scenes.remove(scene_id).unwrap();
            let node = scene
                .godot_dcl_scene
                .root_node
                .share()
                .upcast::<Node>()
                .share();
            self.remove_child(node);

            while let Some(mut obj) = scene.godot_dcl_scene.objs.pop() {
                obj.queue_free();
            }

            scene.godot_dcl_scene.root_node.queue_free();
        }
    }

    fn process_scenes(&mut self, delta: f64) {
        let camera_global_transform = if let Some(camera_node) = self.camera_node.as_ref() {
            camera_node.get_global_transform()
        } else {
            Transform3D::IDENTITY
        };

        // TODO: check infinity loop (loop_end_time)
        loop {
            match self.main_receiver_from_thread.try_recv() {
                Ok(response) => match response {
                    SceneResponse::Error(scene_id, msg) => {
                        godot_print!("[{scene_id:?}] error: {msg}");
                    }
                    SceneResponse::Ok(scene_id, (dirty_entities, dirty_components)) => {
                        if let Some(scene) = self.scenes.get_mut(&scene_id) {
                            let crdt = scene.dcl_scene.scene_crdt.clone();
                            let mut crdt_state = crdt.lock().unwrap();

                            super::update_scene::update_scene(
                                delta,
                                scene,
                                &mut crdt_state,
                                &dirty_entities,
                                &dirty_components,
                                &camera_global_transform,
                            );
                        }
                    }
                },
                Err(std::sync::mpsc::TryRecvError::Empty) => return,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    panic!("render thread receiver exploded");
                }
            }
        }
    }
}

#[godot_api]
impl NodeVirtual for SceneManager {
    fn init(base: Base<Node>) -> Self {
        let (thread_sender_to_main, main_receiver_from_thread) =
            std::sync::mpsc::sync_channel(1000);

        SceneManager {
            base,
            scenes: HashMap::new(),
            main_receiver_from_thread,
            thread_sender_to_main,
            camera_node: None,
            renderering_tick: 0,
        }
    }

    fn ready(&mut self) {
        // Note: this is downcast during load() -- completely type-safe thanks to type inference!
        // If the resource does not exist or has an incompatible type, this panics.
        // There is also try_load() if you want to check whether loading succeeded.
    }

    fn process(&mut self, delta: f64) {
        self.scene_runner_update(delta);
    }
}
