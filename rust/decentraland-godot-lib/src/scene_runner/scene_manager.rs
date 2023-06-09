use crate::{
    dcl::{
        DclScene, DirtyComponents, DirtyEntities, RendererResponse, SceneDefinition, SceneId,
        SceneResponse,
    },
    scene_runner::content::ContentMapping,
};
use godot::{engine::node::InternalMode, prelude::*};
use num::integer::Roots;
use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap},
    time::{Duration, Instant},
};

use super::godot_dcl_scene::GodotDclScene;

pub struct Dirty {
    pub waiting_process: bool,
    pub entities: DirtyEntities,
    pub components: DirtyComponents,
}

pub struct Scene {
    pub scene_id: SceneId,
    pub godot_dcl_scene: GodotDclScene,
    pub dcl_scene: DclScene,
    pub waiting_for_updates: bool,
    pub alive: bool,

    pub content_mapping: Gd<ContentMapping>,

    pub current_dirty: Dirty,
    pub distance: f32,
    pub priority: i8,
    pub last_tick: Instant,
    pub next_update: Instant,
}

impl Eq for Scene {}
impl PartialEq for Scene {
    fn eq(&self, other: &Self) -> bool {
        self.scene_id == other.scene_id
    }
}

impl PartialOrd for Scene {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Scene {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.priority == other.priority {
            self.distance
                .partial_cmp(&other.distance)
                .unwrap()
                .reverse()
        } else {
            self.priority.cmp(&other.priority)
        }
    }
}

impl Scene {
    pub fn min_distance(&self, parcel_position: &Vector2i) -> (f32, bool) {
        let mut inside_scene = false;
        let diff = self.godot_dcl_scene.definition.base - *parcel_position;
        let mut distance_squared = diff.x * diff.x + diff.y * diff.y;
        for parcel in self.godot_dcl_scene.definition.parcels.iter() {
            let diff = self.godot_dcl_scene.definition.base - *parcel_position;
            distance_squared = distance_squared.min(diff.x * diff.x + diff.y * diff.y);
        }
        ((distance_squared as f32).sqrt(), distance_squared == 0)
    }
}

// Deriving GodotClass makes the class available to Godot
#[derive(GodotClass)]
#[class(base=Node)]
pub struct SceneManager {
    #[base]
    base: Base<Node>,
    scenes: HashMap<SceneId, Scene>,

    camera_node: Gd<Node3D>,
    player_node: Gd<Node3D>,

    player_position: Vector2i,
    current_parcel_scene_id: SceneId,

    thread_sender_to_main: std::sync::mpsc::SyncSender<SceneResponse>,
    main_receiver_from_thread: std::sync::mpsc::Receiver<SceneResponse>,

    global_renderering_tick: i64,
    elapsed_time: f32,
    pause: bool,
    begin_time: Instant,
}

#[godot_api]
impl SceneManager {
    // Testing a comment for the API
    #[func]
    fn start_scene(
        &mut self,
        scene_definition: Dictionary,
        content_mapping: Gd<ContentMapping>,
    ) -> u32 {
        let scene_definition = match SceneDefinition::from_dict(scene_definition) {
            Ok(scene_definition) => scene_definition,
            Err(e) => {
                godot_print!("error parsing scene definition: {e:?}");
                return 0;
            }
        };

        let dcl_scene =
            DclScene::spawn_new(scene_definition.clone(), self.thread_sender_to_main.clone());

        let new_scene = Scene {
            scene_id: dcl_scene.scene_id,
            godot_dcl_scene: GodotDclScene::new(
                scene_definition,
                dcl_scene.scene_crdt.clone(),
                dcl_scene.scene_id,
            ),
            dcl_scene,
            waiting_for_updates: false,
            alive: true,

            content_mapping,
            current_dirty: Dirty {
                waiting_process: true,
                entities: DirtyEntities::default(),
                components: DirtyComponents::default(),
            },
            distance: 0.0,
            priority: 0,
            next_update: Instant::now(),
            last_tick: Instant::now(),
        };

        self.base.add_child(
            new_scene.godot_dcl_scene.root_node.share().upcast(),
            false,
            InternalMode::INTERNAL_MODE_DISABLED,
        );
        let ret = new_scene.dcl_scene.scene_id.0;
        self.scenes.insert(new_scene.dcl_scene.scene_id, new_scene);

        self.compute_scene_distance();

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
    fn set_camera_and_player_node(&mut self, camera_node: Gd<Node3D>, player_node: Gd<Node3D>) {
        self.camera_node = camera_node.share();
        self.player_node = player_node.share();
    }

    #[func]
    fn get_scene_content_mapping(&self, scene_id: i32) -> Gd<ContentMapping> {
        if let Some(scene) = self.scenes.get(&SceneId(scene_id as u32)) {
            return scene.content_mapping.share();
        }
        Gd::new_default()
    }

    fn compute_scene_distance(&mut self) {
        let mut player_global_position = self.player_node.get_global_transform().origin;
        player_global_position = player_global_position / 16.0;
        player_global_position.z = -player_global_position.z;
        let player_parcel_position = Vector2i::new(
            player_global_position.x.floor() as i32,
            player_global_position.z.floor() as i32,
        );

        for (id, mut scene) in self.scenes.iter_mut() {
            let (distance, inside_scene) = scene.min_distance(&player_parcel_position);
            scene.distance = distance;
            if inside_scene {
                self.current_parcel_scene_id = *id;
            }
        }
    }

    fn scene_runner_update(&mut self, delta: f64) {
        if self.pause {
            return;
        }
        self.global_renderering_tick += 1;
        self.elapsed_time += delta as f32;

        self.receive_from_thread();

        let camera_global_transform = self.camera_node.get_global_transform();
        let player_global_transform = self.player_node.get_global_transform();

        let player_parcel_position = Vector2i::new(
            (player_global_transform.origin.x / 16.0).floor() as i32,
            (-player_global_transform.origin.z / 16.0).floor() as i32,
        );

        if player_parcel_position != self.player_position {
            self.compute_scene_distance();
            self.player_position = player_parcel_position;
        }

        let start_time = std::time::Instant::now();
        let end_time = start_time + std::time::Duration::from_millis(1);

        let mut scene_ids: Vec<SceneId> = self.scenes.keys().cloned().collect();
        scene_ids.sort_by_key(|&scene_id| {
            self.scenes
                .get(&scene_id)
                .unwrap()
                .next_update
                .duration_since(self.begin_time)
        });

        // let mut scene_to_remove: HashSet<SceneId> = HashSet::new();

        if self.elapsed_time > 1.0 {
            self.elapsed_time = 0.0;
            let now = Instant::now();
            let next_update_vec: Vec<String> = scene_ids
                .iter()
                .map(|value| {
                    format!(
                        "{} = {:#?} => {:#?} || d= {:#?}",
                        value.0,
                        self.scenes.get(value).unwrap().last_tick.duration_since(self.begin_time),
                        self.scenes.get(value).unwrap().next_update.duration_since(self.begin_time),
                        self.scenes.get(value).unwrap().distance
                    )
                })
                .collect();
            godot_print!("next_update: {next_update_vec:#?}");
        }

        for scene_id in scene_ids.iter() {
            let scene = self.scenes.get_mut(&scene_id).unwrap();
            if !scene.alive {
                continue;
            }

            if scene.current_dirty.waiting_process {
                let crdt = scene.dcl_scene.scene_crdt.clone();
                let Ok(mut crdt_state) = crdt.try_lock() else {continue;};

                super::update_scene::update_scene(
                    delta,
                    scene,
                    &mut crdt_state,
                    &camera_global_transform,
                );

                scene.current_dirty.waiting_process = false;
                scene.last_tick = Instant::now();

                scene.next_update = scene.last_tick
                    + Duration::from_millis((20.0 * scene.distance).min(1000.0).max(10.0) as u64);

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
                    // scene.waiting_for_updates = true;
                }
            }

            if Instant::now() > end_time {
                break;
            }

            // if scene.waiting_for_updates && !scene.alive {
            //     if scene.dcl_scene.thread_join_handle.is_finished() {
            //         scene_to_remove.insert(*id);
            //     }
            // } else if scene.alive {
            //     let crdt = scene.dcl_scene.scene_crdt.clone();
            //     let crdt_state = crdt.try_lock();
            //     if crdt_state.is_err() {
            //         continue;
            //     }

            //     let mut crdt_state = crdt_state.unwrap();
            //     let dirty = crdt_state.take_dirty();
            //     drop(crdt_state);

            //     if let Err(_e) = scene
            //         .dcl_scene
            //         .main_sender_to_thread
            //         .blocking_send(RendererResponse::Ok(dirty))
            //     {
            //         // TODO: clean up this scene?
            //         // godot_print!("failed to send updates to scene: {e:?}");
            //     } else {
            //         scene.waiting_for_updates = true;
            //     }
            // } else {
            //     if let Err(_e) = scene
            //         .dcl_scene
            //         .main_sender_to_thread
            //         .blocking_send(RendererResponse::Kill)
            //     {
            //         // TODO: clean up this scene?
            //         // godot_print!("failed to send updates to scene: {e:?} after killing it");
            //     } else {
            //         scene.waiting_for_updates = true;
            //     }
            //     continue;
            // }
        }

        // for scene_id in scene_to_remove.iter() {
        //     let mut scene = self.scenes.remove(scene_id).unwrap();
        //     let node = scene
        //         .godot_dcl_scene
        //         .root_node
        //         .share()
        //         .upcast::<Node>()
        //         .share();
        //     self.remove_child(node);
        //     scene.godot_dcl_scene.root_node.queue_free();
        // }
    }

    fn receive_from_thread(&mut self) {
        // TODO: check infinity loop (loop_end_time)
        loop {
            match self.main_receiver_from_thread.try_recv() {
                Ok(response) => match response {
                    SceneResponse::Error(scene_id, msg) => {
                        godot_print!("[{scene_id:?}] error: {msg}");
                    }
                    SceneResponse::Ok(scene_id, (dirty_entities, dirty_components)) => {
                        if let Some(scene) = self.scenes.get_mut(&scene_id) {
                            if !scene.current_dirty.waiting_process {
                                scene.current_dirty = Dirty {
                                    waiting_process: true,
                                    entities: dirty_entities,
                                    components: dirty_components,
                                };
                            } else {
                                godot_print!("scene {scene_id:?} is already dirty, skipping");
                            }
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

    #[func]
    fn set_pause(&mut self, value: bool) {
        self.pause = value;
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
            camera_node: Node3D::new_alloc(),
            player_node: Node3D::new_alloc(),
            global_renderering_tick: 0,
            pause: false,
            player_position: Vector2i::new(-1000, -1000),
            current_parcel_scene_id: SceneId(0),
            elapsed_time: 0.0,
            begin_time: Instant::now(),
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
