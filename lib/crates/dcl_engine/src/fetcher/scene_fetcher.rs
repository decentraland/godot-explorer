use std::collections::BTreeMap;

use super::globals;
use crate::godot_classes::dcl_gltf_container::DclGltfContainer;
use glam::IVec2;
use godot::classes::{
    file_access::ModeFlags, FileAccess, INode, PhysicsShapeQueryParameters3D, SphereShape3D,
};
use godot::prelude::*;

use super::{
    base_floor::BaseFloorManager,
    layout::{island_layout, PADDING},
    preview_ws::{PreviewEvent, PreviewSocket},
    walls::FloatingIslandWalls,
};
use crate::{
    engine::local_task::{
        await_promise, next_frame, next_physics_frame, physics_tick, spawn_local,
    },
    godot_classes::{
        dcl_floating_islands_manager::DclFloatingIslandsManager, dcl_global::DclGlobal,
        promise::PromiseError,
    },
    realm::{
        dcl_scene_entity_definition::DclSceneEntityDefinition,
        scene_entity_coordinator::SceneEntityCoordinator,
    },
    scene_runner::scene_manager::SceneManager,
};

fn gs(text: &str) -> GString {
    GString::from(text)
}

const PARCEL_SIZE: f32 = 16.0;
const PARCEL_HALF_SIZE: f32 = 8.0;
const ADAPTATION_LAYER_URL: &str =
    "https://renderer-artifacts.decentraland.org/sdk6-adaption-layer/main/index.min.js";
const INVALID_PARCEL: Vector2i = Vector2i::new(-1000, -1000);
const NO_SCENE: i32 = -1;

/// How long a deleted model keeps swallowing change events for its own path: just above the
/// preview server's 800ms debounce, so the trailing event a delete emits lands inside the
/// window while a human re-creating a file does not.
const REMOVED_MODEL_GRACE_MS: u64 = 1500;

/// Radius of scenes kept around the player when scenes load continuously.
const CONTINUOUS_SCENE_RADIUS: i16 = 5;

/// Frames to wait for the coordinator to finish fetching before laying out the islands.
const MAX_COORDINATOR_WAIT_FRAMES: u32 = 300;
const COORDINATOR_START_WAIT_FRAMES: u32 = 30;

pub struct SceneItem {
    pub id: String,
    pub definition: Gd<DclSceneEntityDefinition>,
    /// `NO_SCENE` while the scene is still being fetched.
    pub scene_number_id: i32,
    pub parcels: Vec<Vector2i>,
    pub is_global: bool,
}

impl SceneItem {
    fn is_running(&self) -> bool {
        self.scene_number_id != NO_SCENE
    }
}

#[derive(GodotClass)]
#[class(base=Node)]
pub struct SceneFetcher {
    base: Base<Node>,

    current_position: Vector2i,
    current_scene_entity_id: String,

    islands_manager: Option<Gd<DclFloatingIslandsManager>>,
    wall_manager: Option<Gd<FloatingIslandWalls>>,
    base_floor_manager: Option<Gd<BaseFloorManager>>,
    coordinator: Gd<SceneEntityCoordinator>,
    loaded_scenes: BTreeMap<String, SceneItem>,

    last_version_updated: i64,
    last_version_checked: i64,
    last_scene_group_hash: u64,
    desired_portable_experiences_urns: Vec<String>,

    /// Continuous loading: scenes come and go with the player, no loading screen, no terrain.
    use_dynamic_loading: bool,
    /// One-shot: a teleport or reload wants the loading screen even in dynamic mode.
    is_reloading: bool,
    /// One-shot: a preview hot reload skips the loading screen.
    is_hot_reloading: bool,
    /// Preview mode purges the first scene's cached files so stale content is never used
    /// before the socket's SCENE_UPDATE can trigger a reload.
    purge_cache_on_first_load: bool,
    /// Bumped by every `async_on_desired_scene_changed`; an older run bails out when it sees a
    /// newer one started.
    scene_changed_counter: u64,
    debugging_js_scene_id: String,
    bypass_loading_check: bool,
    teleport_target_parcel: Vector2i,
    coordinator_configured: bool,

    /// The candidate parcels handed to `islands_manager`, so `is_scene_loaded` and
    /// `parcels_processed` can list empty parcels without asking the manager.
    candidate_set: std::collections::HashSet<(i32, i32)>,
    last_candidate_parcels: Vec<Vector2i>,
    /// Coalesces regeneration requests made in the same frame.
    regen_scheduled: bool,
    /// The next `generation_complete` teleports the player onto this empty parcel.
    pending_empty_parcel_spawn: Vector2i,

    preview: PreviewSocket,
    /// Models dropped by an UMT_REMOVE, as src -> uptime ms. Deleting a file makes the server
    /// emit both an immediate UMT_REMOVE and, up to one debounce later, an UMT_CHANGE for the
    /// same missing path; the second one looks exactly like a re-created file.
    recently_removed_models: std::collections::HashMap<String, u64>,
}

#[godot_api]
impl INode for SceneFetcher {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            current_position: INVALID_PARCEL,
            current_scene_entity_id: String::new(),
            islands_manager: None,
            wall_manager: None,
            base_floor_manager: None,
            coordinator: SceneEntityCoordinator::new_alloc(),
            loaded_scenes: BTreeMap::new(),
            last_version_updated: -1,
            last_version_checked: -1,
            last_scene_group_hash: 0,
            desired_portable_experiences_urns: Vec::new(),
            use_dynamic_loading: false,
            is_reloading: false,
            is_hot_reloading: false,
            purge_cache_on_first_load: false,
            scene_changed_counter: 0,
            debugging_js_scene_id: String::new(),
            bypass_loading_check: false,
            teleport_target_parcel: INVALID_PARCEL,
            coordinator_configured: false,
            candidate_set: Default::default(),
            last_candidate_parcels: Vec::new(),
            regen_scheduled: false,
            pending_empty_parcel_spawn: INVALID_PARCEL,
            preview: PreviewSocket::default(),
            recently_removed_models: Default::default(),
        }
    }

    fn ready(&mut self) {
        let this = self.to_gd();
        let global = DclGlobal::singleton();
        let realm = globals::realm();
        realm.clone().connect(
            "realm_changed",
            &Callable::from_fn("fetcher_realm_changed", {
                let mut this = this.clone();
                move |_| {
                    this.bind_mut().on_realm_changed();
                    Variant::nil()
                }
            }),
        );

        if self.is_using_floating_islands() {
            let mut walls = FloatingIslandWalls::new_alloc();
            self.base_mut().add_child(&walls);
            self.wall_manager = Some(walls.clone());
            walls.set_name("FloatingIslandWalls");

            let mut floors = BaseFloorManager::new_alloc();
            floors.set_name("BaseFloorManager");
            self.base_mut().add_child(&floors);
            self.base_floor_manager = Some(floors);

            let mut islands = DclFloatingIslandsManager::new_alloc();
            islands.set_name("FloatingIslandsManager");
            self.base_mut().add_child(&islands);
            islands.connect(
                "generation_progress",
                &Callable::from_fn("fetcher_islands_progress", |args| {
                    let (created, total) = (args[0].to::<i32>(), args[1].to::<i32>());
                    globals::scene_runner()
                        .bind_mut()
                        .report_floating_islands_progress(created, total);
                    Variant::nil()
                }),
            );
            islands.connect(
                "generation_complete",
                &Callable::from_fn("fetcher_islands_complete", {
                    let mut this = this.clone();
                    move |_| {
                        on_islands_generation_complete(&mut this);
                        Variant::nil()
                    }
                }),
            );
            self.islands_manager = Some(islands);
        }

        let radius = self.scene_radius();
        self.coordinator.bind_mut().set_scene_radius(radius);

        let mut scene_runner = globals::scene_runner();
        scene_runner.connect(
            "scene_killed",
            &Callable::from_fn("fetcher_scene_killed", {
                let mut this = this.clone();
                move |args| {
                    this.bind_mut().on_scene_killed(args[0].to::<i32>());
                    Variant::nil()
                }
            }),
        );
        scene_runner.connect(
            "scene_crashed",
            &Callable::from_fn("fetcher_scene_crashed", |args| {
                let crashed = args[0].to::<i32>();
                let current = globals::scene_runner().bind().get_current_parcel_scene_id();
                if crashed == current {
                    globals::engine().bind().report_scene_crashed();
                }
                Variant::nil()
            }),
        );
        // The memory monitor found critical pressure with nothing left to evict but the current
        // scene (issue #2002); Rust emits this once per pressure episode.
        scene_runner.connect(
            "low_memory_warning",
            &Callable::from_fn("fetcher_low_memory", |args| {
                godot_warn!(
                    "[memory] critical pressure in {}: {} MB footprint, {} MB available",
                    args[1].to::<GString>(),
                    args[2].to::<i32>(),
                    args[3].to::<i32>()
                );
                globals::engine()
                    .bind()
                    .report_notice("low_memory".into(), "".into());
                Variant::nil()
            }),
        );
        global.clone().connect(
            "loading_finished",
            &Callable::from_fn("fetcher_loading_finished", {
                let mut this = this.clone();
                move |_| {
                    on_loading_finished(&mut this);
                    Variant::nil()
                }
            }),
        );
    }

    fn process(&mut self, _delta: f64) {
        for event in self.preview.poll() {
            let this = self.to_gd();
            match event {
                PreviewEvent::SceneUpdate { scene_id } => {
                    spawn_local(hot_reload(this, scene_id));
                }
                PreviewEvent::ModelUpdate {
                    scene_id,
                    src,
                    removed,
                    ..
                } => {
                    spawn_local(on_preview_model_update(this, scene_id, src, removed));
                }
            }
        }

        self.coordinator.bind_mut().update();
        let version = self.coordinator.bind().get_version() as i64;

        let continuous = !self.is_using_floating_islands() || self.use_dynamic_loading;
        if continuous {
            if version != self.last_version_updated {
                self.last_version_updated = version;
                self.start_desired_scenes_changed();
            }
            return;
        }

        let scene_entity_id = self.scene_entity_id_at(self.current_position);
        let at_invalid_position = self.current_position == INVALID_PARCEL;
        let has_actual_change = scene_entity_id != self.current_scene_entity_id
            && !(scene_entity_id.is_empty() && self.current_scene_entity_id.is_empty());

        if self.bypass_loading_check || (has_actual_change && !at_invalid_position) {
            self.current_scene_entity_id = scene_entity_id;
            self.bypass_loading_check = false;
            if version != self.last_version_updated {
                self.last_version_updated = version;
                self.base_mut()
                    .emit_signal("notify_pending_loading_scenes", &[false.to_variant()]);
                self.start_desired_scenes_changed();
            }
        } else if version != self.last_version_updated {
            // Same scene, new coordinator data: dynamic loading still needs the update.
            self.last_version_updated = version;
            self.start_desired_scenes_changed();
        } else if version != self.last_version_checked {
            self.last_version_checked = version;
            if self.is_there_any_new_scene_to_load() {
                self.base_mut()
                    .emit_signal("notify_pending_loading_scenes", &[true.to_variant()]);
            }
        }
    }

    fn physics_process(&mut self, _delta: f64) {
        physics_tick();
    }
}

#[godot_api]
impl SceneFetcher {
    #[signal]
    fn parcels_processed(parcel_filled: VarArray, empty: VarArray);
    #[signal]
    fn report_scene_load(done: bool, is_new_loading: bool, pending: i32);
    #[signal]
    fn notify_pending_loading_scenes(is_pending: bool);
    #[signal]
    fn player_parcel_changed(new_position: Vector2i);

    #[func]
    pub fn get_coordinator(&self) -> Gd<SceneEntityCoordinator> {
        self.coordinator.clone()
    }

    #[func]
    pub fn get_current_position(&self) -> Vector2i {
        self.current_position
    }

    #[func]
    pub fn get_current_scene_entity_id(&self) -> GString {
        self.current_scene_entity_id.as_str().into()
    }

    #[func]
    pub fn get_islands_manager(&self) -> Option<Gd<DclFloatingIslandsManager>> {
        self.islands_manager.clone()
    }

    #[func]
    pub fn get_loaded_scene_count(&self) -> i64 {
        self.loaded_scenes.len() as i64
    }

    /// The parcel the current scene asks the player to spawn at; `Vector3.ZERO`-less callers use
    /// `has_spawn_point` first.
    #[func]
    pub fn get_current_spawn_point(&self) -> Variant {
        match self
            .loaded_scenes
            .get(&self.scene_entity_id_at(self.current_position))
        {
            Some(scene) => scene
                .definition
                .bind()
                .get_global_spawn_position()
                .to_variant(),
            None => Variant::nil(),
        }
    }

    #[func]
    pub fn get_scene_definition_at(&self, coord: Vector2i) -> Option<Gd<DclSceneEntityDefinition>> {
        let id = self.scene_entity_id_at(coord);
        if id.is_empty() {
            return None;
        }
        self.coordinator.bind().get_scene_definition(gs(&id))
    }

    #[func]
    pub fn set_scene_radius(&mut self, value: i32) {
        self.coordinator.bind_mut().set_scene_radius(value as i16);
    }

    #[func]
    pub fn is_dynamic_loading_mode(&self) -> bool {
        self.use_dynamic_loading
    }

    /// Continuous loading for deep links to custom realms: no loading screen, no island terrain
    /// (simple base floors instead).
    #[func]
    pub fn set_dynamic_loading_mode(&mut self, enabled: bool) {
        if self.use_dynamic_loading == enabled {
            return;
        }
        self.use_dynamic_loading = enabled;
        let radius = self.scene_radius();
        self.coordinator.bind_mut().set_scene_radius(radius);
        if enabled {
            self.last_scene_group_hash = 0;
            self.clear_floating_islands_state();
        }
    }

    #[func]
    pub fn is_scene_loaded(&self, x: i32, z: i32) -> bool {
        self.get_parcel_scene_id(x, z) != NO_SCENE || self.candidate_set.contains(&(x, z))
    }

    #[func]
    pub fn get_parcel_scene_id(&self, x: i32, z: i32) -> i32 {
        // Scenes may overlap (a world's big scene around small ones): the most specific one wins,
        // and the id breaks ties, so the answer never depends on the map's order.
        self.loaded_scenes
            .values()
            .filter(|scene| scene.is_running())
            .filter(|scene| scene.parcels.iter().any(|p| p.x == x && p.y == z))
            .min_by_key(|scene| (scene.parcels.len(), scene.scene_number_id))
            .map(|scene| scene.scene_number_id)
            .unwrap_or(NO_SCENE)
    }

    #[func]
    pub fn is_using_floating_islands(&self) -> bool {
        let cli = globals::cli();
        let cli = cli.bind();
        !cli.scene_test_mode
    }

    #[func]
    pub fn set_debugging_js_scene_id(&mut self, id: GString) {
        self.debugging_js_scene_id = id.to_string();
    }

    #[func]
    pub fn set_preview_url(&mut self, url: GString) {
        self.preview.set_url(&url.to_string());
    }

    #[func]
    pub fn set_portable_experiences_urns(&mut self, urns: PackedStringArray) {
        self.desired_portable_experiences_urns =
            urns.to_vec().iter().map(|s| s.to_string()).collect();
        self.push_global_urns();
    }

    /// Drops the per-session scene state so the next login starts clean; the Rust scenes
    /// themselves are torn down by `kill_all_scenes`.
    #[func]
    pub fn reset_for_logout(&mut self) {
        self.loaded_scenes.clear();
        self.current_position = INVALID_PARCEL;
        self.current_scene_entity_id.clear();
        self.last_version_updated = -1;
        self.last_version_checked = -1;
        self.last_scene_group_hash = 0;
        self.desired_portable_experiences_urns.clear();
        if let Some(islands) = &mut self.islands_manager {
            islands.bind_mut().clear_all();
        }
        if let Some(walls) = &mut self.wall_manager {
            walls.bind_mut().clear_walls();
        }
        if let Some(floors) = &mut self.base_floor_manager {
            floors.bind_mut().clear_all_floors();
        }
    }

    #[func]
    pub fn update_position(&mut self, new_position: Vector2i, is_teleport: bool) {
        // A teleport to the same place still reloads the scene.
        if self.current_position == new_position && !is_teleport {
            return;
        }
        let position_changed = self.current_position != new_position;
        self.current_position = new_position;

        if is_teleport {
            self.teleport_target_parcel = new_position;
            // The loading screen shows even in dynamic loading mode so scenes are ready before
            // the player can act.
            self.is_reloading = true;
            if !position_changed && self.is_using_floating_islands() {
                self.bypass_loading_check = true;
            }
            self.kill_loaded_scenes(|_| true);
            self.loaded_scenes.clear();
            self.last_scene_group_hash = 0;
        }

        // Only a configured coordinator can take a position.
        if self.coordinator_configured
            && (!self.is_using_floating_islands() || is_teleport || self.use_dynamic_loading)
        {
            self.coordinator
                .bind_mut()
                .set_current_position(new_position.x as i16, new_position.y as i16);
        }
        if let Some(islands) = &mut self.islands_manager {
            islands.bind_mut().set_player_parcel(new_position);
        }
        self.base_mut()
            .emit_signal("player_parcel_changed", &[new_position.to_variant()]);
    }

    /// Lays the islands out around `parcels` as if scenes held them, for the island benchmark.
    #[func]
    pub fn benchmark_generate_islands(&mut self, parcels: Array<Vector2i>) {
        self.last_scene_group_hash = 0;
        let parcels: Vec<IVec2> = parcels
            .iter_shared()
            .map(|p| IVec2::new(p.x, p.y))
            .collect();
        let mut this = self.to_gd();
        // Off this bind: the layout talks to nodes whose signals call back into the fetcher.
        spawn_local(async move {
            let plan = this.bind_mut().plan_islands(parcels, None);
            if let Some(plan) = plan {
                plan.apply();
            }
        });
    }
}

impl SceneFetcher {
    /// The title of the scene under the player, `None` while none is loaded.
    pub fn current_scene_title(&self) -> Option<String> {
        let scene = self
            .loaded_scenes
            .get(&self.scene_entity_id_at(self.current_position))?;
        let title = scene.definition.bind().get_title().to_string();
        Some(title).filter(|title| !title.is_empty())
    }

    fn scene_radius(&self) -> i16 {
        if self.use_dynamic_loading {
            CONTINUOUS_SCENE_RADIUS
        } else {
            0
        }
    }

    fn scene_entity_id_at(&self, coord: Vector2i) -> String {
        self.coordinator
            .bind()
            .get_scene_entity_id(coord)
            .to_string()
    }

    fn kill_loaded_scenes(&mut self, mut predicate: impl FnMut(&SceneItem) -> bool) -> Vec<String> {
        let mut removed = Vec::new();
        for (id, scene) in &self.loaded_scenes {
            if !scene.is_global && scene.is_running() && predicate(scene) {
                kill_scene(scene.scene_number_id);
                if let Some(floors) = &mut self.base_floor_manager {
                    floors.bind_mut().remove_scene_floors(&scene.id);
                }
                removed.push(id.clone());
            }
        }
        removed
    }

    /// Where the player stands once loading ends: the spawn point of the scene they went to,
    /// lifted out of any collider.
    fn spawn_after_loading(&mut self) -> Option<Vector3> {
        let mut spawn_parcel = self.current_position;
        if self.is_using_floating_islands() && self.teleport_target_parcel != INVALID_PARCEL {
            spawn_parcel = self.teleport_target_parcel;
            self.teleport_target_parcel = INVALID_PARCEL;
        }
        let id = self.scene_entity_id_at(spawn_parcel);
        let scene = self.loaded_scenes.get(&id)?;
        let target = scene.definition.bind().get_global_spawn_position();
        Some(self.find_valid_spawn_position(target))
    }

    fn on_scene_killed(&mut self, killed_scene_id: i32) {
        self.loaded_scenes
            .retain(|_, scene| scene.scene_number_id != killed_scene_id);
    }

    fn is_there_any_new_scene_to_load(&self) -> bool {
        desired_scene_ids(&self.coordinator, "loadable_scenes")
            .iter()
            .any(|id| {
                !self.loaded_scenes.contains_key(id)
                    && self
                        .coordinator
                        .bind()
                        .get_scene_definition(id.as_str().into())
                        .is_some()
            })
    }

    fn start_desired_scenes_changed(&self) {
        spawn_local(async_on_desired_scene_changed(self.to_gd()));
    }

    fn on_realm_changed(&mut self) {
        let global = DclGlobal::singleton();
        let realm = globals::realm();
        let (mut content_base_url, realm_url, about, city_loader_url) = {
            let realm = realm.bind();
            (
                realm.get_content_base_url().to_string(),
                realm.get_realm_url(),
                realm.get_realm_about(),
                realm.get_realm_city_loader_content_base_url().to_string(),
            )
        };
        globals::engine().bind().set_setting(
            "last_realm_joined".into(),
            realm_url.to_variant(),
            true,
        );

        // Preview: purge the first scene's cache so stale content is never used before the
        // socket's SCENE_UPDATE can trigger a reload.
        let preview_deep_link = global
            .get("deep_link_obj")
            .try_to::<Gd<Object>>()
            .ok()
            .map(|link| link.get("preview").to::<GString>().is_empty())
            == Some(false);
        if globals::cli().bind().preview_mode || preview_deep_link {
            self.purge_cache_on_first_load = true;
        }

        // `&dynamic-scene-loading=true` works for any realm, Genesis City included.
        let deep_link_dynamic = global
            .get("deep_link_obj")
            .try_to::<Gd<Object>>()
            .ok()
            .map(|link| link.get("dynamic_scene_loading").to::<bool>())
            .unwrap_or(false);
        self.set_dynamic_loading_mode(deep_link_dynamic && self.is_using_floating_islands());

        // A fresh realm lays its islands out again.
        self.last_scene_group_hash = 0;

        if !city_loader_url.is_empty() {
            content_base_url = city_loader_url;
        }
        // Floating islands load the realm's single scene; only the offline test/renderer modes
        // walk city pointers.
        let load_city_pointers = !self.is_using_floating_islands();
        self.coordinator.bind_mut().config(
            gs(&format!("{content_base_url}entities/active")),
            content_base_url.as_str().into(),
            load_city_pointers,
        );
        self.coordinator_configured = true;

        let scenes_urn = about
            .get("configurations")
            .and_then(|c| c.try_to::<VarDictionary>().ok())
            .and_then(|c| c.get("scenesUrn"))
            .and_then(|v| v.try_to::<VarArray>().ok())
            .unwrap_or_default();
        self.coordinator
            .bind_mut()
            .set_fixed_desired_entities_urns(scenes_urn);
        self.coordinator.bind_mut().set_current_position(
            self.current_position.x as i16,
            self.current_position.y as i16,
        );

        self.push_global_urns();
        self.kill_loaded_scenes(|_| true);
        self.clear_floating_islands_state();
        self.loaded_scenes.clear();
    }

    /// The realm's global scenes plus the running portable experiences.
    fn push_global_urns(&mut self) {
        let realm = globals::realm();
        let mut urns = realm
            .bind()
            .get_realm_about()
            .get("configurations")
            .and_then(|c| c.try_to::<VarDictionary>().ok())
            .and_then(|c| c.get("globalScenesUrn"))
            .and_then(|v| v.try_to::<VarArray>().ok())
            .unwrap_or_default();
        for urn in &self.desired_portable_experiences_urns {
            urns.push(&urn.to_variant());
        }
        self.coordinator
            .bind_mut()
            .set_fixed_desired_entities_global_urns(urns);
    }

    /// Genesis City ignores scene.json `landscapeTerrain: false`; multi-scene setups keep the
    /// terrain too (Unity Explorer parity). Only a single loaded world or local-dev scene can
    /// opt out.
    fn should_disable_landscape_terrain(&self) -> bool {
        let realm = globals::realm();
        let realm_name = realm.bind().get_realm_name().to_string();
        if dcl_core::realm::is_genesis_city(&realm_name) {
            return false;
        }
        let mut scenes = self.loaded_scenes.values().filter(|scene| !scene.is_global);
        match (scenes.next(), scenes.next()) {
            (Some(scene), None) => !scene.definition.bind().is_landscape_terrain_enabled(),
            _ => false,
        }
    }

    fn request_regenerate_floating_islands(&mut self) {
        if self.regen_scheduled {
            return;
        }
        self.regen_scheduled = true;
        let mut this = self.to_gd();
        spawn_local(async move {
            next_frame().await;
            regenerate_floating_islands(&mut this);
        });
    }

    /// The islands to lay out for the loaded scenes, `None` when nothing changed.
    fn plan_regeneration(&mut self) -> Option<IslandPlan> {
        self.regen_scheduled = false;
        self.islands_manager.as_ref()?;
        // Honor `landscapeTerrain: false`. No floating-islands phase starts, so the loading
        // session treats it as skipped and reaches 100%.
        if self.should_disable_landscape_terrain() {
            self.clear_floating_islands_state();
            self.last_scene_group_hash = 0;
            return None;
        }

        // Every non-global scene counts, including those still loading: their parcels are
        // known from metadata before they spawn.
        let mut parcels: Vec<IVec2> = self
            .loaded_scenes
            .values()
            .filter(|scene| !scene.is_global)
            .flat_map(|scene| scene.parcels.iter().map(|p| IVec2::new(p.x, p.y)))
            .collect();

        let mut empty_center = None;
        if parcels.is_empty() {
            // Nothing loaded: the island sits under the player (or the teleport target).
            let target = if self.teleport_target_parcel != INVALID_PARCEL {
                self.teleport_target_parcel
            } else {
                self.current_position
            };
            if target == INVALID_PARCEL {
                return None;
            }
            let center = IVec2::new(target.x, target.y);
            empty_center = Some(center);
            parcels = vec![center];
        }

        self.plan_islands(parcels, empty_center)
    }

    /// Walls and bookkeeping for the terrain candidates around `parcels`, unless the same layout
    /// is already up. The calls into the islands manager are the plan's, made outside this bind.
    fn plan_islands(
        &mut self,
        parcels: Vec<IVec2>,
        empty_center: Option<IVec2>,
    ) -> Option<IslandPlan> {
        let group_hash = hash_parcels(&parcels);
        if self.last_scene_group_hash != 0 && self.last_scene_group_hash == group_hash {
            return None;
        }
        self.last_scene_group_hash = group_hash;

        let layout = island_layout(&parcels, empty_center)?;
        if let Some(walls) = &mut self.wall_manager {
            let mut walls = walls.bind_mut();
            walls.clear_walls();
            walls.create_walls_for_bounds(
                layout.min.x,
                layout.max.x,
                layout.min.y,
                layout.max.y,
                PADDING,
            );
        }

        self.last_candidate_parcels = layout
            .candidates
            .iter()
            .map(|c| Vector2i::new(c.x, c.y))
            .collect();
        self.candidate_set = layout.candidates.iter().map(|c| (c.x, c.y)).collect();
        self.pending_empty_parcel_spawn = match empty_center {
            Some(center) => Vector2i::new(center.x, center.y),
            None => INVALID_PARCEL,
        };
        Some(IslandPlan {
            islands: self.islands_manager.clone()?,
            current: self.current_position,
            candidates: Array::from(self.last_candidate_parcels.as_slice()),
            corners: PackedByteArray::from(layout.corners.as_slice()),
        })
    }

    fn clear_floating_islands_state(&mut self) {
        if let Some(islands) = &mut self.islands_manager {
            islands.bind_mut().clear_all();
        }
        self.candidate_set.clear();
        self.last_candidate_parcels.clear();
        self.pending_empty_parcel_spawn = INVALID_PARCEL;
        if let Some(walls) = &mut self.wall_manager {
            walls.bind_mut().clear_walls();
        }
    }

    /// Trusts the spawn point and lifts it out of any collider, rather than raycasting from
    /// above, which would drop the player on top of tall objects.
    fn find_valid_spawn_position(&self, spawn: Vector3) -> Vector3 {
        const MAX_LIFT_STEPS: usize = 50;
        let Some(tree) = self.base().get_tree_or_null() else {
            return spawn;
        };
        let Some(mut space) = tree
            .get_root()
            .and_then(|root| root.get_world_3d())
            .and_then(|world| world.get_direct_space_state())
        else {
            return spawn;
        };
        let mut shape = SphereShape3D::new_gd();
        shape.set_radius(0.3);
        let mut query = PhysicsShapeQueryParameters3D::new_gd();
        query.set_shape(&shape);
        // Layer 1 is scene geometry, layer 2 the empty-parcel terrain.
        query.set_collision_mask(3);
        query.set_collide_with_bodies(true);
        query.set_collide_with_areas(false);

        let mut position = spawn;
        for _ in 0..MAX_LIFT_STEPS {
            query.set_transform(Transform3D::new(Basis::IDENTITY, position));
            if space
                .intersect_shape_ex(&query)
                .max_results(1)
                .done()
                .is_empty()
            {
                return position;
            }
            position.y += 1.0;
        }
        spawn
    }

    fn find_gltf_containers(
        &self,
        scene_number_id: i32,
        lower_src: &str,
    ) -> Vec<Gd<DclGltfContainer>> {
        let mut found = Vec::new();
        let mut stack = vec![globals::scene_runner().upcast::<Node>()];
        while let Some(node) = stack.pop() {
            stack.extend(node.get_children().iter_shared());
            let Ok(container) = node.try_cast::<DclGltfContainer>() else {
                continue;
            };
            let matches = {
                let bound = container.bind();
                bound.get_dcl_scene_id() == scene_number_id
                    && bound.get_dcl_gltf_src().to_string().to_lowercase() == lower_src
            };
            if matches {
                found.push(container);
            }
        }
        found
    }

    fn uptime_ms() -> u64 {
        godot::classes::Time::singleton().get_ticks_msec()
    }
}

/// What lays the islands out: the calls into the runner and the manager, made with no fetcher
/// bind held because both emit signals whose handlers call back into the fetcher.
struct IslandPlan {
    islands: Gd<DclFloatingIslandsManager>,
    current: Vector2i,
    candidates: Array<Vector2i>,
    corners: PackedByteArray,
}

impl IslandPlan {
    fn apply(mut self) {
        scene_runner()
            .bind_mut()
            .start_floating_islands((self.corners.len() / 8) as i32);
        let mut islands = self.islands.bind_mut();
        islands.set_player_parcel(self.current);
        islands.set_candidate_parcels(self.candidates, self.corners);
    }
}

fn regenerate_floating_islands(this: &mut Gd<SceneFetcher>) {
    let plan = this.bind_mut().plan_regeneration();
    if let Some(plan) = plan {
        plan.apply();
    }
}

fn on_loading_finished(this: &mut Gd<SceneFetcher>) {
    let position = this.bind_mut().spawn_after_loading();
    if let Some(position) = position {
        move_player_to(position);
    }
}

fn on_islands_generation_complete(this: &mut Gd<SceneFetcher>) {
    scene_runner().bind_mut().finish_floating_islands();
    let pending = {
        let mut fetcher = this.bind_mut();
        let pending = fetcher.pending_empty_parcel_spawn;
        fetcher.pending_empty_parcel_spawn = INVALID_PARCEL;
        pending
    };
    if pending != INVALID_PARCEL {
        spawn_local(spawn_on_empty_parcel(this.clone(), pending));
    }
}

/// A preview update for a whole scene: reload it without the loading screen.
async fn hot_reload(mut this: Gd<SceneFetcher>, scene_id: String) {
    hot_reload_flagged(&mut this, &scene_id);
}

fn hot_reload_flagged(this: &mut Gd<SceneFetcher>, scene_id: &str) {
    this.bind_mut().is_hot_reloading = true;
    reload_scene(this, scene_id);
}

fn reload_scene(this: &mut Gd<SceneFetcher>, scene_id: &str) {
    let (present, hot) = {
        let fetcher = this.bind();
        (
            fetcher.loaded_scenes.contains_key(scene_id),
            fetcher.is_hot_reloading,
        )
    };
    if !present {
        return;
    }
    // Up front, so killing the scene does not flash an empty parcel before it re-loads. A
    // preview hot reload is meant to be seamless.
    if !hot {
        if let Some(loading) = explorer_loading() {
            loading.bind().begin("".into(), "on_reload".into(), true);
        }
    }
    let mut fetcher = this.bind_mut();
    let Some(scene) = fetcher.loaded_scenes.remove(scene_id) else {
        return;
    };
    if scene.is_running() {
        kill_scene(scene.scene_number_id);
        if let Some(floors) = &mut fetcher.base_floor_manager {
            floors.bind_mut().remove_scene_floors(scene_id);
        }
    }
    let mapping = scene.definition.bind().get_content_mapping();
    let mut provider = globals::content_provider();
    for file in mapping.bind().get_files().to_vec() {
        let hash = mapping.bind().get_hash(file);
        provider.bind_mut().purge_file(hash);
    }
    fetcher
        .coordinator
        .bind_mut()
        .reload_scene_data(scene_id.into());
    fetcher.is_reloading = true;
}

fn hash_parcels(parcels: &[IVec2]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    parcels.iter().for_each(|p| (p.x, p.y).hash(&mut hasher));
    // 0 means "no layout yet".
    hasher.finish().max(1)
}

fn desired_scene_ids(coordinator: &Gd<SceneEntityCoordinator>, key: &str) -> Vec<String> {
    coordinator
        .bind()
        .get_desired_scenes()
        .get(key)
        .and_then(|v| v.try_to::<VarArray>().ok())
        .map(|ids| {
            ids.iter_shared()
                .map(|id| id.to::<GString>().to_string())
                .collect()
        })
        .unwrap_or_default()
}

fn scene_runner() -> Gd<SceneManager> {
    globals::scene_runner()
}

fn kill_scene(scene_number_id: i32) {
    scene_runner().bind_mut().kill_scene(scene_number_id);
}

pub(super) fn explorer() -> Option<Gd<crate::world::explorer::Explorer>> {
    DclGlobal::get_explorer()
}

fn explorer_loading() -> Option<Gd<super::loading_session::LoadingSession>> {
    crate::world::state::loading()
}

fn move_player_to(position: Vector3) {
    // The spawn point is already validated, so skip the stuck check.
    crate::world::api::move_to(position, true, false);
}

async fn spawn_on_empty_parcel(this: Gd<SceneFetcher>, parcel: Vector2i) {
    // One more physics frame so the collision shape is ready.
    next_physics_frame().await;
    let center = Vector3::new(
        parcel.x as f32 * PARCEL_SIZE + PARCEL_HALF_SIZE,
        0.0,
        -parcel.y as f32 * PARCEL_SIZE - PARCEL_HALF_SIZE,
    );
    let position = this.bind().find_valid_spawn_position(center);
    move_player_to(position);
}

async fn on_preview_model_update(
    this: Gd<SceneFetcher>,
    scene_id: String,
    src: String,
    removed: bool,
) {
    // A single .glb changed: swap just that model in place. Anything else falls back to a
    // full loading-screen-free reload. Only .glb is swapped: a .gltf points at sibling
    // .bin/texture files that are not models, get no event of their own and may be missing
    // from the mapping, so swapping the .gltf alone fails or rebuilds from stale buffers.
    let lower_src = src.to_lowercase();
    let full_reload = |mut this: Gd<SceneFetcher>| {
        hot_reload_flagged(&mut this, &scene_id);
    };

    let scene_number_id = {
        let fetcher = this.bind();
        match fetcher.loaded_scenes.get(&scene_id) {
            Some(scene) if lower_src.ends_with(".glb") => scene.scene_number_id,
            _ => {
                drop(fetcher);
                full_reload(this);
                return;
            }
        }
    };
    let containers = this
        .bind()
        .find_gltf_containers(scene_number_id, &lower_src);
    if containers.is_empty() {
        // Not used by any live container: a component that has not loaded yet picks up the
        // full reload.
        full_reload(this);
        return;
    }

    // The message's hash is deliberately not written into the mapping: the file watcher hashes
    // the project-relative path, the scene definition the absolute one, so they never match.
    // A path-derived hash cannot change when the contents do, so only a deletion touches the
    // mapping.
    let mapping = scene_runner()
        .bind()
        .get_scene_content_mapping(scene_number_id);
    let old_hash = mapping
        .bind()
        .get_hash(lower_src.as_str().into())
        .to_string();

    if !removed && old_hash.is_empty() {
        let removed_at = this.bind().recently_removed_models.get(&lower_src).copied();
        if let Some(at) = removed_at {
            if SceneFetcher::uptime_ms().saturating_sub(at) < REMOVED_MODEL_GRACE_MS {
                // The delete's own trailing change event: the file is still gone.
                return;
            }
        }
        // The file is back after a delete dropped its entry, which cannot be rebuilt here (the
        // hash needs the server's absolute path and machine id): a full reload refetches the
        // scene definition.
        let mut this = this;
        this.bind_mut().recently_removed_models.remove(&lower_src);
        hot_reload_flagged(&mut this, &scene_id);
        return;
    }

    if removed {
        let updated = scene_runner().bind_mut().update_scene_content_entry(
            scene_number_id,
            lower_src.as_str().into(),
            "".into(),
        );
        if !updated {
            full_reload(this);
            return;
        }
        this.clone()
            .bind_mut()
            .recently_removed_models
            .insert(lower_src.clone(), SceneFetcher::uptime_ms());
    } else {
        this.clone()
            .bind_mut()
            .recently_removed_models
            .remove(&lower_src);
    }

    // Every cache layer keyed by the (stable) hash now holds stale bytes. Awaited, because the
    // re-request would otherwise race the delete and re-import the file just evicted.
    if !old_hash.is_empty() {
        let purge = globals::content_provider()
            .bind_mut()
            .purge_file(old_hash.as_str().into());
        await_promise(purge).await;
    }
    for mut container in containers {
        if container.is_instance_valid() {
            container.bind_mut().force_reload_gltf();
        }
    }
}

fn metrics_failed(position: Vector2i, scene_id: &str, error: &str, message: &str) {
    let mut data = serde_json::json!({
        "scene_id": scene_id,
        "position": format!("{},{}", position.x, position.y),
        "status": "Failed",
        "error": error,
    });
    if !message.is_empty() {
        data["error_message"] = message.into();
    }
    globals::metrics()
        .bind_mut()
        .track_screen_viewed("LOADING_END".to_string(), data.to_string());
}

fn promise_error(result: &Variant) -> Option<String> {
    result
        .try_to::<Gd<PromiseError>>()
        .ok()
        .map(|error| error.bind().get_error().to_string())
}

/// Loads the scenes the coordinator wants and unloads the ones it dropped, then lays out the
/// floating islands. Runs concurrently with itself; `scene_changed_counter` makes only the
/// newest run finish.
async fn async_on_desired_scene_changed(mut this: Gd<SceneFetcher>) {
    let coordinator = this.bind().coordinator.clone();
    let loadable = desired_scene_ids(&coordinator, "loadable_scenes");
    let mut runner = scene_runner();
    runner.bind_mut().loading_mark_discovery();

    let (counter, is_reloading_now, is_hot_reloading_now) = {
        let mut fetcher = this.bind_mut();
        fetcher.scene_changed_counter += 1;
        let flags = (fetcher.is_reloading, fetcher.is_hot_reloading);
        // Only consumed once there is something to load, so a coordinator that has not found
        // its scenes yet does not lose them.
        if !loadable.is_empty() {
            fetcher.is_reloading = false;
            fetcher.is_hot_reloading = false;
        }
        (fetcher.scene_changed_counter, flags.0, flags.1)
    };
    let superseded = |this: &Gd<SceneFetcher>| this.bind().scene_changed_counter != counter;

    let use_islands = this.bind().is_using_floating_islands();
    let dynamic = this.bind().use_dynamic_loading;

    // Show the loading screen when nothing is loaded yet, scenes are coming, islands are on and
    // this is not a hot reload; in dynamic mode only a teleport/reload makes the player wait.
    let mut new_loading = this.bind().loaded_scenes.is_empty()
        && !loadable.is_empty()
        && use_islands
        && (!dynamic || is_reloading_now)
        && !is_hot_reloading_now;

    let mut to_load = Vec::new();
    for scene_id in &loadable {
        let loaded = this
            .bind()
            .loaded_scenes
            .get(scene_id)
            .map(SceneItem::is_running);
        match loaded {
            Some(true) => new_loading = false,
            Some(false) => {}
            None => match coordinator
                .bind()
                .get_scene_definition(scene_id.as_str().into())
            {
                Some(definition) => to_load.push((scene_id.clone(), definition)),
                None => {
                    godot_error!("should load scene_id {scene_id} but data is empty");
                    // A null definition still completes the loading session.
                    runner
                        .bind_mut()
                        .report_scene_fetched(scene_id.as_str().into());
                }
            },
        }
    }

    let mut loading_session_started = false;
    if !to_load.is_empty() && new_loading {
        let ids: Vec<GString> = to_load.iter().map(|(id, _)| id.as_str().into()).collect();
        runner
            .bind_mut()
            .start_loading_session(PackedStringArray::from(ids.as_slice()));
        loading_session_started = true;
    }

    this.bind_mut().base_mut().emit_signal(
        "report_scene_load",
        &[
            false.to_variant(),
            new_loading.to_variant(),
            (loadable.len() as i32).to_variant(),
        ],
    );

    futures_util::future::join_all(
        to_load
            .into_iter()
            .map(|(id, definition)| load_scene(this.clone(), id, definition)),
    )
    .await;

    // The last call to finish does the rest.
    if superseded(&this) {
        return;
    }

    let desired = coordinator.bind().get_desired_scenes();
    let keep = |key: &str| -> Vec<String> {
        desired
            .get(key)
            .and_then(|v| v.try_to::<VarArray>().ok())
            .map(|ids| {
                ids.iter_shared()
                    .map(|id| id.to::<GString>().to_string())
                    .collect()
            })
            .unwrap_or_default()
    };
    let (current_loadable, keep_alive) = (keep("loadable_scenes"), keep("keep_alive_scenes"));
    {
        // Scenes still loading and global scenes stay.
        let mut fetcher = this.bind_mut();
        let stale: Vec<String> = fetcher
            .loaded_scenes
            .keys()
            .filter(|id| !current_loadable.contains(id) && !keep_alive.contains(id))
            .cloned()
            .collect();
        let killed = fetcher.kill_loaded_scenes(|scene| stale.contains(&scene.id));
        for id in killed {
            fetcher.loaded_scenes.remove(&id);
        }
    }

    // Dynamic loading uses simple base floors instead of terrain.
    let use_floating_islands = use_islands && !dynamic;
    if !use_floating_islands && this.bind().last_scene_group_hash != 0 {
        let mut fetcher = this.bind_mut();
        fetcher.last_scene_group_hash = 0;
        fetcher.clear_floating_islands_state();
    }

    if use_floating_islands {
        // The occupied parcels must all be known before the terrain is generated.
        let mut waited = 0;
        let mut coordinator_was_busy = coordinator.bind().is_busy();
        while coordinator.bind().is_busy() && waited < MAX_COORDINATOR_WAIT_FRAMES {
            coordinator_was_busy = true;
            next_frame().await;
            waited += 1;
            if superseded(&this) {
                return;
            }
        }
        // A realm that just changed may not have queued its requests yet: give it a moment to
        // start, then wait for it to finish.
        if !coordinator_was_busy && is_reloading_now {
            let mut startup = 0;
            while !coordinator.bind().is_busy() && startup < COORDINATOR_START_WAIT_FRAMES {
                next_frame().await;
                startup += 1;
                if superseded(&this) {
                    return;
                }
            }
            while coordinator.bind().is_busy() && waited < MAX_COORDINATOR_WAIT_FRAMES {
                coordinator_was_busy = true;
                next_frame().await;
                waited += 1;
                if superseded(&this) {
                    return;
                }
            }
        }
        if waited >= MAX_COORDINATOR_WAIT_FRAMES {
            godot_warn!("Timed out waiting for scene_entity_coordinator to finish fetching scenes");
        }

        // Scenes found while waiting.
        let mut late = Vec::new();
        for scene_id in desired_scene_ids(&coordinator, "loadable_scenes") {
            if this.bind().loaded_scenes.contains_key(&scene_id) {
                continue;
            }
            if let Some(definition) = coordinator
                .bind()
                .get_scene_definition(scene_id.as_str().into())
            {
                late.push((scene_id, definition));
            }
        }
        if !late.is_empty() {
            let ids: Vec<GString> = late.iter().map(|(id, _)| id.as_str().into()).collect();
            runner
                .bind_mut()
                .start_loading_session(PackedStringArray::from(ids.as_slice()));
            loading_session_started = true;
            futures_util::future::join_all(
                late.into_iter()
                    .map(|(id, definition)| load_scene(this.clone(), id, definition)),
            )
            .await;
            if superseded(&this) {
                return;
            }
        }

        regenerate_floating_islands(&mut this);

        // No session started in islands mode (e.g. a teleport to an empty parcel): end the
        // loading screen now, unless a realm change is still on its way.
        if !loading_session_started
            && !runner.bind().has_active_loading_session()
            && (coordinator_was_busy || !is_reloading_now)
        {
            runner.emit_signal("loading_complete", &[(-1_i64).to_variant()]);
        }
    }

    let (filled, empty) = {
        let fetcher = this.bind();
        let filled: Vec<Vector2i> = fetcher
            .loaded_scenes
            .values()
            .flat_map(|scene| scene.parcels.iter().copied())
            .collect();
        // Test/renderer modes draw no empty parcels, so no grass appears in snapshots.
        let empty = if use_floating_islands && fetcher.last_scene_group_hash != 0 {
            fetcher.last_candidate_parcels.clone()
        } else {
            Vec::new()
        };
        (filled, empty)
    };
    let to_array =
        |parcels: Vec<Vector2i>| -> VarArray { parcels.iter().map(|p| p.to_variant()).collect() };
    let mut fetcher = this.bind_mut();
    fetcher.base_mut().emit_signal(
        "report_scene_load",
        &[
            true.to_variant(),
            false.to_variant(),
            (loadable.len() as i32).to_variant(),
        ],
    );
    fetcher.base_mut().emit_signal(
        "parcels_processed",
        &[to_array(filled).to_variant(), to_array(empty).to_variant()],
    );
}

/// Fetches a scene's code, crdt and optimized-assets manifest, then spawns it.
async fn load_scene(
    mut this: Gd<SceneFetcher>,
    scene_id: String,
    definition: Gd<DclSceneEntityDefinition>,
) -> bool {
    let (parcels, is_global) = {
        let def = definition.bind();
        (
            def.get_parcels().iter_shared().collect::<Vec<_>>(),
            def.is_global(),
        )
    };
    {
        let mut fetcher = this.bind_mut();
        if let Some(existing) = fetcher.loaded_scenes.get(&scene_id) {
            godot_warn!(
                "Scene already in loaded_scenes! scene_number_id: {}",
                existing.scene_number_id
            );
        }
        if fetcher.is_using_floating_islands()
            && !fetcher.use_dynamic_loading
            && fetcher.last_scene_group_hash != 0
            && !is_global
        {
            godot_warn!(
"Scene {scene_id} discovered after floating islands were generated. Islands will be regenerated."
            );
        }
        fetcher.loaded_scenes.insert(
            scene_id.clone(),
            SceneItem {
                id: scene_id.clone(),
                definition: definition.clone(),
                scene_number_id: NO_SCENE,
                parcels,
                is_global,
            },
        );
    }

    let global = DclGlobal::singleton();
    let mut provider = globals::content_provider();
    let mapping = definition.bind().get_content_mapping();

    if std::mem::take(&mut this.bind_mut().purge_cache_on_first_load) {
        let purges: Vec<_> = mapping
            .bind()
            .get_files()
            .to_vec()
            .into_iter()
            .map(|file| {
                let hash = mapping.bind().get_hash(file);
                provider.bind_mut().purge_file(hash)
            })
            .collect();
        for purge in purges {
            await_promise(purge).await;
        }
    }

    // The optimized manifest (`{entity}-optimized.json`, a plain file) and the scene boot files
    // (`{hash}.js`, `{hash}.crdt`) live in the optimized bucket; a 404 there only means "not
    // optimized" and the content server has the file.
    let (skip_optimized, only_optimized) = {
        let cli = globals::cli();
        let cli = cli.bind();
        (
            global.bind().testing_scene_mode || cli.only_no_optimized,
            cli.only_optimized,
        )
    };
    let opt_base = provider.bind().get_optimized_base_url().to_string();
    let cache_path = |provider: &Gd<crate::content::content_provider::ContentProvider>,
                      hash: &str| {
        provider.bind().get_cache_file_path(hash.into()).to_string()
    };
    let file_exists = |path: &str| FileAccess::file_exists(path);
    let is_sdk7 = definition.bind().is_sdk7();
    let main_js_hash = definition.bind().get_main_js_hash().to_string();
    let main_crdt_hash = definition.bind().get_main_crdt_hash().to_string();
    let manifest_name = format!("{scene_id}-optimized.json");
    let manifest_path = format!("user://content/{manifest_name}");

    // `{entity}-boot.zip` carries the manifest, main.js and main.crdt in one request, extracted
    // under the names the individual fetches would use, so those become disk hits. A 404 or any
    // failure just means the older layout.
    let boot_files_cached = file_exists(&manifest_path)
        && (!is_sdk7 || file_exists(&cache_path(&provider, &main_js_hash)))
        && (main_crdt_hash.is_empty() || file_exists(&cache_path(&provider, &main_crdt_hash)));
    let mut boot_bundled = false;
    if !skip_optimized && !boot_files_cached {
        let boot_zip = format!("{scene_id}-boot.zip");
        let promise = provider.bind_mut().fetch_boot_bundle(
            boot_zip.as_str().into(),
            gs(&format!("{opt_base}/{boot_zip}")),
        );
        let result = await_promise(promise).await;
        match promise_error(&result) {
            Some(error) => godot_error!("Scene {scene_id} boot bundle failed: {error}"),
            None => boot_bundled = result.try_to::<bool>().unwrap_or(false),
        }
    }
    let have_boot_files = boot_bundled || boot_files_cached;

    let local_main_js_path;
    let mut script_promise = None;
    let mut script_from_optimized = false;
    if is_sdk7 {
        let script_path = definition.bind().get_main_js_path();
        local_main_js_path = cache_path(&provider, &main_js_hash);
        if have_boot_files && file_exists(&local_main_js_path) {
            // Extracted from the boot bundle, or already cached.
        } else if !skip_optimized && !main_js_hash.is_empty() {
            script_promise = Some(provider.bind_mut().fetch_file_by_url(
                main_js_hash.as_str().into(),
                gs(&format!("{opt_base}/{main_js_hash}.js")),
            ));
            script_from_optimized = true;
        } else {
            script_promise = Some(provider.bind_mut().fetch_file(script_path, mapping.clone()));
        }
    } else {
        let name = "sdk-adaptation-layer.js";
        script_promise = Some(
            provider
                .bind_mut()
                .fetch_file_by_url(name.into(), ADAPTATION_LAYER_URL.into()),
        );
        local_main_js_path = format!("user://content/{name}");
    }

    let manifest_bundled = have_boot_files && file_exists(&manifest_path);
    let manifest_promise = (!skip_optimized && !manifest_bundled).then(|| {
        provider.bind_mut().fetch_file_by_url(
            manifest_name.as_str().into(),
            gs(&format!("{opt_base}/{manifest_name}")),
        )
    });

    let mut local_main_crdt_path = String::new();
    let mut crdt_promise = None;
    let mut crdt_from_optimized = false;
    if !main_crdt_hash.is_empty() {
        local_main_crdt_path = cache_path(&provider, &main_crdt_hash);
        if have_boot_files && file_exists(&local_main_crdt_path) {
            // Extracted from the boot bundle, or already cached.
        } else if !skip_optimized {
            crdt_promise = Some(provider.bind_mut().fetch_file_by_url(
                main_crdt_hash.as_str().into(),
                gs(&format!("{opt_base}/{main_crdt_hash}.crdt")),
            ));
            crdt_from_optimized = true;
        } else {
            crdt_promise = Some(
                provider
                    .bind_mut()
                    .fetch_file("main.crdt".into(), mapping.clone()),
            );
        }
    }

    let position = this.bind().current_position;
    let fail = |this: &Gd<SceneFetcher>, error: &str, message: &str| {
        // Reported as fetched (with the error) so the loading session can still progress.
        scene_runner()
            .bind_mut()
            .report_scene_fetched(scene_id.as_str().into());
        metrics_failed(position, &scene_id, error, message);
        let _ = this;
    };

    if let Some(promise) = script_promise {
        let mut result = await_promise(promise).await;
        if promise_error(&result).is_some() && script_from_optimized {
            let fallback = provider
                .bind_mut()
                .fetch_file(definition.bind().get_main_js_path(), mapping.clone());
            result = await_promise(fallback).await;
        }
        if let Some(error) = promise_error(&result) {
            godot_error!(
                "Scene {scene_id} fail getting the script code content, error message: {error}"
            );
            fail(&this, "script_fetch_failed", &error);
            return false;
        }
    }

    if let Some(promise) = crdt_promise {
        let mut result = await_promise(promise).await;
        if promise_error(&result).is_some() && crdt_from_optimized {
            let fallback = provider
                .bind_mut()
                .fetch_file("main.crdt".into(), mapping.clone());
            result = await_promise(fallback).await;
        }
        if let Some(error) = promise_error(&result) {
            godot_error!(
                "Scene {scene_id} fail getting the main crdt content, error message: {error}"
            );
            fail(&this, "crdt_fetch_failed", &error);
            return false;
        }
    }

    let mut manifest_ready = manifest_bundled;
    if let Some(promise) = manifest_promise {
        let result = await_promise(promise).await;
        if let Some(error) = promise_error(&result) {
            // A 404 is the expected "scene not optimized"; anything else is a download failure
            // worth a metric (the scene still loads, runtime-processed).
            if !error.contains("404") {
                godot_error!("Scene {scene_id} failed to download optimized manifest: {error}");
                metrics_failed(position, &scene_id, "manifest_download_failed", &error);
            }
            if only_optimized {
                godot_error!("Scene {scene_id} skipped (--only-optimized flag set)");
                scene_runner()
                    .bind_mut()
                    .report_scene_fetched(scene_id.as_str().into());
                this.bind_mut().loaded_scenes.remove(&scene_id);
                return false;
            }
        } else {
            manifest_ready = true;
        }
    }

    if manifest_ready {
        match FileAccess::open(&manifest_path, ModeFlags::READ) {
            Some(mut file) => {
                let json = file.get_as_text();
                file.close();
                let promise = provider.bind_mut().load_optimized_assets_metadata(json);
                await_promise(promise).await;
                godot_print!("Scene {scene_id} optimized assets metadata loaded successfully.");
            }
            None => {
                godot_error!("Scene {scene_id} failed to read optimized manifest");
                let error = godot::global::error_string(FileAccess::get_open_error().ord() as i64);
                metrics_failed(
                    position,
                    &scene_id,
                    "optimized_scene_json_load_failed",
                    &error.to_string(),
                );
            }
        }
    }

    // The scene was removed while it was loading.
    if !this.bind().loaded_scenes.contains_key(&scene_id) {
        godot_error!("Scene was removed while loading: {scene_id}");
        fail(&this, "scene_removed_while_loading", "");
        return false;
    }

    scene_runner()
        .bind_mut()
        .report_scene_fetched(scene_id.as_str().into());
    spawn_scene(
        &mut this,
        &scene_id,
        local_main_js_path,
        local_main_crdt_path,
    );
    true
}

/// Starts the scene's runtime. The runner is called with no fetcher bind held: it announces
/// the spawn to handlers that call back into the fetcher.
fn spawn_scene(this: &mut Gd<SceneFetcher>, scene_id: &str, main_js: String, main_crdt: String) {
    let Some(spawn) = this.bind_mut().prepare_spawn(scene_id, main_js, main_crdt) else {
        return;
    };
    let scene_number_id = scene_runner().bind_mut().start_scene(
        spawn.main_js.as_str().into(),
        spawn.main_crdt.as_str().into(),
        spawn.definition,
        spawn.enable_inspector,
    );
    this.bind_mut()
        .finish_spawn(scene_id, scene_number_id, &spawn.parcels);
}

struct SpawnArgs {
    main_js: String,
    main_crdt: String,
    definition: Gd<DclSceneEntityDefinition>,
    parcels: Vec<Vector2i>,
    enable_inspector: bool,
}

impl SceneFetcher {
    fn prepare_spawn(
        &mut self,
        scene_id: &str,
        mut main_js: String,
        mut main_crdt: String,
    ) -> Option<SpawnArgs> {
        // Between explorer teardown (sign-out, realm change) and the next explorer there is
        // nothing to spawn into: drop the entry so the next position scan re-fetches it.
        let global = DclGlobal::singleton();
        if explorer().is_none() {
            godot_error!("Skipping scene spawn, no explorer: {scene_id}");
            self.loaded_scenes.remove(scene_id);
            return None;
        }
        if !main_js.is_empty() && !FileAccess::file_exists(&main_js) {
            godot_error!("Couldn't get main.js file: {main_js}");
            main_js.clear();
        }
        if !main_crdt.is_empty() && !FileAccess::file_exists(&main_crdt) {
            godot_error!("Couldn't get main.crdt file: {main_crdt}");
            main_crdt.clear();
        }
        if main_js.is_empty() && main_crdt.is_empty() {
            godot_error!("Couldn't spawn the scene (no js/crdt): {scene_id}");
            return None;
        }
        let scene = self.loaded_scenes.get(scene_id)?;

        // Attach the JS inspector to the debugged scene, or to the one whose title matches
        // `--inspect-scene-title` / the `inspect-scene-title=<title>` deeplink param (the
        // deeplink wins), so a benchmark launch can pick the isolate without touching the UI.
        let mut enable_inspector = false;
        if global.bind().has_javascript_debugger {
            let title = self.inspect_scene_title();
            enable_inspector = self.debugging_js_scene_id == scene_id
                || (!title.is_empty() && scene.definition.bind().get_title().to_string() == title);
        }
        Some(SpawnArgs {
            main_js,
            main_crdt,
            definition: scene.definition.clone(),
            parcels: scene.parcels.clone(),
            enable_inspector,
        })
    }

    fn finish_spawn(&mut self, scene_id: &str, scene_number_id: i32, parcels: &[Vector2i]) {
        if let Some(scene) = self.loaded_scenes.get_mut(scene_id) {
            scene.scene_number_id = scene_number_id;
        }
        if let Some(floors) = &mut self.base_floor_manager {
            floors.bind_mut().add_scene_floors(scene_id, parcels);
        }
        if self.is_using_floating_islands() && !self.use_dynamic_loading {
            self.request_regenerate_floating_islands();
        }
    }

    fn inspect_scene_title(&self) -> String {
        let global = DclGlobal::singleton();
        let from_link = global
            .get("deep_link_obj")
            .try_to::<Gd<Object>>()
            .ok()
            .and_then(|link| {
                link.get("params")
                    .try_to::<VarDictionary>()
                    .ok()?
                    .get("inspect-scene-title")
                    .map(|v| v.to::<GString>().to_string())
            })
            .unwrap_or_default();
        if from_link.is_empty() {
            globals::cli().bind().inspect_scene_title.to_string()
        } else {
            from_link
        }
    }
}
