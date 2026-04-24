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
use godot::global::godot_error;
use godot::obj::{Base, Gd};
use godot::prelude::*;

use crate::godot_classes::floating_islands::{
    self, cliffs, props, terrain, CornerConfig, ParcelData, SimpleRng, CLIFF_MATERIAL_PATH,
    GRASS_BASE_SCALE, GRASS_BLADES_MATERIAL_PATH, GRASS_BLADE_MESH_PATH, GRASS_CULLING_RANGE,
    OVERHANG_MATERIAL_PATH, PARCEL_HALF_SIZE, PARCEL_HEIGHT_BOUND, PARCEL_SIZE,
    TERRAIN_MATERIAL_PATH,
};

#[derive(GodotClass)]
#[class(base=Node)]
pub struct DclFloatingIslandsManager {
    base: Base<Node>,

    candidates: HashMap<(i32, i32), CornerConfig>,
    active: HashMap<(i32, i32), ParcelData>,

    player_parcel: Vector2i,
    view_distance: i32,
    /// Extra ring (past `view_distance`) kept alive to avoid pop-in when the
    /// camera rotates along the boundary.
    destroy_hysteresis: i32,
    /// Cap for creates-per-frame AND destroys-per-frame (applied separately,
    /// not combined).
    frame_budget: i32,

    /// True from `set_candidate_parcels` until the first tick in which every
    /// in-view candidate is materialized. `generation_complete` fires on the
    /// falling edge.
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
    prop_cache: props::PropCache,

    worker: Option<WorkerHandle>,
    /// Coords that were enqueued to the worker and haven't come back yet.
    /// Prevents re-enqueueing the same coord on subsequent ticks while the
    /// worker is still generating its mesh.
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
            view_distance: 5,
            destroy_hysteresis: 2,
            frame_budget: 2,
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
            prop_cache: props::PropCache::default(),
            worker: None,
            pending: HashSet::new(),
        }
    }

    fn process(&mut self, _delta: f64) {
        self.tick_culling();
    }
}

#[godot_api]
impl DclFloatingIslandsManager {
    #[signal]
    fn generation_progress(created: i32, total: i32);

    #[signal]
    fn generation_complete();

    /// `corner_configs` must have length `parcels.len() * 8`. Each 8-byte
    /// window is `[N, S, E, W, NW, NE, SW, SE]`, values matching
    /// `CornerConfiguration.ParcelState` (0=EMPTY, 1=NOTHING, 2=LOADED).
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

        for (i, coord) in parcels.iter_shared().enumerate() {
            let Some(cfg) = CornerConfig::from_packed(&corner_configs, i * 8) else {
                floating_islands::warn_invalid_corner_config(i);
                continue;
            };
            self.candidates.insert((coord.x, coord.y), cfg);
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
        for (_, data) in self.active.drain() {
            Self::destroy_parcel_data(data);
        }
        self.candidates.clear();
        self.pending.clear();
        self.generating = false;
        self.generated_so_far = 0;
        self.generation_total = 0;
        // Drain any in-flight responses so they don't materialize after a
        // clear (e.g. realm switch). They reference nothing in `active` but
        // would still leak their Packed arrays otherwise.
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
        }
        if self.grass_blade_mesh.is_none() {
            let mut loader = ResourceLoader::singleton();
            if let Some(resource) = loader.load(GRASS_BLADE_MESH_PATH) {
                if let Ok(mesh) = resource.try_cast::<Mesh>() {
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

        // Drain anything the worker finished last frame BEFORE we evaluate
        // the visible set — otherwise the same coords would be re-enqueued.
        let submitted_this_frame = self.drain_worker_responses();
        self.generated_so_far += submitted_this_frame;

        let player = self.player_parcel;
        let view = self.view_distance;
        let hyst = self.destroy_hysteresis;
        let budget = self.frame_budget.max(1) as usize;

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

        // Hard floor: the 3x3 around the player is always kept so the ground
        // behind the camera never pops when turning around.
        let keep_radius = hyst.max(1);
        let now_msec = godot::classes::Time::singleton().get_ticks_msec();

        // Classify each currently-active parcel into one of four buckets:
        //   - `to_show`: it was stale but the camera came back; un-hide it.
        //   - `to_hide`: it just dropped out of view; mark stale so we can
        //     reclaim it later without paying the destroy/recreate cost now.
        //   - `to_destroy_immediate`: it is so far from the player that
        //     reviving it later would cost more than a fresh rebuild.
        //   - `to_destroy_stale`: it has been hidden past the grace period.
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
        for coord in to_destroy_immediate {
            if let Some(data) = self.active.remove(&coord) {
                Self::destroy_parcel_data(data);
            }
        }
        for coord in to_destroy_stale.into_iter().take(budget) {
            if let Some(data) = self.active.remove(&coord) {
                Self::destroy_parcel_data(data);
            }
        }

        let mut to_promote_grass: Vec<(i32, i32)> = Vec::new();
        let mut to_demote_grass: Vec<(i32, i32)> = Vec::new();
        for (coord, data) in &self.active {
            let should_have = Self::grass_should_be_visible(*coord, player);
            let has_grass = data.grass_instance.is_valid();
            if should_have && !has_grass {
                to_promote_grass.push(*coord);
            } else if !should_have && has_grass {
                to_demote_grass.push(*coord);
            }
        }
        let blade_rid = self.grass_blade_mesh.as_ref().map(|m| m.get_rid());
        let mat_rid = self.grass_material.as_ref().map(|m| m.get_rid());
        let scenario = self.scenario;
        if let (Some(blade), Some(mat)) = (blade_rid, mat_rid) {
            for coord in to_promote_grass {
                if let Some(data) = self.active.get_mut(&coord) {
                    let transform = parcel_world_transform(coord);
                    build_grass_for_parcel(data, coord, scenario, transform, blade, mat);
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

    /// Send a build request to the worker thread. The heavy mesh generation
    /// happens off the main thread; the manager later picks up the result in
    /// `drain_worker_responses` and does the RenderingServer + PhysicsServer
    /// submits.
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

    /// Consume any meshes the worker finished last frame and submit them to
    /// RenderingServer + PhysicsServer + spawn grass / props. Called at the
    /// top of `tick_culling`.
    fn drain_worker_responses(&mut self) -> i32 {
        let mut ready: Vec<BuiltParcelMeshes> = Vec::new();
        if let Some(worker) = self.worker.as_ref() {
            while let Ok(built) = worker.rx.try_recv() {
                ready.push(built);
            }
        }
        let mut submitted = 0;
        for built in ready {
            self.pending.remove(&built.coord);
            // The candidate set or `active` may have shifted while the worker
            // was busy — only submit if still wanted and not already present.
            if !self.candidates.contains_key(&built.coord) {
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
        let coord = built.coord;
        let config = built.config;

        let world_x = coord.0 as f32 * PARCEL_SIZE + PARCEL_HALF_SIZE;
        let world_z = -(coord.1 as f32 * PARCEL_SIZE + PARCEL_HALF_SIZE);
        let transform = Transform3D::IDENTITY.translated(Vector3::new(world_x, 0.0, world_z));

        let mut rs = RenderingServer::singleton();

        let terrain_vertices = packed_vector3_from_slice(&built.terrain.vertices);
        let terrain_normals = packed_vector3_from_slice(&built.terrain.normals);
        let terrain_uvs = packed_vector2_from_slice(&built.terrain.uvs);
        let terrain_indices = packed_int32_from_slice(&built.terrain.indices);

        let mut arrays = VarArray::new();
        arrays.resize(ArrayType::MAX.ord() as usize, &Variant::nil());
        arrays.set(
            ArrayType::VERTEX.ord() as usize,
            &terrain_vertices.to_variant(),
        );
        arrays.set(
            ArrayType::NORMAL.ord() as usize,
            &terrain_normals.to_variant(),
        );
        arrays.set(ArrayType::TEX_UV.ord() as usize, &terrain_uvs.to_variant());
        arrays.set(
            ArrayType::INDEX.ord() as usize,
            &terrain_indices.to_variant(),
        );

        let mesh_rid = rs.mesh_create();
        rs.mesh_add_surface_from_arrays(mesh_rid, RsPrimitiveType::TRIANGLES, &arrays);

        let instance = rs.instance_create2(mesh_rid, scenario);
        rs.instance_set_transform(instance, transform);

        if let Some(material) = &self.terrain_material {
            rs.instance_geometry_set_material_override(instance, material.get_rid());
        }

        let collision_faces = expand_indexed_faces(&built.terrain.vertices, &built.terrain.indices);
        let (collision_body, collision_shape) =
            Self::build_terrain_collision(&collision_faces, space, transform);

        let mut cliff_meshes: Vec<Rid> = Vec::new();
        let mut cliff_instances: Vec<Rid> = Vec::new();
        let mut overhang_meshes: Vec<Rid> = Vec::new();
        let mut overhang_instances: Vec<Rid> = Vec::new();

        for built_side in &built.sides {
            let (mesh_rid, inst_rid) = self.spawn_indexed_surface(
                scenario,
                transform,
                &built_side.cliff.vertices,
                &built_side.cliff.normals,
                &built_side.cliff.uvs,
                None,
                &built_side.cliff.indices,
                self.cliff_material.as_ref(),
            );
            cliff_meshes.push(mesh_rid);
            cliff_instances.push(inst_rid);

            let (mesh_rid, inst_rid) = self.spawn_indexed_surface(
                scenario,
                transform,
                &built_side.overhang.vertices,
                &built_side.overhang.normals,
                &built_side.overhang.uvs,
                Some(&built_side.overhang.colors),
                &built_side.overhang.indices,
                self.overhang_material.as_ref(),
            );
            overhang_meshes.push(mesh_rid);
            overhang_instances.push(inst_rid);
        }

        let mut data = ParcelData {
            terrain_mesh: mesh_rid,
            terrain_instance: instance,
            collision_body,
            collision_shape,
            cliff_meshes,
            cliff_instances,
            overhang_meshes,
            overhang_instances,
            spawn_locations: built.terrain.spawn_locations,
            ..ParcelData::default()
        };

        let player = self.player_parcel;
        if Self::grass_should_be_visible(coord, player) {
            let blade_rid = self.grass_blade_mesh.as_ref().map(|m| m.get_rid());
            let mat_rid = self.grass_material.as_ref().map(|m| m.get_rid());
            if let (Some(blade), Some(mat)) = (blade_rid, mat_rid) {
                build_grass_for_parcel(&mut data, coord, scenario, transform, blade, mat);
            }
        }

        self.spawn_parcel_props(coord, &config, &mut data, scenario, space, transform);

        self.active.insert(coord, data);
    }

    fn spawn_parcel_props(
        &self,
        coord: (i32, i32),
        config: &CornerConfig,
        data: &mut ParcelData,
        scenario: Rid,
        space: Rid,
        parcel_world: Transform3D,
    ) {
        if !self.prop_cache.is_populated() {
            return;
        }
        let world_origin = parcel_world.origin;
        let mut rng = SimpleRng::new((coord.0 as u32 ^ 0xA53F, coord.1 as u32 ^ 0x91C2));
        let mut ctx = props::SpawnContext {
            scenario,
            space,
            parcel_world,
            parcel_world_origin: world_origin,
            prop_instances: &mut data.prop_instances,
            prop_bodies: &mut data.prop_bodies,
        };
        props::spawn_rocks(&self.prop_cache, &data.spawn_locations, &mut rng, &mut ctx);
        props::spawn_trees(
            &self.prop_cache,
            config,
            &data.spawn_locations,
            &mut rng,
            &mut ctx,
        );
        props::spawn_generic_props(&self.prop_cache, &data.spawn_locations, &mut rng, &mut ctx);
        props::spawn_cliff_rocks(&self.prop_cache, config, &mut rng, &mut ctx);
    }

    fn grass_should_be_visible(coord: (i32, i32), player: Vector2i) -> bool {
        let dist = (coord.0 - player.x).abs().max((coord.1 - player.y).abs());
        dist <= GRASS_CULLING_RANGE
    }

    #[allow(clippy::too_many_arguments)]
    fn spawn_indexed_surface(
        &self,
        scenario: Rid,
        transform: Transform3D,
        vertices: &[Vector3],
        normals: &[Vector3],
        uvs: &[Vector2],
        colors: Option<&[Color]>,
        indices: &[i32],
        material: Option<&Gd<Material>>,
    ) -> (Rid, Rid) {
        let packed_vertices = packed_vector3_from_slice(vertices);
        let packed_normals = packed_vector3_from_slice(normals);
        let packed_uvs = packed_vector2_from_slice(uvs);
        let packed_indices = packed_int32_from_slice(indices);

        let mut rs = RenderingServer::singleton();
        let mut arrays = VarArray::new();
        arrays.resize(ArrayType::MAX.ord() as usize, &Variant::nil());
        arrays.set(
            ArrayType::VERTEX.ord() as usize,
            &packed_vertices.to_variant(),
        );
        arrays.set(
            ArrayType::NORMAL.ord() as usize,
            &packed_normals.to_variant(),
        );
        arrays.set(ArrayType::TEX_UV.ord() as usize, &packed_uvs.to_variant());
        if let Some(c) = colors {
            let packed_colors = packed_color_from_slice(c);
            arrays.set(ArrayType::COLOR.ord() as usize, &packed_colors.to_variant());
        }
        arrays.set(
            ArrayType::INDEX.ord() as usize,
            &packed_indices.to_variant(),
        );

        let mesh_rid = rs.mesh_create();
        rs.mesh_add_surface_from_arrays(mesh_rid, RsPrimitiveType::TRIANGLES, &arrays);

        let instance = rs.instance_create2(mesh_rid, scenario);
        rs.instance_set_transform(instance, transform);
        if let Some(material) = material {
            rs.instance_geometry_set_material_override(instance, material.get_rid());
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
        const OBSTACLE_LAYER: u32 = 1 << 1;

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

    fn destroy_parcel_data(data: ParcelData) {
        let mut rs = RenderingServer::singleton();

        if data.terrain_instance.is_valid() {
            rs.free_rid(data.terrain_instance);
        }
        if data.terrain_mesh.is_valid() {
            rs.free_rid(data.terrain_mesh);
        }
        for rid in data.cliff_instances {
            if rid.is_valid() {
                rs.free_rid(rid);
            }
        }
        for rid in data.cliff_meshes {
            if rid.is_valid() {
                rs.free_rid(rid);
            }
        }
        for rid in data.overhang_instances {
            if rid.is_valid() {
                rs.free_rid(rid);
            }
        }
        for rid in data.overhang_meshes {
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
        for rid in data.prop_instances {
            if rid.is_valid() {
                rs.free_rid(rid);
            }
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

/// A parcel that has been hidden (out of view) for longer than this is freed
/// on the next `tick_culling`. Shorter values reclaim RIDs faster; longer
/// values absorb more back-and-forth camera motion without paying the
/// create/destroy cost. Chosen by eye against typical pan speeds.
const STALE_DEADLINE_MSEC: u64 = 5000;

/// Spawns the mesh-building worker. The worker builds its own noise configs
/// (deterministic from the same seeds as the main thread's, so results
/// match) and receives `(coord, config)` requests to produce terrain +
/// cliff + overhang meshes off the main thread. Results are `Vec<T>` (Send)
/// and the main thread converts them to Packed arrays at the RS submit
/// boundary.
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
    for rid in &data.cliff_instances {
        if rid.is_valid() {
            rs.instance_set_visible(*rid, visible);
        }
    }
    for rid in &data.overhang_instances {
        if rid.is_valid() {
            rs.instance_set_visible(*rid, visible);
        }
    }
    for rid in &data.prop_instances {
        if rid.is_valid() {
            rs.instance_set_visible(*rid, visible);
        }
    }
    if data.grass_instance.is_valid() {
        rs.instance_set_visible(data.grass_instance, visible);
        data.grass_visible = visible;
    }
    // Physics bodies stay active regardless so the player never falls through
    // a parcel that was just hidden for a frame or two.
}

fn build_grass_for_parcel(
    data: &mut ParcelData,
    coord: (i32, i32),
    scenario: Rid,
    transform: Transform3D,
    blade_mesh_rid: Rid,
    grass_material_rid: Rid,
) {
    if data.spawn_locations.is_empty() {
        return;
    }
    let instance_count = data.spawn_locations.len() as i32;

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
    buffer.resize(data.spawn_locations.len() * 12);
    {
        let slice = buffer.as_mut_slice();
        for (i, loc) in data.spawn_locations.iter().enumerate() {
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

/// Writes the 12 floats expected by Godot's `multimesh_set_buffer` for a
/// TRANSFORM_3D instance: three rows of `[basis_row.x, .y, .z, origin.component]`.
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

/// Expand an indexed triangle list back to a flat face list suitable for
/// `concave_polygon_shape_create`. The physics shape doesn't support indexed
/// data, so we re-expand at submit time — it's still a net RAM win because
/// the expanded face list is ephemeral (dropped after the shape is built).
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
