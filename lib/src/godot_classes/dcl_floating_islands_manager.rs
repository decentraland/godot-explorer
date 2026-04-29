use std::collections::{HashMap, HashSet};
use std::sync::mpsc;
use std::thread;

use godot::builtin::{
    Array, Color, PackedByteArray, PackedColorArray, PackedFloat32Array, PackedInt32Array,
    PackedVector2Array, PackedVector3Array, Rid, Transform3D, VarArray, VarDictionary, Vector2,
    Vector2i, Vector3,
};
use godot::classes::mesh::ArrayType;
use godot::classes::physics_server_3d::{BodyMode, BodyState};
use godot::classes::rendering_server::{
    MultimeshTransformFormat, PrimitiveType as RsPrimitiveType,
};
use godot::classes::{
    Camera3D, INode, Material, Mesh, Node, PhysicsServer3D, RenderingServer, ResourceLoader,
};
use godot::global::{godot_error, godot_warn};
use godot::obj::{Base, Gd};
use godot::prelude::*;

use crate::godot_classes::floating_islands::{
    self, cliffs, props, props_pool, terrain, CornerConfig, ParcelData, PendingPhysicsGeometry,
    SimpleRng, CLIFF_MATERIAL_PATH, GRASS_BASE_SCALE, GRASS_BLADES_MATERIAL_PATH,
    GRASS_BLADE_MESH_PATH, GRASS_CULLING_RANGE, OBSTACLE_LAYER, OVERHANG_MATERIAL_PATH,
    PARCEL_HALF_SIZE, PARCEL_HEIGHT_BOUND, PARCEL_SIZE, TERRAIN_MATERIAL_PATH,
};

#[derive(GodotClass)]
#[class(base=Node)]
pub struct DclFloatingIslandsManager {
    base: Base<Node>,

    candidates: HashMap<(i32, i32), CornerConfig>,
    active: HashMap<(i32, i32), ParcelData>,

    player_parcel: Vector2i,
    view_distance: i32,
    destroy_hysteresis: i32,
    /// Applied separately as a creates-per-frame cap AND a destroys-per-frame cap (not combined).
    frame_budget: i32,

    generating: bool,
    generated_so_far: i32,
    generation_total: i32,

    scenario: Rid,
    physics_space: Rid,
    terrain_material: Option<Gd<Material>>,
    cliff_material: Option<Gd<Material>>,
    overhang_material: Option<Gd<Material>>,
    grass_blade_mesh: Option<Gd<Mesh>>,
    grass_material: Option<Gd<Material>>,
    grass_blade_mesh_rid: Rid,
    grass_material_rid: Rid,
    prop_cache: props::PropCache,
    prop_pool: props_pool::PropPoolManager,

    worker: Option<WorkerHandle>,
    pending: HashSet<(i32, i32)>,
}

enum WorkerMsg {
    Build(WorkerPayload),
}

struct WorkerPayload {
    coord: (i32, i32),
    config: CornerConfig,
}

struct BuiltCliffSide {
    cliff: cliffs::CliffMeshData,
    overhang: cliffs::OverhangMeshData,
}

struct BuiltParcelMeshes {
    coord: (i32, i32),
    config: CornerConfig,
    terrain: terrain::TerrainMeshData,
    sides: Vec<BuiltCliffSide>,
}

struct WorkerHandle {
    tx: mpsc::Sender<WorkerMsg>,
    rx: mpsc::Receiver<BuiltParcelMeshes>,
}

#[godot_api]
impl INode for DclFloatingIslandsManager {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            candidates: HashMap::new(),
            active: HashMap::new(),
            player_parcel: Vector2i::new(0, 0),
            view_distance: 7,
            destroy_hysteresis: 2,
            frame_budget: 4,
            generating: false,
            generated_so_far: 0,
            generation_total: 0,
            scenario: Rid::Invalid,
            physics_space: Rid::Invalid,
            terrain_material: None,
            cliff_material: None,
            overhang_material: None,
            grass_blade_mesh: None,
            grass_material: None,
            grass_blade_mesh_rid: Rid::Invalid,
            grass_material_rid: Rid::Invalid,
            prop_cache: props::PropCache::default(),
            prop_pool: props_pool::PropPoolManager::default(),
            worker: None,
            pending: HashSet::new(),
        }
    }

    fn process(&mut self, _delta: f64) {
        self.tick_culling();
    }

    fn exit_tree(&mut self) {
        // The node is being removed; release every Godot RID we own so they
        // don't outlive the manager. `clear_all` also drains the worker queue
        // and clears the prop pool; dropping the manager later closes the
        // mpsc channel, which lets the worker thread exit on its next `recv`.
        self.clear_all();
        self.scenario = Rid::Invalid;
        self.physics_space = Rid::Invalid;
        self.grass_blade_mesh_rid = Rid::Invalid;
        self.grass_material_rid = Rid::Invalid;
    }
}

#[godot_api]
impl DclFloatingIslandsManager {
    #[signal]
    fn generation_progress(created: i32, total: i32);

    #[signal]
    fn generation_complete();

    /// `corner_configs`: `parcels.len() * 8` bytes, laid out per parcel as
    /// `[N, S, E, W, NW, NE, SW, SE]` with values 0=EMPTY, 1=NOTHING, 2=LOADED
    /// (matches GDScript `CornerConfiguration.ParcelState`).
    #[func]
    pub fn set_candidate_parcels(
        &mut self,
        parcels: Array<Vector2i>,
        corner_configs: PackedByteArray,
    ) {
        let expected_len = parcels.len() * 8;
        if corner_configs.len() != expected_len {
            godot_error!(
                "[DclFloatingIslandsManager] set_candidate_parcels: corner_configs length {} \
                 does not match parcels count {} * 8 = {}",
                corner_configs.len(),
                parcels.len(),
                expected_len
            );
            return;
        }

        self.candidates.clear();
        self.candidates.reserve(parcels.len());

        // Latch within a single call so a fully-malformed buffer doesn't flood
        // the console with thousands of duplicate warnings; resets across calls
        // so a later batch with new bad bytes can still surface once.
        let mut warned_invalid = false;
        for (i, coord) in parcels.iter_shared().enumerate() {
            let Some(cfg) = CornerConfig::from_packed(&corner_configs, i * 8) else {
                if !warned_invalid {
                    warned_invalid = true;
                    godot_warn!(
                        "[DclFloatingIslandsManager] invalid ParcelState byte in corner_configs \
                         near parcel index {i}, skipping (further warnings suppressed)"
                    );
                }
                continue;
            };
            self.candidates.insert((coord.x, coord.y), cfg);
        }

        // Stale active parcels whose corner configuration changed must be
        // rebuilt — otherwise we'd be left with cliff/overhang geometry that
        // no longer matches the surrounding scenes (T-junctions, overhangs
        // pointing at a now-loaded neighbor, etc.). Drop them now and let
        // `tick_culling` re-enqueue with the fresh config.
        let stale_actives: Vec<(i32, i32)> = self
            .active
            .iter()
            .filter_map(|(coord, data)| match self.candidates.get(coord) {
                Some(new_cfg) if new_cfg != &data.config => Some(*coord),
                _ => None,
            })
            .collect();
        for coord in stale_actives {
            if let Some(data) = self.active.remove(&coord) {
                self.destroy_parcel_data(data);
            }
        }

        self.generating = true;
        self.generated_so_far = 0;
        self.generation_total = self.candidates.len() as i32;
    }

    #[func]
    pub fn set_player_parcel(&mut self, parcel: Vector2i) {
        self.player_parcel = parcel;
    }

    #[func]
    pub fn set_view_distance(&mut self, radius: i32) {
        self.view_distance = radius.max(0);
    }

    #[func]
    pub fn set_destroy_hysteresis(&mut self, ring: i32) {
        self.destroy_hysteresis = ring.max(0);
    }

    #[func]
    pub fn set_frame_budget(&mut self, budget: i32) {
        self.frame_budget = budget.max(1);
    }

    #[func]
    pub fn clear_all(&mut self) {
        let drained: Vec<ParcelData> = self.active.drain().map(|(_, data)| data).collect();
        for data in drained {
            self.destroy_parcel_data(data);
        }
        self.prop_pool.clear();
        self.candidates.clear();
        self.pending.clear();
        self.generating = false;
        self.generated_so_far = 0;
        self.generation_total = 0;
        if let Some(worker) = &self.worker {
            while worker.rx.try_recv().is_ok() {}
        }
    }

    #[func]
    pub fn get_active_parcel_count(&self) -> i32 {
        self.active.len() as i32
    }

    #[func]
    pub fn get_candidate_count(&self) -> i32 {
        self.candidates.len() as i32
    }

    fn get_main_camera(&self) -> Option<Gd<Camera3D>> {
        self.base().get_viewport()?.get_camera_3d()
    }

    fn ensure_world_resources(&mut self) -> Option<()> {
        if !self.scenario.is_valid() || !self.physics_space.is_valid() {
            let viewport = self.base().get_viewport()?;
            let world = viewport.find_world_3d()?;
            self.scenario = world.get_scenario();
            self.physics_space = world.get_space();
        }
        if self.terrain_material.is_none() {
            self.terrain_material = Self::load_material(TERRAIN_MATERIAL_PATH);
        }
        if self.cliff_material.is_none() {
            self.cliff_material = Self::load_material(CLIFF_MATERIAL_PATH);
        }
        if self.overhang_material.is_none() {
            self.overhang_material = Self::load_material(OVERHANG_MATERIAL_PATH);
        }
        if self.grass_material.is_none() {
            self.grass_material = Self::load_material(GRASS_BLADES_MATERIAL_PATH);
            self.grass_material_rid = self
                .grass_material
                .as_ref()
                .map(|m| m.get_rid())
                .unwrap_or(Rid::Invalid);
        }
        if self.grass_blade_mesh.is_none() {
            let mut loader = ResourceLoader::singleton();
            if let Some(resource) = loader.load(GRASS_BLADE_MESH_PATH) {
                if let Ok(mesh) = resource.try_cast::<Mesh>() {
                    self.grass_blade_mesh_rid = mesh.get_rid();
                    self.grass_blade_mesh = Some(mesh);
                }
            }
        }
        if !self.prop_cache.is_populated() {
            if let Some(tree) = self.base().get_tree() {
                if let Some(cache) = props::PropCache::load_from_autoload(&tree) {
                    self.prop_cache = cache;
                }
            }
        }
        if self.worker.is_none() {
            self.worker = Some(spawn_worker());
        }
        if self.scenario.is_valid() && self.physics_space.is_valid() {
            Some(())
        } else {
            None
        }
    }

    fn tick_culling(&mut self) {
        let Some(camera) = self.get_main_camera() else {
            return;
        };
        if self.ensure_world_resources().is_none() {
            return;
        }

        let player = self.player_parcel;
        let view = self.view_distance;
        let hyst = self.destroy_hysteresis;
        let budget = self.frame_budget.max(1) as usize;

        // Drain before the visibility pass so freshly-submitted coords aren't re-enqueued.
        let submitted_this_frame = self.drain_worker_responses(budget);
        self.generated_so_far += submitted_this_frame;

        let mut in_view_candidates = 0;
        let mut in_view_missing: Vec<(i32, i32)> = Vec::new();

        for dx in -view..=view {
            for dz in -view..=view {
                let coord = (player.x + dx, player.y + dz);
                if !self.candidates.contains_key(&coord) {
                    continue;
                }
                let dist = dx.abs().max(dz.abs());
                let wanted = dist <= 1 || Self::parcel_in_camera_view(&camera, coord);
                if !wanted {
                    continue;
                }
                in_view_candidates += 1;
                if !self.active.contains_key(&coord) && !self.pending.contains(&coord) {
                    in_view_missing.push(coord);
                }
            }
        }

        let mut enqueued_this_frame = 0;
        for coord in in_view_missing.into_iter().take(budget) {
            self.enqueue_parcel_build(coord);
            enqueued_this_frame += 1;
        }
        let created_this_frame = submitted_this_frame + enqueued_this_frame;

        let keep_radius = hyst.max(1);
        let now_msec = godot::classes::Time::singleton().get_ticks_msec();

        let mut to_show: Vec<(i32, i32)> = Vec::new();
        let mut to_hide: Vec<(i32, i32)> = Vec::new();
        let mut to_destroy_immediate: Vec<(i32, i32)> = Vec::new();
        let mut to_destroy_stale: Vec<(i32, i32)> = Vec::new();

        for (&coord, data) in &self.active {
            let (x, z) = coord;
            let dist = (x - player.x).abs().max((z - player.y).abs());

            if dist > view + hyst {
                to_destroy_immediate.push(coord);
                continue;
            }

            let stale = data.stale_since_msec.is_some();

            if dist <= keep_radius {
                if stale {
                    to_show.push(coord);
                }
                continue;
            }

            let in_frustum = Self::parcel_in_camera_view(&camera, coord);
            if in_frustum {
                if stale {
                    to_show.push(coord);
                }
            } else if let Some(since) = data.stale_since_msec {
                if now_msec.saturating_sub(since) > STALE_DEADLINE_MSEC {
                    to_destroy_stale.push(coord);
                }
            } else {
                to_hide.push(coord);
            }
        }

        for coord in to_show {
            if let Some(data) = self.active.get_mut(&coord) {
                set_parcel_visible(data, true);
                data.stale_since_msec = None;
            }
        }
        for coord in to_hide {
            if let Some(data) = self.active.get_mut(&coord) {
                set_parcel_visible(data, false);
                data.stale_since_msec = Some(now_msec);
            }
        }
        for coord in to_destroy_immediate.into_iter().take(budget) {
            if let Some(data) = self.active.remove(&coord) {
                self.destroy_parcel_data(data);
            }
        }
        for coord in to_destroy_stale.into_iter().take(budget) {
            if let Some(data) = self.active.remove(&coord) {
                self.destroy_parcel_data(data);
            }
        }

        let mut to_promote_physics: Vec<(i32, i32)> = Vec::new();
        let mut to_demote_physics: Vec<(i32, i32)> = Vec::new();
        let mut to_promote_prop_physics: Vec<(i32, i32)> = Vec::new();
        let mut to_demote_prop_physics: Vec<(i32, i32)> = Vec::new();
        let mut to_promote_grass: Vec<(i32, i32)> = Vec::new();
        let mut to_demote_grass: Vec<(i32, i32)> = Vec::new();
        for (coord, data) in &self.active {
            let dist = (coord.0 - player.x).abs().max((coord.1 - player.y).abs());
            let has_physics = data.collision_body.is_valid();
            if dist <= PHYSICS_RANGE && !has_physics && data.pending_physics_geometry.is_some() {
                to_promote_physics.push(*coord);
            } else if dist > PHYSICS_RANGE && has_physics {
                to_demote_physics.push(*coord);
            }
            let prop_physics_wanted = dist <= PHYSICS_RANGE;
            let has_prop_physics = !data.prop_bodies.is_empty();
            if prop_physics_wanted && !has_prop_physics && !data.prop_physics_blueprints.is_empty()
            {
                to_promote_prop_physics.push(*coord);
            } else if !prop_physics_wanted && has_prop_physics {
                to_demote_prop_physics.push(*coord);
            }
            let should_have = Self::grass_should_be_visible(*coord, player);
            let has_grass = data.grass_instance.is_valid();
            if should_have && !has_grass {
                to_promote_grass.push(*coord);
            } else if !should_have && has_grass {
                to_demote_grass.push(*coord);
            }
        }
        let space = self.physics_space;
        for coord in to_promote_physics {
            if let Some(data) = self.active.get_mut(&coord) {
                if let Some(geom) = data.pending_physics_geometry.as_ref() {
                    let transform = parcel_world_transform(coord);
                    let faces = expand_indexed_faces(&geom.vertices, &geom.indices);
                    let (body, shape) = Self::build_terrain_collision(&faces, space, transform);
                    data.collision_body = body;
                    data.collision_shape = shape;
                }
            }
        }
        for coord in to_demote_physics {
            if let Some(data) = self.active.get_mut(&coord) {
                free_parcel_physics(data);
            }
        }
        for coord in to_promote_prop_physics {
            if let Some(data) = self.active.get_mut(&coord) {
                for blueprint in &data.prop_physics_blueprints {
                    let body = props::build_prop_body_from_blueprint(blueprint, space);
                    data.prop_bodies.push(body);
                }
            }
        }
        for coord in to_demote_prop_physics {
            if let Some(data) = self.active.get_mut(&coord) {
                free_parcel_prop_physics(data);
            }
        }
        let blade = self.grass_blade_mesh_rid;
        let mat = self.grass_material_rid;
        let scenario = self.scenario;
        if blade.is_valid() && mat.is_valid() {
            for coord in to_promote_grass {
                if let Some(data) = self.active.get_mut(&coord) {
                    let Some(geom) = data.pending_physics_geometry.as_ref() else {
                        continue;
                    };
                    let spawn_locations = terrain::derive_spawn_locations(
                        coord,
                        &data.config,
                        &geom.vertices,
                        &geom.indices,
                    );
                    let transform = parcel_world_transform(coord);
                    build_grass_for_parcel(
                        data,
                        coord,
                        &spawn_locations,
                        scenario,
                        transform,
                        blade,
                        mat,
                    );
                }
            }
        }
        for coord in to_demote_grass {
            if let Some(data) = self.active.get_mut(&coord) {
                destroy_parcel_grass(data);
            }
        }

        if created_this_frame > 0 {
            let created = self.generated_so_far;
            let total = self.generation_total;
            self.base_mut().emit_signal(
                "generation_progress",
                &[created.to_variant(), total.to_variant()],
            );
        }

        if self.generating
            && in_view_candidates > 0
            && self.in_view_all_materialized(player, view, &camera)
        {
            self.generating = false;
            self.base_mut().emit_signal("generation_complete", &[]);
        }
    }

    fn in_view_all_materialized(&self, player: Vector2i, view: i32, camera: &Gd<Camera3D>) -> bool {
        for dx in -view..=view {
            for dz in -view..=view {
                let coord = (player.x + dx, player.y + dz);
                if !self.candidates.contains_key(&coord) {
                    continue;
                }
                if !Self::parcel_in_camera_view(camera, coord) {
                    continue;
                }
                if !self.active.contains_key(&coord) {
                    return false;
                }
            }
        }
        true
    }

    fn parcel_in_camera_view(camera: &Gd<Camera3D>, coord: (i32, i32)) -> bool {
        let (cx, cz) = coord;
        let world_x = cx as f32 * PARCEL_SIZE + PARCEL_HALF_SIZE;
        let world_z = -(cz as f32 * PARCEL_SIZE + PARCEL_HALF_SIZE);

        let min_x = world_x - PARCEL_HALF_SIZE;
        let max_x = world_x + PARCEL_HALF_SIZE;
        let min_z = world_z - PARCEL_HALF_SIZE;
        let max_z = world_z + PARCEL_HALF_SIZE;

        let probes = [
            Vector3::new(world_x, 0.0, world_z),
            Vector3::new(min_x, 0.0, min_z),
            Vector3::new(max_x, 0.0, min_z),
            Vector3::new(min_x, 0.0, max_z),
            Vector3::new(max_x, 0.0, max_z),
            Vector3::new(world_x, PARCEL_HEIGHT_BOUND, world_z),
            Vector3::new(world_x, -PARCEL_HEIGHT_BOUND, world_z),
        ];

        probes.iter().any(|p| camera.is_position_in_frustum(*p))
    }

    fn enqueue_parcel_build(&mut self, coord: (i32, i32)) {
        if self.active.contains_key(&coord) || self.pending.contains(&coord) {
            return;
        }
        if self.ensure_world_resources().is_none() {
            return;
        }
        let Some(config) = self.candidates.get(&coord).copied() else {
            return;
        };
        let Some(worker) = &self.worker else {
            return;
        };
        if worker
            .tx
            .send(WorkerMsg::Build(WorkerPayload { coord, config }))
            .is_ok()
        {
            self.pending.insert(coord);
        }
    }

    fn drain_worker_responses(&mut self, budget: usize) -> i32 {
        let mut drained = 0;
        let mut submitted = 0;
        while drained < budget {
            let Some(worker) = self.worker.as_ref() else {
                break;
            };
            let Ok(built) = worker.rx.try_recv() else {
                break;
            };
            drained += 1;
            self.pending.remove(&built.coord);
            // The candidate set may have changed (or vanished) while this
            // build was in flight. Discard the result if it no longer applies;
            // `tick_culling` will re-enqueue with the current config. Stale
            // results still count against the budget so a burst of config
            // changes can't drain unbounded queued items in a single frame.
            let Some(current_cfg) = self.candidates.get(&built.coord).copied() else {
                continue;
            };
            if current_cfg != built.config {
                continue;
            }
            if self.active.contains_key(&built.coord) {
                continue;
            }
            self.submit_built_parcel(built);
            submitted += 1;
        }
        submitted
    }

    fn submit_built_parcel(&mut self, built: BuiltParcelMeshes) {
        let scenario = self.scenario;
        let space = self.physics_space;
        let BuiltParcelMeshes {
            coord,
            config,
            terrain,
            sides,
        } = built;
        let terrain::TerrainMeshData {
            vertices: terrain_vertices_vec,
            normals: terrain_normals_vec,
            uvs: terrain_uvs_vec,
            indices: terrain_indices_vec,
            spawn_locations,
        } = terrain;

        let world_x = coord.0 as f32 * PARCEL_SIZE + PARCEL_HALF_SIZE;
        let world_z = -(coord.1 as f32 * PARCEL_SIZE + PARCEL_HALF_SIZE);
        let transform = Transform3D::IDENTITY.translated(Vector3::new(world_x, 0.0, world_z));

        let mut rs = RenderingServer::singleton();

        let arrays = build_surface_arrays(
            &terrain_vertices_vec,
            &terrain_normals_vec,
            &terrain_uvs_vec,
            None,
            &terrain_indices_vec,
        );

        let mesh_rid = rs.mesh_create();
        rs.mesh_add_surface_from_arrays(mesh_rid, RsPrimitiveType::TRIANGLES, &arrays);

        let instance = rs.instance_create2(mesh_rid, scenario);
        rs.instance_set_transform(instance, transform);

        if let Some(material) = &self.terrain_material {
            rs.instance_geometry_set_material_override(instance, material.get_rid());
        }

        let dist_to_player = (coord.0 - self.player_parcel.x)
            .abs()
            .max((coord.1 - self.player_parcel.y).abs());
        let (collision_body, collision_shape) = if dist_to_player <= PHYSICS_RANGE {
            let collision_faces = expand_indexed_faces(&terrain_vertices_vec, &terrain_indices_vec);
            Self::build_terrain_collision(&collision_faces, space, transform)
        } else {
            (Rid::Invalid, Rid::Invalid)
        };

        let pending_physics_geometry = Some(PendingPhysicsGeometry {
            vertices: terrain_vertices_vec,
            indices: terrain_indices_vec,
        });

        let mut cliff_side_meshes: Vec<Rid> = Vec::with_capacity(sides.len());
        let mut cliff_side_instances: Vec<Rid> = Vec::with_capacity(sides.len());

        for built_side in &sides {
            let (mesh_rid, inst_rid) = self.spawn_cliff_side(scenario, transform, built_side);
            cliff_side_meshes.push(mesh_rid);
            cliff_side_instances.push(inst_rid);
        }

        let mut data = ParcelData {
            terrain_mesh: mesh_rid,
            terrain_instance: instance,
            collision_body,
            collision_shape,
            pending_physics_geometry,
            cliff_side_meshes,
            cliff_side_instances,
            config,
            ..ParcelData::default()
        };

        let player = self.player_parcel;
        if Self::grass_should_be_visible(coord, player) {
            let blade = self.grass_blade_mesh_rid;
            let mat = self.grass_material_rid;
            if blade.is_valid() && mat.is_valid() {
                build_grass_for_parcel(
                    &mut data,
                    coord,
                    &spawn_locations,
                    scenario,
                    transform,
                    blade,
                    mat,
                );
            }
        }

        self.spawn_parcel_props(
            coord,
            &config,
            &spawn_locations,
            &mut data,
            scenario,
            space,
            transform,
        );

        self.active.insert(coord, data);
    }

    #[allow(clippy::too_many_arguments)]
    fn spawn_parcel_props(
        &mut self,
        coord: (i32, i32),
        config: &CornerConfig,
        spawn_locations: &[floating_islands::SpawnLocation],
        data: &mut ParcelData,
        scenario: Rid,
        space: Rid,
        parcel_world: Transform3D,
    ) {
        if !self.prop_cache.is_populated() {
            return;
        }
        let include_physics = prop_physics_in_range(coord, self.player_parcel);
        let world_origin = parcel_world.origin;
        let mut rng = SimpleRng::new((coord.0 as u32 ^ 0xA53F, coord.1 as u32 ^ 0x91C2));
        let mut ctx = props::SpawnContext {
            scenario,
            space,
            parcel_world,
            parcel_world_origin: world_origin,
            include_physics,
            prop_slots: &mut data.prop_slots,
            prop_bodies: &mut data.prop_bodies,
            prop_blueprints: &mut data.prop_physics_blueprints,
            pool: &mut self.prop_pool,
        };
        props::spawn_rocks(&self.prop_cache, spawn_locations, &mut rng, &mut ctx);
        props::spawn_trees(
            &self.prop_cache,
            config,
            spawn_locations,
            &mut rng,
            &mut ctx,
        );
        props::spawn_generic_props(&self.prop_cache, spawn_locations, &mut rng, &mut ctx);
        props::spawn_cliff_rocks(&self.prop_cache, config, &mut rng, &mut ctx);
    }

    fn grass_should_be_visible(coord: (i32, i32), player: Vector2i) -> bool {
        let dist = (coord.0 - player.x).abs().max((coord.1 - player.y).abs());
        dist <= GRASS_CULLING_RANGE
    }

    fn spawn_cliff_side(
        &self,
        scenario: Rid,
        transform: Transform3D,
        side: &BuiltCliffSide,
    ) -> (Rid, Rid) {
        let mut rs = RenderingServer::singleton();
        let mesh_rid = rs.mesh_create();

        let cliff_arrays = build_surface_arrays(
            &side.cliff.vertices,
            &side.cliff.normals,
            &side.cliff.uvs,
            None,
            &side.cliff.indices,
        );
        rs.mesh_add_surface_from_arrays(mesh_rid, RsPrimitiveType::TRIANGLES, &cliff_arrays);

        let overhang_arrays = build_surface_arrays(
            &side.overhang.vertices,
            &side.overhang.normals,
            &side.overhang.uvs,
            Some(&side.overhang.colors),
            &side.overhang.indices,
        );
        rs.mesh_add_surface_from_arrays(mesh_rid, RsPrimitiveType::TRIANGLES, &overhang_arrays);

        let instance = rs.instance_create2(mesh_rid, scenario);
        rs.instance_set_transform(instance, transform);
        if let Some(material) = &self.cliff_material {
            rs.instance_set_surface_override_material(instance, 0, material.get_rid());
        }
        if let Some(material) = &self.overhang_material {
            rs.instance_set_surface_override_material(instance, 1, material.get_rid());
        }
        (mesh_rid, instance)
    }

    fn load_material(path: &str) -> Option<Gd<Material>> {
        let mut loader = ResourceLoader::singleton();
        let resource = loader.load(path)?;
        resource.try_cast::<Material>().ok()
    }

    fn build_terrain_collision(
        faces: &PackedVector3Array,
        space: Rid,
        transform: Transform3D,
    ) -> (Rid, Rid) {
        let mut physics = PhysicsServer3D::singleton();

        let shape = physics.concave_polygon_shape_create();
        let mut shape_data = VarDictionary::new();
        shape_data.set("faces", faces.to_variant());
        shape_data.set("backface_collision", false.to_variant());
        physics.shape_set_data(shape, &shape_data.to_variant());

        let body = physics.body_create();
        physics.body_set_mode(body, BodyMode::STATIC);
        physics.body_set_space(body, space);
        physics.body_add_shape(body, shape);
        physics.body_set_state(body, BodyState::TRANSFORM, &transform.to_variant());
        physics.body_set_collision_layer(body, OBSTACLE_LAYER);

        (body, shape)
    }

    fn destroy_parcel_data(&mut self, data: ParcelData) {
        let mut rs = RenderingServer::singleton();

        if data.terrain_instance.is_valid() {
            rs.free_rid(data.terrain_instance);
        }
        if data.terrain_mesh.is_valid() {
            rs.free_rid(data.terrain_mesh);
        }
        for rid in data.cliff_side_instances {
            if rid.is_valid() {
                rs.free_rid(rid);
            }
        }
        for rid in data.cliff_side_meshes {
            if rid.is_valid() {
                rs.free_rid(rid);
            }
        }
        if data.grass_instance.is_valid() {
            rs.free_rid(data.grass_instance);
        }
        if data.grass_multimesh.is_valid() {
            rs.free_rid(data.grass_multimesh);
        }
        for slot in data.prop_slots {
            self.prop_pool.release_slot(slot);
        }
        let mut physics = PhysicsServer3D::singleton();
        if data.collision_body.is_valid() {
            physics.free_rid(data.collision_body);
        }
        if data.collision_shape.is_valid() {
            physics.free_rid(data.collision_shape);
        }
        for rid in data.prop_bodies {
            if rid.is_valid() {
                physics.free_rid(rid);
            }
        }
    }
}

const STALE_DEADLINE_MSEC: u64 = 5000;

const PHYSICS_RANGE: i32 = 1;

fn prop_physics_in_range(coord: (i32, i32), player: Vector2i) -> bool {
    let dist = (coord.0 - player.x).abs().max((coord.1 - player.y).abs());
    dist <= PHYSICS_RANGE
}

fn spawn_worker() -> WorkerHandle {
    let (tx_req, rx_req) = mpsc::channel::<WorkerMsg>();
    let (tx_res, rx_res) = mpsc::channel::<BuiltParcelMeshes>();
    thread::Builder::new()
        .name("floating-islands-mesh-gen".into())
        .spawn(move || {
            let terrain_noise = terrain::build_terrain_noise();
            let cliff_noise = terrain::build_cliff_noise();
            loop {
                let Ok(WorkerMsg::Build(payload)) = rx_req.recv() else {
                    break;
                };
                let terrain = terrain::build_terrain_mesh(
                    payload.coord,
                    &payload.config,
                    &terrain_noise,
                    &cliff_noise,
                );
                let mut sides = Vec::new();
                for side in cliffs::nothing_sides(&payload.config) {
                    let cliff = cliffs::build_cliff_mesh(
                        &side,
                        payload.coord,
                        &payload.config,
                        &terrain_noise,
                        &cliff_noise,
                    );
                    let overhang = cliffs::build_overhang_mesh(
                        &side,
                        payload.coord,
                        &payload.config,
                        &terrain_noise,
                        &cliff_noise,
                    );
                    sides.push(BuiltCliffSide { cliff, overhang });
                }
                if tx_res
                    .send(BuiltParcelMeshes {
                        coord: payload.coord,
                        config: payload.config,
                        terrain,
                        sides,
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .expect("failed to spawn floating-islands mesh worker");
    WorkerHandle {
        tx: tx_req,
        rx: rx_res,
    }
}

fn parcel_world_transform(coord: (i32, i32)) -> Transform3D {
    let x = coord.0 as f32 * PARCEL_SIZE + PARCEL_HALF_SIZE;
    let z = -(coord.1 as f32 * PARCEL_SIZE + PARCEL_HALF_SIZE);
    Transform3D::IDENTITY.translated(Vector3::new(x, 0.0, z))
}

fn set_parcel_visible(data: &mut ParcelData, visible: bool) {
    let mut rs = RenderingServer::singleton();
    if data.terrain_instance.is_valid() {
        rs.instance_set_visible(data.terrain_instance, visible);
    }
    for rid in &data.cliff_side_instances {
        if rid.is_valid() {
            rs.instance_set_visible(*rid, visible);
        }
    }
    if data.grass_instance.is_valid() {
        rs.instance_set_visible(data.grass_instance, visible);
        data.grass_visible = visible;
    }
}

#[allow(clippy::too_many_arguments)]
fn build_grass_for_parcel(
    data: &mut ParcelData,
    coord: (i32, i32),
    spawn_locations: &[floating_islands::SpawnLocation],
    scenario: Rid,
    transform: Transform3D,
    blade_mesh_rid: Rid,
    grass_material_rid: Rid,
) {
    if spawn_locations.is_empty() {
        return;
    }
    let instance_count = spawn_locations.len() as i32;

    let mut rs = RenderingServer::singleton();
    let multimesh = rs.multimesh_create();
    rs.multimesh_allocate_data_ex(
        multimesh,
        instance_count,
        MultimeshTransformFormat::TRANSFORM_3D,
    )
    .done();
    rs.multimesh_set_mesh(multimesh, blade_mesh_rid);

    let mut rng = SimpleRng::new((coord.0 as u32, coord.1 as u32));
    let mut buffer = PackedFloat32Array::new();
    buffer.resize(spawn_locations.len() * 12);
    {
        let slice = buffer.as_mut_slice();
        for (i, loc) in spawn_locations.iter().enumerate() {
            let random_variation = 0.8 + rng.next_f32() * 0.4;
            let grass_scale_falloff = loc.falloff.powf(0.3);
            let final_scale = GRASS_BASE_SCALE * grass_scale_falloff * random_variation;
            let t = floating_islands::aligned_transform(
                loc.position,
                loc.normal,
                Some(&mut rng),
                final_scale,
            );
            write_transform_3d_row_major(&mut slice[i * 12..i * 12 + 12], &t);
        }
    }
    rs.multimesh_set_buffer(multimesh, &buffer);

    let instance = rs.instance_create2(multimesh, scenario);
    rs.instance_set_transform(instance, transform);
    rs.instance_geometry_set_material_override(instance, grass_material_rid);

    data.grass_multimesh = multimesh;
    data.grass_instance = instance;
    data.grass_visible = true;
}

fn free_parcel_physics(data: &mut ParcelData) {
    let mut physics = PhysicsServer3D::singleton();
    if data.collision_body.is_valid() {
        physics.free_rid(data.collision_body);
        data.collision_body = Rid::Invalid;
    }
    if data.collision_shape.is_valid() {
        physics.free_rid(data.collision_shape);
        data.collision_shape = Rid::Invalid;
    }
}

fn free_parcel_prop_physics(data: &mut ParcelData) {
    let mut physics = PhysicsServer3D::singleton();
    for body in data.prop_bodies.drain(..) {
        if body.is_valid() {
            physics.free_rid(body);
        }
    }
}

fn destroy_parcel_grass(data: &mut ParcelData) {
    let mut rs = RenderingServer::singleton();
    if data.grass_instance.is_valid() {
        rs.free_rid(data.grass_instance);
        data.grass_instance = Rid::Invalid;
    }
    if data.grass_multimesh.is_valid() {
        rs.free_rid(data.grass_multimesh);
        data.grass_multimesh = Rid::Invalid;
    }
    data.grass_visible = false;
}

/// Row-major `[basis_row.x, .y, .z, origin.component] × 3` — the layout
/// Godot's `multimesh_set_buffer` expects for TRANSFORM_3D.
fn write_transform_3d_row_major(slot: &mut [f32], t: &Transform3D) {
    let r0 = t.basis.rows[0];
    let r1 = t.basis.rows[1];
    let r2 = t.basis.rows[2];
    slot[0] = r0.x;
    slot[1] = r0.y;
    slot[2] = r0.z;
    slot[3] = t.origin.x;
    slot[4] = r1.x;
    slot[5] = r1.y;
    slot[6] = r1.z;
    slot[7] = t.origin.y;
    slot[8] = r2.x;
    slot[9] = r2.y;
    slot[10] = r2.z;
    slot[11] = t.origin.z;
}

fn build_surface_arrays(
    vertices: &[Vector3],
    normals: &[Vector3],
    uvs: &[Vector2],
    colors: Option<&[Color]>,
    indices: &[i32],
) -> VarArray {
    let mut arrays = VarArray::new();
    arrays.resize(ArrayType::MAX.ord() as usize, &Variant::nil());
    arrays.set(
        ArrayType::VERTEX.ord() as usize,
        &packed_vector3_from_slice(vertices).to_variant(),
    );
    arrays.set(
        ArrayType::NORMAL.ord() as usize,
        &packed_vector3_from_slice(normals).to_variant(),
    );
    arrays.set(
        ArrayType::TEX_UV.ord() as usize,
        &packed_vector2_from_slice(uvs).to_variant(),
    );
    if let Some(c) = colors {
        arrays.set(
            ArrayType::COLOR.ord() as usize,
            &packed_color_from_slice(c).to_variant(),
        );
    }
    arrays.set(
        ArrayType::INDEX.ord() as usize,
        &packed_int32_from_slice(indices).to_variant(),
    );
    arrays
}

fn expand_indexed_faces(vertices: &[Vector3], indices: &[i32]) -> PackedVector3Array {
    let mut faces = PackedVector3Array::new();
    faces.resize(indices.len());
    let slice = faces.as_mut_slice();
    for (i, idx) in indices.iter().enumerate() {
        slice[i] = vertices[*idx as usize];
    }
    faces
}

fn packed_vector3_from_slice(src: &[Vector3]) -> PackedVector3Array {
    let mut arr = PackedVector3Array::new();
    arr.resize(src.len());
    arr.as_mut_slice().copy_from_slice(src);
    arr
}

fn packed_vector2_from_slice(src: &[Vector2]) -> PackedVector2Array {
    let mut arr = PackedVector2Array::new();
    arr.resize(src.len());
    arr.as_mut_slice().copy_from_slice(src);
    arr
}

fn packed_int32_from_slice(src: &[i32]) -> PackedInt32Array {
    let mut arr = PackedInt32Array::new();
    arr.resize(src.len());
    arr.as_mut_slice().copy_from_slice(src);
    arr
}

fn packed_color_from_slice(src: &[Color]) -> PackedColorArray {
    let mut arr = PackedColorArray::new();
    arr.resize(src.len());
    arr.as_mut_slice().copy_from_slice(src);
    arr
}
