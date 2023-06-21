use crate::{
    dcl::{
        components::{proto_components::sdk::components::PbPointerEventsResult, SceneEntityId},
        js::{SceneLogLevel, SceneLogMessage},
        DclScene, DirtyEntities, DirtyGosComponents, DirtyLwwComponents, RendererResponse,
        SceneDefinition, SceneId, SceneResponse,
    },
    scene_runner::content::ContentMapping,
};
use godot::{
    engine::{node::InternalMode, CharacterBody3D, PhysicsRayQueryParameters3D},
    prelude::*,
};
use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};

use super::{
    components::pointer_events::pointer_events_system, godot_dcl_scene::GodotDclScene,
    input::InputState,
};

pub struct Dirty {
    pub waiting_process: bool,
    pub entities: DirtyEntities,
    pub lww_components: DirtyLwwComponents,
    pub gos_components: DirtyGosComponents,
    pub logs: Vec<SceneLogMessage>,
    pub elapsed_time: f32,
}

pub enum SceneState {
    Alive,
    ToKill,
    KillSignal(i64),
    Dead,
}

pub struct Scene {
    pub scene_id: SceneId,
    pub godot_dcl_scene: GodotDclScene,
    pub dcl_scene: DclScene,
    pub waiting_for_updates: bool,
    pub state: SceneState,

    pub content_mapping: Gd<ContentMapping>,

    pub gltf_loading: HashSet<SceneEntityId>,
    pub pointer_events_result: Vec<(SceneEntityId, PbPointerEventsResult)>,

    pub current_dirty: Dirty,
    pub distance: f32,
    pub last_tick_us: i64,
    pub next_tick_us: i64,
}

#[derive(Debug)]
pub struct GodotDclRaycastResult {
    pub scene_id: SceneId,
    pub entity_id: SceneEntityId,
    pub hit: Dictionary,
}

impl GodotDclRaycastResult {
    pub fn eq_key(a: &Option<GodotDclRaycastResult>, b: &Option<GodotDclRaycastResult>) -> bool {
        if a.is_some() && b.is_some() {
            let a = a.as_ref().unwrap();
            let b = b.as_ref().unwrap();
            a.scene_id == b.scene_id && a.entity_id == b.entity_id
        } else {
            a.is_none() && b.is_none()
        }
    }

    // pub fn get_hit(&self) -> RaycastHit {
    //     RaycastHit {
    //         // pub position: ::core::option::Option<super::super::super::common::Vector3>,
    //         // pub global_origin: ::core::option::Option<super::super::super::common::Vector3>,
    //         // pub direction: ::core::option::Option<super::super::super::common::Vector3>,
    //         // pub normal_hit: ::core::option::Option<super::super::super::common::Vector3>,
    //         // pub length: f32,
    //         // pub mesh_name: ::core::option::Option<::prost::alloc::string::String>,
    //         // pub entity_id: ::core::option::Option<u32>,
    //     }
    // }
}

impl Scene {
    pub fn min_distance(&self, parcel_position: &Vector2i) -> (f32, bool) {
        let diff = self.godot_dcl_scene.definition.base - *parcel_position;
        let mut distance_squared = diff.x * diff.x + diff.y * diff.y;
        for parcel in self.godot_dcl_scene.definition.parcels.iter() {
            let diff = *parcel - *parcel_position;
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

    camera_node: Gd<Camera3D>,
    player_node: Gd<CharacterBody3D>,

    console: Callable,

    player_position: Vector2i,
    current_parcel_scene_id: SceneId,

    thread_sender_to_main: std::sync::mpsc::SyncSender<SceneResponse>,
    main_receiver_from_thread: std::sync::mpsc::Receiver<SceneResponse>,

    elapsed_time: f32,
    pause: bool,
    begin_time: Instant,
    sorted_scene_ids: Vec<SceneId>,
    dying_scene_ids: Vec<SceneId>,

    input_state: InputState,
    last_raycast_result: Option<GodotDclRaycastResult>,
    global_tick_number: u32,
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
        let scene_id = dcl_scene.scene_id;

        let new_scene = Scene {
            scene_id,
            godot_dcl_scene: GodotDclScene::new(
                scene_definition,
                dcl_scene.scene_crdt.clone(),
                scene_id,
            ),
            dcl_scene,
            waiting_for_updates: false,
            state: SceneState::Alive,

            content_mapping,
            current_dirty: Dirty {
                waiting_process: true,
                entities: DirtyEntities::default(),
                lww_components: DirtyLwwComponents::default(),
                gos_components: DirtyGosComponents::default(),
                logs: Vec::new(),
                elapsed_time: 0.0,
            },
            distance: 0.0,
            next_tick_us: 0,
            last_tick_us: 0,
            gltf_loading: HashSet::new(),
            pointer_events_result: Vec::new(),
        };

        self.base.add_child(
            new_scene.godot_dcl_scene.root_node.share().upcast(),
            false,
            InternalMode::INTERNAL_MODE_DISABLED,
        );
        let ret = new_scene.dcl_scene.scene_id.0;
        self.scenes.insert(new_scene.dcl_scene.scene_id, new_scene);

        self.sorted_scene_ids.push(scene_id);
        self.compute_scene_distance();

        ret
    }

    #[func]
    fn kill_scene(&mut self, scene_id: u32) -> bool {
        let scene_id = SceneId(scene_id);
        if let Some(scene) = self.scenes.get_mut(&scene_id) {
            if let SceneState::Alive = scene.state {
                scene.state = SceneState::ToKill;
                self.dying_scene_ids.push(scene_id);
                return true;
            }
        }
        false
    }

    #[func]
    fn set_camera_and_player_node(
        &mut self,
        camera_node: Gd<Camera3D>,
        player_node: Gd<CharacterBody3D>,
        console: Callable,
    ) {
        self.camera_node = camera_node.share();
        self.player_node = player_node.share();
        self.console = console;
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
        player_global_position.x *= 0.0625;
        player_global_position.y *= 0.0625;
        player_global_position.z *= -0.0625;
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

        let start_time_us = (std::time::Instant::now() - self.begin_time).as_micros() as i64;
        let end_time_us = start_time_us + 5000;

        //
        self.sorted_scene_ids.sort_by_key(|&scene_id| {
            let mut scene = self.scenes.get_mut(&scene_id).unwrap();
            if !scene.current_dirty.waiting_process {
                scene.next_tick_us = start_time_us + 120000;
            } else if scene_id == self.current_parcel_scene_id {
                scene.next_tick_us = 0;
            } else {
                scene.next_tick_us = scene.last_tick_us
                    + (20000.0 * scene.distance).max(10000.0).min(100000.0) as i64;
            }
            scene.next_tick_us
        });

        let mut scene_to_remove: HashSet<SceneId> = HashSet::new();

        // TODO: this is debug information, very useful to see the scene priority
        // if self.elapsed_time > 1.0 {
        //     self.elapsed_time = 0.0;
        //     let next_update_vec: Vec<String> = self
        //         .sorted_scene_ids
        //         .iter()
        //         .map(|value| {
        //             let scene = self.scenes.get(value).unwrap();
        //             let last_tick_ms = ((scene.last_tick_us - start_time_us) as f32) / 1000.0;
        //             let next_tick_ms = ((scene.next_tick_us - start_time_us) as f32) / 1000.0;
        //             format!(
        //                 "{} = {:#?}ms => {:#?}ms || d= {:#?}",
        //                 value.0, last_tick_ms, next_tick_ms, scene.distance
        //             )
        //         })
        //         .collect();
        //     godot_print!("next_update: {next_update_vec:#?}");
        // }

        let mut current_time_us = (std::time::Instant::now() - self.begin_time).as_micros() as i64;
        for scene_id in self.sorted_scene_ids.iter() {
            let scene = self.scenes.get_mut(scene_id).unwrap();

            if scene.next_tick_us > current_time_us {
                break;
            }

            if let SceneState::Alive = scene.state {
                let crdt = scene.dcl_scene.scene_crdt.clone();
                let Ok(mut crdt_state) = crdt.try_lock() else {continue;};

                super::update_scene::update_scene(
                    delta,
                    scene,
                    &mut crdt_state,
                    &camera_global_transform,
                    &player_global_transform,
                );

                // enable logs
                for log in &scene.current_dirty.logs {
                    let mut arguments = VariantArray::new();
                    arguments.push((scene_id.0 as i32).to_variant());
                    arguments.push((log.level as i32).to_variant());
                    arguments.push((log.timestamp as f32).to_variant());
                    arguments.push(GodotString::from(&log.message).to_variant());
                    self.console.callv(arguments);
                }

                scene.current_dirty.waiting_process = false;
                let dirty = crdt_state.take_dirty();
                drop(crdt_state);

                if let Err(_e) = scene
                    .dcl_scene
                    .main_sender_to_thread
                    .blocking_send(RendererResponse::Ok(dirty))
                {
                    // TODO: clean up this scene?
                    // godot_print!("failed to send updates to scene: {e:?}");
                }

                current_time_us = (std::time::Instant::now() - self.begin_time).as_micros() as i64;
                scene.last_tick_us = current_time_us;
                if current_time_us > end_time_us {
                    break;
                }
            }
        }

        for scene_id in self.dying_scene_ids.iter() {
            let scene = self.scenes.get_mut(scene_id).unwrap();
            match scene.state {
                SceneState::ToKill => {
                    scene.state = SceneState::KillSignal(current_time_us);
                    if let Err(_e) = scene
                        .dcl_scene
                        .main_sender_to_thread
                        .try_send(RendererResponse::Kill)
                    {
                        // show error
                    } else {
                        scene.state = SceneState::KillSignal(current_time_us);
                    }
                }
                SceneState::KillSignal(kill_time_us) => {
                    if scene.dcl_scene.thread_join_handle.is_finished() {
                        scene.state = SceneState::Dead;
                    } else {
                        let elapsed_from_kill_us = current_time_us - kill_time_us;
                        if elapsed_from_kill_us > 10 * 1e6 as i64 {
                            // 10 seconds from the kill signal
                        }
                    }
                }
                SceneState::Dead => {
                    scene_to_remove.insert(*scene_id);
                }
                _ => {}
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
            scene.godot_dcl_scene.root_node.queue_free();
            self.sorted_scene_ids.retain(|x| x != scene_id);
            self.dying_scene_ids.retain(|x| x != scene_id);
            self.scenes.remove(scene_id);
        }
    }

    fn receive_from_thread(&mut self) {
        // TODO: check infinity loop (loop_end_time)
        loop {
            match self.main_receiver_from_thread.try_recv() {
                Ok(response) => match response {
                    SceneResponse::Error(scene_id, msg) => {
                        let mut arguments = VariantArray::new();
                        arguments.push((scene_id.0 as i32).to_variant());
                        arguments.push((SceneLogLevel::SystemError as i32).to_variant());
                        arguments.push(self.elapsed_time.to_variant());
                        arguments.push(GodotString::from(&msg).to_variant());
                        self.console.callv(arguments);
                    }
                    SceneResponse::Ok(
                        scene_id,
                        (dirty_entities, dirty_lww_components, dirty_gos_components),
                        logs,
                        elapsed_time,
                    ) => {
                        if let Some(scene) = self.scenes.get_mut(&scene_id) {
                            if !scene.current_dirty.waiting_process {
                                scene.current_dirty = Dirty {
                                    waiting_process: true,
                                    entities: dirty_entities,
                                    lww_components: dirty_lww_components,
                                    gos_components: dirty_gos_components,
                                    logs,
                                    elapsed_time,
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

    fn get_current_mouse_entity(&mut self) -> Option<GodotDclRaycastResult> {
        const RAY_LENGTH: f32 = 100.0;

        let mouse_position = self.get_viewport()?.get_mouse_position();
        let raycast_from = self.camera_node.project_ray_origin(mouse_position);
        let raycast_to =
            raycast_from + self.camera_node.project_ray_normal(mouse_position) * RAY_LENGTH;
        let mut space = self.camera_node.get_world_3d()?.get_direct_space_state()?;
        let mut raycast_query = PhysicsRayQueryParameters3D::new();
        raycast_query.set_from(raycast_from);
        raycast_query.set_to(raycast_to);
        raycast_query.set_collision_mask(1); // CL_POINTER

        let raycast_result = space.intersect_ray(raycast_query);
        let collider = raycast_result.get("collider")?;

        let has_dcl_entity_id = collider
            .call(
                StringName::from("has_meta"),
                &[Variant::from("dcl_entity_id")],
            )
            .booleanize();

        if !has_dcl_entity_id {
            return None;
        }

        let dcl_entity_id = collider
            .call(
                StringName::from("get_meta"),
                &[Variant::from("dcl_entity_id")],
            )
            .to::<i32>();
        let dcl_scene_id = collider
            .call(
                StringName::from("get_meta"),
                &[Variant::from("dcl_scene_id")],
            )
            .to::<i32>();

        Some(GodotDclRaycastResult {
            scene_id: SceneId(dcl_scene_id as u32),
            entity_id: SceneEntityId::from_i32(dcl_entity_id),
            hit: raycast_result,
        })
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
            pause: false,
            sorted_scene_ids: vec![],
            dying_scene_ids: vec![],
            current_parcel_scene_id: SceneId(0),

            main_receiver_from_thread,
            thread_sender_to_main,

            camera_node: Camera3D::new_alloc(),
            player_node: CharacterBody3D::new_alloc(),

            player_position: Vector2i::new(-1000, -1000),

            elapsed_time: 0.0,
            begin_time: Instant::now(),
            console: Callable::default(),
            input_state: InputState::default(),
            last_raycast_result: None,
            global_tick_number: 0,
        }
    }

    fn process(&mut self, delta: f64) {
        self.scene_runner_update(delta);

        let changed_inputs = self.input_state.get_new_inputs();
        let current_pointer_raycast_result = self.get_current_mouse_entity();

        pointer_events_system(
            self.global_tick_number,
            &mut self.scenes,
            &changed_inputs,
            &self.last_raycast_result,
            &current_pointer_raycast_result,
        );
        self.last_raycast_result = current_pointer_raycast_result;
        self.global_tick_number += 1;
    }
}
