use self::godot_dcl_scene::GodotDclScene;
use crate::dcl::{RendererResponse, SceneDefinition, SceneId, SceneResponse};
use godot::{engine::node::InternalMode, prelude::*};
use std::collections::{HashMap, HashSet};

mod godot_dcl_scene;
mod update_scene;

// Deriving GodotClass makes the class available to Godot
#[derive(GodotClass)]
#[class(base=Node)]
pub struct SceneRunner {
    #[base]
    base: Base<Node>,
    scenes: HashMap<SceneId, GodotDclScene>,

    thread_sender_to_main: std::sync::mpsc::SyncSender<SceneResponse>,
    main_receiver_from_thread: std::sync::mpsc::Receiver<SceneResponse>,
}

#[godot_api]
impl SceneRunner {
    #[func]
    fn start_scene(&mut self, path: GodotString, offset: godot::prelude::Vector3) -> u32 {
        let scene_definition = SceneDefinition {
            path: path.to_string(),
            offset,
            visible: true,
        };
        let new_scene = GodotDclScene::new(scene_definition, self.thread_sender_to_main.clone());

        self.base.add_child(
            new_scene.root_node.share().upcast(),
            false,
            InternalMode::INTERNAL_MODE_DISABLED,
        );

        godot_print!(
            "starting scene {} with id {:?}",
            path,
            new_scene.dcl_scene.scene_id
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

    fn scene_runner_update(&mut self, delta: f64) {
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

                if let Err(e) = scene
                    .dcl_scene
                    .main_sender_to_thread
                    .blocking_send(RendererResponse::Ok(dirty))
                {
                    println!("failed to send updates to scene: {e:?}");
                } else {
                    scene.waiting_for_updates = true;
                }
            } else {
                if let Err(e) = scene
                    .dcl_scene
                    .main_sender_to_thread
                    .blocking_send(RendererResponse::Kill)
                {
                    println!("failed to send updates to scene: {e:?}");
                    // TODO: clean up
                } else {
                    scene.waiting_for_updates = true;
                }
                continue;
            }
        }

        for scene_id in scene_to_remove.iter() {
            let mut scene = self.scenes.remove(scene_id).unwrap();
            let node = scene.root_node.share().upcast::<Node>().share();
            self.remove_child(node);

            while let Some(mut obj) = scene.objs.pop() {
                obj.queue_free();
            }

            scene.root_node.queue_free();
        }
    }

    fn process_scenes(&mut self, delta: f64) {
        // TODO: check infinity loop (loop_end_time)
        loop {
            match self.main_receiver_from_thread.try_recv() {
                Ok(response) => match response {
                    SceneResponse::Error(scene_id, msg) => {
                        println!("[{scene_id:?}] error: {msg}");
                    }
                    SceneResponse::Ok(scene_id, (dirty_entities, dirty_components)) => {
                        if let Some(scene) = self.scenes.get_mut(&scene_id) {
                            update_scene::update_scene(
                                delta,
                                scene,
                                dirty_entities,
                                dirty_components,
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
impl NodeVirtual for SceneRunner {
    fn init(base: Base<Node>) -> Self {
        let (thread_sender_to_main, main_receiver_from_thread) =
            std::sync::mpsc::sync_channel(1000);

        SceneRunner {
            base,
            scenes: HashMap::new(),
            main_receiver_from_thread,
            thread_sender_to_main,
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
