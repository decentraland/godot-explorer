use crate::{
    content::content_mapping::DclContentMappingAndUrl,
    dcl::{
        common::SceneLogLevel,
        components::{
            internal_player_data::InternalPlayerData,
            proto_components::{
                common::BorderRect,
                sdk::components::{
                    common::{InputAction, PointerEventType, RaycastHit},
                    PbUiCanvasInformation,
                },
            },
            SceneEntityId,
        },
        DclScene, RendererResponse, SceneDefinition, SceneId, SceneResponse,
    },
    godot_classes::{
        dcl_camera_3d::DclCamera3D, dcl_global::DclGlobal, dcl_ui_control::DclUiControl,
        rpc_sender::take_and_compare_snapshot_response::DclRpcSenderTakeAndCompareSnapshotResponse,
        JsonGodotClass,
    },
};
use godot::{
    engine::{
        control::{LayoutPreset, MouseFilter},
        PhysicsRayQueryParameters3D,
    },
    prelude::*,
};
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
    interactable_area: Rect2i,

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
    fn start_scene(
        &mut self,
        scene_definition: Dictionary,
        shared_content_mapping: Gd<DclContentMappingAndUrl>,
    ) -> i32 {
        let scene_definition = match SceneDefinition::from_dict(scene_definition) {
            Ok(scene_definition) => scene_definition,
            Err(e) => {
                tracing::info!("error parsing scene definition: {e:?}");
                return 0;
            }
        };

        let content_mapping = shared_content_mapping.bind().get_content_mapping();
        let scene_type = if scene_definition.is_global {
            SceneType::Global(GlobalSceneType::GlobalRealm)
        } else {
            SceneType::Parcel
        };

        let new_scene_id = Scene::new_id();
        let signal_data = (new_scene_id, scene_definition.entity_id.clone());
        let testing_mode_active = DclGlobal::singleton().bind().testing_scene_mode;
        let ethereum_provider = DclGlobal::singleton().bind().ethereum_provider.clone();
        let ephemeral_wallet = DclGlobal::singleton()
            .bind()
            .player_identity
            .bind()
            .try_get_ephemeral_auth_chain();
        let dcl_scene = DclScene::spawn_new_js_dcl_scene(
            new_scene_id,
            scene_definition.clone(),
            content_mapping.clone(),
            self.thread_sender_to_main.clone(),
            testing_mode_active,
            ethereum_provider,
            ephemeral_wallet,
        );

        let new_scene = Scene::new(
            new_scene_id,
            scene_definition,
            dcl_scene,
            content_mapping.clone(),
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

        self.base.call_deferred(
            "emit_signal".into(),
            &[
                "scene_spawned".to_variant(),
                signal_data.0 .0.to_variant(),
                signal_data.1.to_variant(),
            ],
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
    fn get_scene_content_mapping(&self, scene_id: i32) -> Gd<DclContentMappingAndUrl> {
        if let Some(scene) = self.scenes.get(&SceneId(scene_id)) {
            DclContentMappingAndUrl::from_ref(scene.content_mapping.clone())
        } else {
            DclContentMappingAndUrl::empty()
        }
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
                if scene.dcl_scene.thread_join_handle.is_finished() {
                    tracing::error!("scene closed without kill signal");
                    scene_to_remove.insert(*scene_id);
                    continue;
                }

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
            if node_ui.get_parent().is_some() {
                self.base_ui.remove_child(node_ui);
            }
            scene.godot_dcl_scene.root_node_3d.queue_free();
            self.sorted_scene_ids.retain(|x| x != scene_id);
            self.dying_scene_ids.retain(|x| x != scene_id);
            self.scenes.remove(scene_id);

            if scene.dcl_scene.thread_join_handle.is_finished() {
                if let Err(err) = scene.dcl_scene.thread_join_handle.join() {
                    let msg = if let Some(panic_info) = err.downcast_ref::<&str>() {
                        format!("Thread panicked with: {}", panic_info)
                    } else if let Some(panic_info) = err.downcast_ref::<String>() {
                        format!("Thread panicked with: {}", panic_info)
                    } else {
                        "Thread panicked with an unknown payload".to_string()
                    };
                    tracing::error!("scene {} thread result: {:?}", scene_id.0, msg);
                }
            }

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

                            testing_tools.call_deferred(
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

    #[func]
    fn is_paused(&mut self) -> bool {
        self.pause
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
        let window_size: Vector2i = godot::engine::DisplayServer::singleton().window_get_size();

        let device_pixel_ratio = window_size.y as f32 / canvas_size.y;

        PbUiCanvasInformation {
            device_pixel_ratio,
            width: canvas_size.x as i32,
            height: canvas_size.y as i32,
            interactable_area: Some(BorderRect {
                top: self.interactable_area.position.x as f32,
                left: self.interactable_area.position.y as f32,
                right: self.interactable_area.end().x as f32,
                bottom: self.interactable_area.end().y as f32,
            }),
        }
    }

    #[func]
    fn _on_ui_resize(&mut self) {
        self.ui_canvas_information = self.create_ui_canvas_information();
    }

    #[func]
    fn set_interactable_area(&mut self, interactable_area: Rect2i) {
        self.interactable_area = interactable_area;
        self.ui_canvas_information = self.create_ui_canvas_information();
    }

    #[func]
    fn get_current_parcel_scene_id(&self) -> i32 {
        self.current_parcel_scene_id.0
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

            scene
                .avatar_scene_updates
                .internal_player_data
                .insert(SceneEntityId::PLAYER, InternalPlayerData { inside: false });

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

            scene
                .avatar_scene_updates
                .internal_player_data
                .insert(SceneEntityId::PLAYER, InternalPlayerData { inside: true });

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

    pub fn get_all_scenes(&mut self) -> &HashMap<SceneId, Scene> {
        &self.scenes
    }

    pub fn get_scene_mut(&mut self, scene_id: &SceneId) -> Option<&mut Scene> {
        self.scenes.get_mut(scene_id)
    }

    pub fn get_scene(&self, scene_id: &SceneId) -> Option<&Scene> {
        self.scenes.get(scene_id)
    }

    // this could be cached
    pub fn get_global_scene_ids(&self) -> Vec<SceneId> {
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

    #[func]
    pub fn is_scene_tests_finished(&self, scene_id: i32) -> bool {
        let Some(scene) = self.scenes.get(&SceneId(scene_id)) else {
            return false;
        };

        scene.scene_test_plan_received
            && scene
                .scene_tests
                .iter()
                .all(|(_, test_result)| test_result.is_some())
    }

    #[func]
    pub fn get_scene_tests_result(&self, scene_id: i32) -> Dictionary {
        let Some(scene) = self.scenes.get(&SceneId(scene_id)) else {
            return Dictionary::default();
        };

        let test_total = scene.scene_tests.len() as u32;
        let mut test_fail = 0;
        let mut text_test_list = String::new();
        let mut text_detail_failed = String::new();
        for value in scene.scene_tests.iter() {
            if let Some(result) = value.1 {
                if result.ok {
                    text_test_list += &format!(
                        "\t🟢 {} (frames={},time={}): OK\n",
                        value.0, result.total_frames, result.total_time
                    );
                } else {
                    text_test_list += &format!(
                        "\t🔴 {} (frames={},time={}):",
                        value.0, result.total_frames, result.total_time
                    );
                    test_fail += 1;
                    if let Some(error) = &result.error {
                        text_test_list += "\tFAIL with Error\n";
                        text_detail_failed += &format!("🔴{}: ❌{}\n", value.0, error);
                    } else {
                        text_test_list += "\tFAIL with Unknown Error \n";
                    }
                }
            }
        }

        let mut text = format!("Scene {:?} tests:\n", scene.definition.title);
        text += &format!("{}\n", text_test_list);
        if test_fail == 0 {
            text += &format!(
                "✅ All tests ({}) passed in the scene {:?}\n",
                test_total, scene.definition.title
            );
        } else {
            text += &format!(
                "❌ {} tests failed of {} in the scene {:?}\n",
                test_fail, test_total, scene.definition.title
            );
        }

        let mut dict = Dictionary::default();
        dict.set("text", text.to_variant());
        dict.set("text_detail_failed", text_detail_failed.to_variant());
        dict.set("total", test_total.to_variant());
        dict.set("fail", test_fail.to_variant());

        dict
    }
}

#[godot_api]
impl INode for SceneManager {
    fn init(base: Base<Node>) -> Self {
        let (thread_sender_to_main, main_receiver_from_thread) =
            std::sync::mpsc::sync_channel(1000);

        let mut base_ui = DclUiControl::alloc_gd();
        base_ui.set_anchors_preset(LayoutPreset::PRESET_FULL_RECT);
        base_ui.set_mouse_filter(MouseFilter::MOUSE_FILTER_IGNORE);

        let canvas_size = base_ui.get_size();

        SceneManager {
            base,
            base_ui,
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
            interactable_area: Rect2i::from_components(
                0,
                0,
                canvas_size.x as i32,
                canvas_size.y as i32,
            ),
        }
    }

    fn ready(&mut self) {
        self.base_ui
            .connect("resized".into(), self.base.callable("_on_ui_resize"));
        self.base_ui.set_name("scenes_ui".into());
        self.ui_canvas_information = self.create_ui_canvas_information();
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

        let should_update_tooltip = !GodotDclRaycastResult::eq_key(
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

                            let is_pet_up =
                                pointer_event.event_type == PointerEventType::PetUp as i32;
                            let is_pet_down =
                                pointer_event.event_type == PointerEventType::PetDown as i32;
                            if is_pet_up || is_pet_down {
                                let text = if let Some(text) = info.hover_text.as_ref() {
                                    GString::from(text)
                                } else {
                                    GString::from("Interact")
                                };

                                let input_action_gstr = GString::from(input_action.as_str_name());

                                let dict = tooltips.iter_shared().find_map(|tooltip| {
                                    let dictionary = tooltip.to::<Dictionary>();
                                    dictionary.get("action").and_then(|action| {
                                        if action.to_string() == input_action_gstr.to_string() {
                                            Some(dictionary.clone())
                                        } else {
                                            None
                                        }
                                    })
                                });

                                let exists = dict.is_some();
                                let mut dict = dict.unwrap_or_else(Dictionary::new);

                                if is_pet_down {
                                    dict.set(StringName::from("text_pet_down"), text);
                                } else if is_pet_up {
                                    dict.set(StringName::from("text_pet_up"), text);
                                }

                                dict.set(StringName::from("action"), input_action_gstr);

                                if !exists {
                                    tooltips.push(dict.to_variant());
                                }
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
                camera_node.set_global_transform(self.camera_node.get_global_transform());
            }
        }

        self.last_raycast_result = current_pointer_raycast_result;
        GLOBAL_TICK_NUMBER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}
