use crate::{
    auth::wallet::Wallet,
    dcl::{
        components::{
            proto_components::{
                common::BorderRect,
                sdk::components::{
                    common::{InputAction, PointerEventType, RaycastHit},
                    PbUiCanvasInformation,
                },
            },
            SceneEntityId,
        },
        js::SceneLogLevel,
        DclScene, RendererResponse, SceneDefinition, SceneId, SceneResponse,
    },
    godot_classes::{
        dcl_camera_3d::DclCamera3D, dcl_global::DclGlobal, dcl_ui_control::DclUiControl,
        rpc_sender::take_and_compare_snapshot_response::DclRpcSenderTakeAndCompareSnapshotResponse,
        JsonGodotClass,
    },
};
use godot::{engine::PhysicsRayQueryParameters3D, prelude::*};
use std::{
    collections::{HashMap, HashSet},
    sync::atomic::AtomicU32,
    time::Instant,
};

use super::{
    components::pointer_events::{get_entity_pointer_event, pointer_events_system},
    input::InputState,
    scene::{
        Dirty, GlobalSceneType, GodotDclRaycastResult, Scene, SceneState, SceneType,
        SceneUpdateState,
    },
    update_scene::_process_scene,
};

// Deriving GodotClass makes the class available to Godot
#[derive(GodotClass)]
#[class(base=Node)]
pub struct SceneManager {
    #[base]
    base: Base<Node>,

    #[var]
    base_ui: Gd<DclUiControl>,
    ui_canvas_information: PbUiCanvasInformation,

    scenes: HashMap<SceneId, Scene>,

    #[export]
    camera_node: Gd<DclCamera3D>,

    #[export]
    player_node: Gd<Node3D>,

    #[var]
    console: Callable,

    player_position: Vector2i,
    current_parcel_scene_id: SceneId,
    last_current_parcel_scene_id: SceneId,

    thread_sender_to_main: std::sync::mpsc::SyncSender<SceneResponse>,
    main_receiver_from_thread: std::sync::mpsc::Receiver<SceneResponse>,

    total_time_seconds_time: f32,
    pause: bool,
    begin_time: Instant,
    sorted_scene_ids: Vec<SceneId>,
    dying_scene_ids: Vec<SceneId>,

    input_state: InputState,
    last_raycast_result: Option<GodotDclRaycastResult>,

    #[export]
    pointer_tooltips: VariantArray,
}

// This value is the current global tick number, is used for marking the cronolgy of lamport timestamp
pub static GLOBAL_TICK_NUMBER: AtomicU32 = AtomicU32::new(0);

#[godot_api]
impl SceneManager {
    #[signal]
    fn scene_spawned(&self, scene_id: i32, entity_id: GString) {}

    #[signal]
    fn scene_killed(&self, scene_id: i32, entity_id: GString) {}

    // Testing a comment for the API
    #[func]
    fn start_scene(&mut self, scene_definition: Dictionary, content_mapping: Dictionary) -> i32 {
        // TODO: Inject wallet from creator
        let wallet = Wallet::new_local_wallet();

        let scene_definition = match SceneDefinition::from_dict(scene_definition) {
            Ok(scene_definition) => scene_definition,
            Err(e) => {
                tracing::info!("error parsing scene definition: {e:?}");
                return 0;
            }
        };

        let base_url = GString::from_variant(&content_mapping.get("base_url").unwrap()).to_string();
        let content_dictionary = Dictionary::from_variant(&content_mapping.get("content").unwrap());
        let scene_type = if scene_definition.is_global {
            SceneType::Global(GlobalSceneType::GlobalRealm)
        } else {
            SceneType::Parcel
        };

        let content_mapping_hash_map: HashMap<String, String> = content_dictionary
            .iter_shared()
            .map(|(file_name, file_hash)| (file_name.to_string(), file_hash.to_string()))
            .collect();

        let new_scene_id = Scene::new_id();
        let signal_data = (new_scene_id, scene_definition.entity_id.clone());
        let dcl_scene = DclScene::spawn_new_js_dcl_scene(
            new_scene_id,
            scene_definition.clone(),
            content_mapping_hash_map,
            base_url,
            self.thread_sender_to_main.clone(),
            wallet,
        );

        let new_scene = Scene::new(
            new_scene_id,
            scene_definition,
            dcl_scene,
            content_mapping,
            scene_type.clone(),
            self.base_ui.clone(),
        );

        self.base
            .add_child(new_scene.godot_dcl_scene.root_node_3d.clone().upcast());

        if let SceneType::Global(_) = scene_type {
            self.base_ui
                .add_child(new_scene.godot_dcl_scene.root_node_ui.clone().upcast());
        }

        self.scenes.insert(new_scene.dcl_scene.scene_id, new_scene);
        self.sorted_scene_ids.push(new_scene_id);
        self.compute_scene_distance();

        self.base.emit_signal(
            "scene_spawned".into(),
            &[signal_data.0 .0.to_variant(), signal_data.1.to_variant()],
        );
        new_scene_id.0
    }

    #[func]
    fn kill_scene(&mut self, scene_id: i32) -> bool {
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
        camera_node: Gd<DclCamera3D>,
        player_node: Gd<Node3D>,
        console: Callable,
    ) {
        self.camera_node = camera_node.clone();
        self.player_node = player_node.clone();
        self.console = console;
    }

    #[func]
    fn get_scene_content_mapping(&self, scene_id: i32) -> Dictionary {
        if let Some(scene) = self.scenes.get(&SceneId(scene_id)) {
            return scene.content_mapping.clone();
        }
        Dictionary::default()
    }

    #[func]
    fn get_scene_title(&self, scene_id: i32) -> GString {
        if let Some(scene) = self.scenes.get(&SceneId(scene_id)) {
            return GString::from(scene.definition.title.clone());
        }
        GString::default()
    }

    #[func]
    pub fn get_scene_id_by_parcel_position(&self, parcel_position: Vector2i) -> i32 {
        for scene in self.scenes.values() {
            if let SceneType::Global(_) = scene.scene_type {
                continue;
            }

            if scene.definition.parcels.contains(&parcel_position) {
                return scene.scene_id.0;
            }
        }

        SceneId::INVALID.0
    }

    #[func]
    fn get_scene_base_parcel(&self, scene_id: i32) -> Vector2i {
        if let Some(scene) = self.scenes.get(&SceneId(scene_id)) {
            return scene.definition.base;
        }
        Vector2i::default()
    }

    fn compute_scene_distance(&mut self) {
        self.current_parcel_scene_id = SceneId::INVALID;

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
                if let SceneType::Parcel = scene.scene_type {
                    self.current_parcel_scene_id = *id;
                }
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
        let camera_mode = self.camera_node.bind().get_camera_mode();

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

        if self.current_parcel_scene_id != self.last_current_parcel_scene_id {
            self.on_current_parcel_scene_changed();
        }

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
                    camera_mode,
                    self.console.clone(),
                    &self.current_parcel_scene_id,
                    &self.begin_time,
                    &self.ui_canvas_information,
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
            let signal_data = (*scene_id, scene.definition.entity_id);
            let node_3d = scene
                .godot_dcl_scene
                .root_node_3d
                .clone()
                .upcast::<Node>()
                .clone();
            let node_ui = scene
                .godot_dcl_scene
                .root_node_ui
                .clone()
                .upcast::<Node>()
                .clone();
            self.base.remove_child(node_3d);
            self.base_ui.remove_child(node_ui);
            scene.godot_dcl_scene.root_node_3d.queue_free();
            self.sorted_scene_ids.retain(|x| x != scene_id);
            self.dying_scene_ids.retain(|x| x != scene_id);
            self.scenes.remove(scene_id);

            self.base.emit_signal(
                "scene_killed".into(),
                &[signal_data.0 .0.to_variant(), signal_data.1.to_variant()],
            );
        }
    }

    fn receive_from_thread(&mut self) {
        // TODO: check infinity loop (loop_end_time)
        loop {
            match self.main_receiver_from_thread.try_recv() {
                Ok(response) => match response {
                    SceneResponse::Error(scene_id, msg) => {
                        let mut arguments = VariantArray::new();
                        arguments.push((scene_id.0).to_variant());
                        arguments.push((SceneLogLevel::SystemError as i32).to_variant());
                        arguments.push(self.total_time_seconds_time.to_variant());
                        arguments.push(GString::from(&msg).to_variant());
                        self.console.callv(arguments);
                    }
                    SceneResponse::Ok(scene_id, dirty_crdt_state, logs, _, rpc_calls) => {
                        if let Some(scene) = self.scenes.get_mut(&scene_id) {
                            let dirty = Dirty {
                                waiting_process: true,
                                entities: dirty_crdt_state.entities,
                                lww_components: dirty_crdt_state.lww,
                                gos_components: dirty_crdt_state.gos,
                                logs,
                                renderer_response: None,
                                update_state: SceneUpdateState::None,
                                rpc_calls,
                            };

                            if !scene.current_dirty.waiting_process {
                                scene.current_dirty = dirty;
                            } else {
                                scene.enqueued_dirty.push(dirty);
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
                            arguments.push(scene_id.0.to_variant());
                            arguments.push((log.level as i32).to_variant());
                            arguments.push((log.timestamp as f32).to_variant());
                            arguments.push(GString::from(&log.message).to_variant());
                            self.console.callv(arguments);
                        }
                    }

                    SceneResponse::TakeSnapshot {
                        scene_id,
                        src_stored_snapshot,
                        camera_position,
                        camera_target,
                        screeshot_size,
                        method,
                        response,
                    } => {
                        let offset = if let Some(scene) = self.scenes.get(&scene_id) {
                            Vector3::new(
                                scene.definition.base.x as f32 * 16.0,
                                0.0,
                                -scene.definition.base.y as f32 * 16.0,
                            )
                        } else {
                            Vector3::new(0.0, 0.0, 0.0)
                        };

                        let global_camera_position =
                            Vector3::new(camera_position.x, camera_position.y, camera_position.z)
                                + offset;

                        let global_camera_target =
                            Vector3::new(camera_target.x, camera_target.y, camera_target.z)
                                + offset;

                        let mut testing_tools = DclGlobal::singleton().bind().get_testing_tools();
                        if testing_tools.has_method("async_take_and_compare_snapshot".into()) {
                            let mut dcl_rpc_sender: Gd<DclRpcSenderTakeAndCompareSnapshotResponse> =
                                DclRpcSenderTakeAndCompareSnapshotResponse::new_gd();
                            dcl_rpc_sender.bind_mut().set_sender(response);

                            testing_tools.call(
                                "async_take_and_compare_snapshot".into(),
                                &[
                                    scene_id.0.to_variant(),
                                    src_stored_snapshot.to_variant(),
                                    global_camera_position.to_variant(),
                                    global_camera_target.to_variant(),
                                    screeshot_size.to_variant(),
                                    method
                                        .to_godot_from_json()
                                        .unwrap_or(Dictionary::new().to_variant())
                                        .to_variant(),
                                    dcl_rpc_sender.to_variant(),
                                ],
                            );
                        } else {
                            response.send(Err("Testing tools not available".to_string()));
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

        let scene = self.scenes.get(&SceneId(dcl_scene_id))?;
        let scene_position = scene.godot_dcl_scene.root_node_3d.get_position();
        let raycast_data = RaycastHit::from_godot_raycast(
            scene_position,
            raycast_from,
            &raycast_result,
            Some(dcl_entity_id as u32),
        )?;

        Some(GodotDclRaycastResult {
            scene_id: SceneId(dcl_scene_id),
            entity_id: SceneEntityId::from_i32(dcl_entity_id),
            hit: raycast_data,
        })
    }

    #[signal]
    fn pointer_tooltip_changed() {}

    fn create_ui_canvas_information(&self) -> PbUiCanvasInformation {
        let canvas_size = self.base_ui.get_size();
        let device_pixel_ratio = godot::engine::DisplayServer::singleton().screen_get_dpi() as f32;
        PbUiCanvasInformation {
            device_pixel_ratio,
            width: canvas_size.x as i32,
            height: canvas_size.y as i32,
            interactable_area: Some(BorderRect {
                top: 0.0,
                left: 0.0,
                right: canvas_size.x,
                bottom: canvas_size.y,
            }),
        }
    }

    #[func]
    fn _on_ui_resize(&mut self) {
        self.ui_canvas_information = self.create_ui_canvas_information();
    }

    fn on_current_parcel_scene_changed(&mut self) {
        if let Some(scene) = self.scenes.get_mut(&self.last_current_parcel_scene_id) {
            for (_, audio_source_node) in scene.audio_sources.iter() {
                let mut audio_source_node = audio_source_node.clone();
                audio_source_node.bind_mut().set_dcl_enable(false);
                audio_source_node.call("apply_audio_props".into(), &[false.to_variant()]);
            }
            for (_, audio_stream_node) in scene.audio_streams.iter_mut() {
                audio_stream_node.bind_mut().set_muted(true);
            }
            for (_, video_player_node) in scene.video_players.iter_mut() {
                video_player_node.bind_mut().set_muted(true);
            }

            self.base_ui
                .remove_child(scene.godot_dcl_scene.root_node_ui.clone().upcast());
        }

        if let Some(scene) = self.scenes.get_mut(&self.current_parcel_scene_id) {
            for (_, audio_source_node) in scene.audio_sources.iter() {
                let mut audio_source_node = audio_source_node.clone();
                audio_source_node.bind_mut().set_dcl_enable(true);
                audio_source_node.call("apply_audio_props".into(), &[false.to_variant()]);
            }
            for (_, audio_stream_node) in scene.audio_streams.iter_mut() {
                audio_stream_node.bind_mut().set_muted(false);
            }
            for (_, video_player_node) in scene.video_players.iter_mut() {
                video_player_node.bind_mut().set_muted(false);
            }

            self.base_ui
                .add_child(scene.godot_dcl_scene.root_node_ui.clone().upcast());
        }

        self.last_current_parcel_scene_id = self.current_parcel_scene_id;
        self.base.emit_signal(
            "on_change_scene_id".into(),
            &[Variant::from(self.current_parcel_scene_id.0)],
        );
    }

    #[signal]
    fn on_change_scene_id(scene_id: i32) {}

    pub fn get_all_scenes_mut(&mut self) -> &mut HashMap<SceneId, Scene> {
        &mut self.scenes
    }

    pub fn get_scene_mut(&mut self, scene_id: &SceneId) -> Option<&mut Scene> {
        self.scenes.get_mut(scene_id)
    }

    // this could be cached
    pub fn get_global_scenes(&self) -> Vec<SceneId> {
        self.scenes
            .iter()
            .filter(|(_scene_id, scene)| {
                if let SceneType::Global(_) = scene.scene_type {
                    return true;
                }
                false
            })
            .map(|(scene_id, _)| *scene_id)
            .collect::<Vec<SceneId>>()
    }
}

#[godot_api]
impl INode for SceneManager {
    fn init(base: Base<Node>) -> Self {
        let (thread_sender_to_main, main_receiver_from_thread) =
            std::sync::mpsc::sync_channel(1000);

        SceneManager {
            base,
            base_ui: DclUiControl::alloc_gd(),
            ui_canvas_information: PbUiCanvasInformation::default(),

            scenes: HashMap::new(),
            pause: false,
            sorted_scene_ids: vec![],
            dying_scene_ids: vec![],
            current_parcel_scene_id: SceneId(0),
            last_current_parcel_scene_id: SceneId::INVALID,

            main_receiver_from_thread,
            thread_sender_to_main,

            camera_node: DclCamera3D::alloc_gd(),
            player_node: Node3D::new_alloc(),

            player_position: Vector2i::new(-1000, -1000),

            total_time_seconds_time: 0.0,
            begin_time: Instant::now(),
            console: Callable::invalid(),
            input_state: InputState::default(),
            last_raycast_result: None,
            pointer_tooltips: VariantArray::new(),
        }
    }

    fn ready(&mut self) {
        let callable = self.base.get("_on_ui_resize".into()).to::<Callable>();
        self.base_ui.connect("resized".into(), callable);
        self.base_ui.set_name("scenes_ui".into());
    }

    fn process(&mut self, delta: f64) {
        self.scene_runner_update(delta);

        let changed_inputs = self.input_state.get_new_inputs();
        let current_pointer_raycast_result = self.get_current_mouse_entity();

        pointer_events_system(
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
                                    GString::from(text)
                                } else {
                                    GString::from("Interact")
                                };

                                let mut dict = Dictionary::new();
                                dict.set(StringName::from("text"), text);
                                dict.set(
                                    StringName::from("action"),
                                    GString::from(input_action.as_str_name()),
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

            self.set_pointer_tooltips(tooltips);
            self.base.emit_signal("pointer_tooltip_changed".into(), &[]);
        }

        // This update the mirror node that copies every frame the global transform of the player/camera
        //  every entity attached to the player/camera is really attached to these mirror nodes
        // TODO: should only update the current scnes + globals?
        for (_, scene) in self.scenes.iter_mut() {
            if let Some(player_node) = scene
                .godot_dcl_scene
                .get_node_3d_mut(&SceneEntityId::PLAYER)
            {
                player_node.set_global_transform(self.player_node.get_global_transform());
            }

            if let Some(camera_node) = scene
                .godot_dcl_scene
                .get_node_3d_mut(&SceneEntityId::CAMERA)
            {
                camera_node.set_global_transform(self.player_node.get_global_transform());
            }
        }

        self.last_raycast_result = current_pointer_raycast_result;
        GLOBAL_TICK_NUMBER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
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
