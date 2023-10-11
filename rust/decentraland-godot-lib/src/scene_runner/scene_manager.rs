use crate::{
    dcl::{
        components::{
            proto_components::sdk::components::common::{
                InputAction, PointerEventType, RaycastHit,
            },
            SceneEntityId,
        },
        js::SceneLogLevel,
        DclScene, RendererResponse, SceneDefinition, SceneId, SceneResponse,
    },
    wallet::Wallet,
};
use godot::{
    engine::{CharacterBody3D, PhysicsRayQueryParameters3D},
    prelude::*,
};
use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};
use tracing::info;

use super::{
    components::pointer_events::{get_entity_pointer_event, pointer_events_system},
    input::InputState,
    scene::{Dirty, GodotDclRaycastResult, Scene, SceneState, SceneUpdateState},
    update_scene::_process_scene,
};

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

    total_time_seconds_time: f32,
    pause: bool,
    begin_time: Instant,
    sorted_scene_ids: Vec<SceneId>,
    dying_scene_ids: Vec<SceneId>,

    input_state: InputState,
    last_raycast_result: Option<GodotDclRaycastResult>,
    global_tick_number: u32,

    pointer_tooltips: VariantArray,
}

#[godot_api]
impl SceneManager {
    // Testing a comment for the API
    #[func]
    fn start_scene(&mut self, scene_definition: Dictionary, content_mapping: Dictionary) -> u32 {
        // TODO: Inject wallet from creator
        let wallet = Wallet::new_local_wallet();

        let scene_definition = match SceneDefinition::from_dict(scene_definition) {
            Ok(scene_definition) => scene_definition,
            Err(e) => {
                tracing::info!("error parsing scene definition: {e:?}");
                return 0;
            }
        };

        let base_url =
            GodotString::from_variant(&content_mapping.get("base_url").unwrap()).to_string();
        let content_dictionary = Dictionary::from_variant(&content_mapping.get("content").unwrap());

        let content_mapping_hash_map: HashMap<String, String> = content_dictionary
            .iter_shared()
            .map(|(file_name, file_hash)| (file_name.to_string(), file_hash.to_string()))
            .collect();

        let new_scene_id = Scene::new_id();
        let dcl_scene = DclScene::spawn_new_js_dcl_scene(
            new_scene_id,
            scene_definition.clone(),
            content_mapping_hash_map,
            base_url,
            self.thread_sender_to_main.clone(),
            wallet,
        );

        let new_scene = Scene::new(new_scene_id, scene_definition, dcl_scene, content_mapping);

        self.base
            .add_child(new_scene.godot_dcl_scene.root_node.clone().upcast());

        self.scenes.insert(new_scene.dcl_scene.scene_id, new_scene);
        self.sorted_scene_ids.push(new_scene_id);
        self.compute_scene_distance();

        new_scene_id.0
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
        self.camera_node = camera_node.clone();
        self.player_node = player_node.clone();
        self.console = console;
    }

    #[func]
    fn get_scene_content_mapping(&self, scene_id: i32) -> Dictionary {
        if let Some(scene) = self.scenes.get(&SceneId(scene_id as u32)) {
            return scene.content_mapping.clone();
        }
        Dictionary::default()
    }

    #[func]
    fn get_scene_title(&self, scene_id: i32) -> GodotString {
        if let Some(scene) = self.scenes.get(&SceneId(scene_id as u32)) {
            return GodotString::from(scene.definition.title.clone());
        }
        GodotString::default()
    }

    #[func]
    fn get_scene_base_parcel(&self, scene_id: i32) -> Vector2i {
        if let Some(scene) = self.scenes.get(&SceneId(scene_id as u32)) {
            return scene.definition.base;
        }
        Vector2i::default()
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

        for (id, scene) in self.scenes.iter_mut() {
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

        let start_time_us = (std::time::Instant::now() - self.begin_time).as_micros() as i64;
        let end_time_us = start_time_us + 1000;

        self.total_time_seconds_time += delta as f32;

        self.receive_from_thread();

        let camera_global_transform = self.camera_node.get_global_transform();
        let player_global_transform = self.player_node.get_global_transform();

        let frames_count = godot::engine::Engine::singleton().get_physics_frames();

        let player_parcel_position = Vector2i::new(
            (player_global_transform.origin.x / 16.0).floor() as i32,
            (-player_global_transform.origin.z / 16.0).floor() as i32,
        );

        if player_parcel_position != self.player_position {
            self.compute_scene_distance();
            self.player_position = player_parcel_position;
        }

        // TODO: review to define a better behavior
        self.sorted_scene_ids.sort_by_key(|&scene_id| {
            let scene = self.scenes.get_mut(&scene_id).unwrap();
            if !scene.current_dirty.waiting_process {
                scene.next_tick_us = start_time_us + 120000;
                // Set at the end of the queue: scenes without processing from scene-runtime, wait until something comes
            } else if scene_id == self.current_parcel_scene_id {
                scene.next_tick_us = 1; // hardcoded priority for current parcel
            } else {
                scene.next_tick_us =
                    scene.last_tick_us + (20000.0 * scene.distance).clamp(10000.0, 100000.0) as i64;

                // TODO: distance in meters or in parcels
            }
            scene.next_tick_us
        });

        let mut scene_to_remove: HashSet<SceneId> = HashSet::new();

        // TODO: this is debug information, very useful to see the scene priority
        // if self.total_time_seconds_time > 1.0 {
        //     self.total_time_seconds_time = 0.0;
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
        //     tracing::info!("next_update: {next_update_vec:#?}");
        // }

        let mut current_time_us = (std::time::Instant::now() - self.begin_time).as_micros() as i64;
        for scene_id in self.sorted_scene_ids.iter() {
            let scene: &mut Scene = self.scenes.get_mut(scene_id).unwrap();

            current_time_us = (std::time::Instant::now() - self.begin_time).as_micros() as i64;
            if scene.next_tick_us > current_time_us {
                break;
            }

            if let SceneState::Alive = scene.state {
                if _process_scene(
                    scene,
                    end_time_us,
                    frames_count,
                    &camera_global_transform,
                    &player_global_transform,
                    &self.begin_time,
                ) {
                    current_time_us =
                        (std::time::Instant::now() - self.begin_time).as_micros() as i64;
                    scene.last_tick_us = current_time_us;
                    if current_time_us > end_time_us {
                        break;
                    }
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
                        tracing::error!("error sending kill signal to thread");
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
                _ => {
                    if scene.dcl_scene.thread_join_handle.is_finished() {
                        tracing::error!("scene closed without kill signal");
                        scene.state = SceneState::Dead;
                    }
                }
            }
        }

        for scene_id in scene_to_remove.iter() {
            let mut scene = self.scenes.remove(scene_id).unwrap();
            let node = scene
                .godot_dcl_scene
                .root_node
                .clone()
                .upcast::<Node>()
                .clone();
            self.base.remove_child(node);
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
                        arguments.push(self.total_time_seconds_time.to_variant());
                        arguments.push(GodotString::from(&msg).to_variant());
                        self.console.callv(arguments);
                    }
                    SceneResponse::Ok(
                        scene_id,
                        (dirty_entities, dirty_lww_components, dirty_gos_components),
                        logs,
                        _,
                    ) => {
                        if let Some(scene) = self.scenes.get_mut(&scene_id) {
                            if !scene.current_dirty.waiting_process {
                                scene.current_dirty = Dirty {
                                    waiting_process: true,
                                    entities: dirty_entities,
                                    lww_components: dirty_lww_components,
                                    gos_components: dirty_gos_components,
                                    logs,
                                    renderer_response: None,
                                    update_state: SceneUpdateState::None,
                                };
                            } else {
                                scene.enqueued_dirty.push(Dirty {
                                    waiting_process: true,
                                    entities: dirty_entities,
                                    lww_components: dirty_lww_components,
                                    gos_components: dirty_gos_components,
                                    logs,
                                    renderer_response: None,
                                    update_state: SceneUpdateState::None,
                                });
                            }
                        }
                    }
                    SceneResponse::RemoveGodotScene(scene_id, logs) => {
                        if let Some(scene) = self.scenes.get_mut(&scene_id) {
                            scene.state = SceneState::Dead;
                            if !self.dying_scene_ids.contains(&scene_id) {
                                self.dying_scene_ids.push(scene_id);
                            }
                        }
                        // enable logs
                        for log in &logs {
                            let mut arguments = VariantArray::new();
                            arguments.push((scene_id.0 as i32).to_variant());
                            arguments.push((log.level as i32).to_variant());
                            arguments.push((log.timestamp as f32).to_variant());
                            arguments.push(GodotString::from(&log.message).to_variant());
                            self.console.callv(arguments);
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

        let viewport_size = self.base.get_viewport()?.get_visible_rect();
        let mouse_position = Vector2::new(viewport_size.size.x * 0.5, viewport_size.size.y * 0.5);
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

        let scene = self.scenes.get(&SceneId(dcl_scene_id as u32))?;
        let scene_position = scene.godot_dcl_scene.root_node.get_position();
        let raycast_data = RaycastHit::from_godot_raycast(
            scene_position,
            raycast_from,
            &raycast_result,
            Some(dcl_entity_id as u32),
        )?;

        Some(GodotDclRaycastResult {
            scene_id: SceneId(dcl_scene_id as u32),
            entity_id: SceneEntityId::from_i32(dcl_entity_id),
            hit: raycast_data,
        })
    }

    #[func]
    fn get_tooltips(&self) -> VariantArray {
        self.pointer_tooltips.clone()
    }

    #[signal]
    fn pointer_tooltip_changed() {}
}

#[godot_api]
impl NodeVirtual for SceneManager {
    fn init(base: Base<Node>) -> Self {
        let (thread_sender_to_main, main_receiver_from_thread) =
            std::sync::mpsc::sync_channel(1000);

        #[cfg(target_os = "android")]
        android::init_logger();

        #[cfg(not(target_os = "android"))]
        tracing_subscriber::fmt::init();

        info!("SceneManager started");

        log_panics::init();

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

            total_time_seconds_time: 0.0,
            begin_time: Instant::now(),
            console: Callable::invalid(),
            input_state: InputState::default(),
            last_raycast_result: None,
            global_tick_number: 0,
            pointer_tooltips: VariantArray::new(),
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

        let should_update_tooltip = !changed_inputs.is_empty()
            || !GodotDclRaycastResult::eq_key(
                &self.last_raycast_result,
                &current_pointer_raycast_result,
            );

        if should_update_tooltip {
            let mut tooltips = VariantArray::new();
            if let Some(raycast) = current_pointer_raycast_result.as_ref() {
                if let Some(pointer_events) =
                    get_entity_pointer_event(&self.scenes, &raycast.scene_id, &raycast.entity_id)
                {
                    for pointer_event in pointer_events.pointer_events.iter() {
                        if let Some(info) = pointer_event.event_info.as_ref() {
                            // TODO: filter by show_beedback and max_distance
                            // let (show_feedback, max_distance) = (
                            //     info.show_feedback.as_ref().unwrap_or(&true).clone(),
                            //     info.max_distance.as_ref().unwrap_or(&10.0).clone(),
                            // );
                            // if !show_feedback || raycast.hit.length > max_distance {
                            //     continue;
                            // }

                            let input_action =
                                InputAction::from_i32(*info.button.as_ref().unwrap_or(&0))
                                    .unwrap_or(InputAction::IaAny);

                            let state =
                                *self.input_state.state.get(&input_action).unwrap_or(&false);
                            let match_state = (pointer_event.event_type
                                == PointerEventType::PetUp as i32
                                && state)
                                || (pointer_event.event_type == PointerEventType::PetDown as i32
                                    && !state);
                            if match_state {
                                let text = if let Some(text) = info.hover_text.as_ref() {
                                    GodotString::from(text)
                                } else {
                                    GodotString::from("Interact")
                                };

                                let mut dict = Dictionary::new();
                                dict.set(StringName::from("text"), text);
                                dict.set(
                                    StringName::from("action"),
                                    GodotString::from(input_action.as_str_name()),
                                );
                                dict.set(
                                    StringName::from("event_type"),
                                    Variant::from(pointer_event.event_type),
                                );
                                tooltips.push(dict.to_variant());
                            }
                        }
                    }
                }
            }

            self.pointer_tooltips = tooltips;
            self.base.emit_signal("pointer_tooltip_changed".into(), &[]);
        }

        // TODO: should only update the current scnes + globals
        for (_, scene) in self.scenes.iter_mut() {
            if let Some(player_node_entity) =
                scene.godot_dcl_scene.get_node_mut(&SceneEntityId::PLAYER)
            {
                player_node_entity
                    .base
                    .set_global_transform(self.player_node.get_global_transform());
            }

            if let Some(camera_node_entity) =
                scene.godot_dcl_scene.get_node_mut(&SceneEntityId::CAMERA)
            {
                camera_node_entity
                    .base
                    .set_global_transform(self.player_node.get_global_transform());
            }
        }

        self.last_raycast_result = current_pointer_raycast_result;
        self.global_tick_number += 1;
    }
}

#[cfg(target_os = "android")]
mod android {
    use tracing_subscriber::filter::LevelFilter;
    use tracing_subscriber::fmt::format::FmtSpan;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{self, registry};

    pub fn init_logger() {
        let android_layer = paranoid_android::layer(env!("CARGO_PKG_NAME"))
            .with_span_events(FmtSpan::CLOSE)
            .with_thread_names(true)
            .with_filter(LevelFilter::DEBUG);

        registry().with(android_layer).init();
    }
}
