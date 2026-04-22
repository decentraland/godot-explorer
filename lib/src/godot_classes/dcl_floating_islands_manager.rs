use std::collections::HashMap;

use fastnoise_lite::FastNoiseLite;
use godot::builtin::{
    Array, PackedByteArray, PackedFloat32Array, Rid, Transform3D, VarArray, VarDictionary,
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
    terrain_noise: FastNoiseLite,
    cliff_noise: FastNoiseLite,
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
            terrain_noise: terrain::build_terrain_noise(),
            cliff_noise: terrain::build_cliff_noise(),
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
        self.generating = false;
        self.generated_so_far = 0;
        self.generation_total = 0;
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
                if !self.active.contains_key(&coord) {
                    in_view_missing.push(coord);
                }
            }
        }

        let mut created_this_frame = 0;
        for coord in in_view_missing.into_iter().take(budget) {
            self.materialize_parcel(coord);
            self.generated_so_far += 1;
            created_this_frame += 1;
        }

        // Hard floor: the 3x3 around the player is always kept so the ground
        // behind the camera never pops when turning around.
        let keep_radius = hyst.max(1);

        let doomed: Vec<(i32, i32)> = self
            .active
            .keys()
            .copied()
            .filter(|&(x, z)| {
                let dist = (x - player.x).abs().max((z - player.y).abs());
                if dist > view + hyst {
                    return true;
                }
                if dist <= keep_radius {
                    return false;
                }
                !Self::parcel_in_camera_view(&camera, (x, z))
            })
            .take(budget)
            .collect();
        for coord in doomed {
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

    fn materialize_parcel(&mut self, coord: (i32, i32)) {
        if self.active.contains_key(&coord) {
            return;
        }
        if self.ensure_world_resources().is_none() {
            return;
        }
        let scenario = self.scenario;
        let space = self.physics_space;
        let Some(config) = self.candidates.get(&coord).copied() else {
            return;
        };

        let terrain_data =
            terrain::build_terrain_mesh(coord, &config, &self.terrain_noise, &self.cliff_noise);

        let world_x = coord.0 as f32 * PARCEL_SIZE + PARCEL_HALF_SIZE;
        let world_z = -(coord.1 as f32 * PARCEL_SIZE + PARCEL_HALF_SIZE);
        let transform = Transform3D::IDENTITY.translated(Vector3::new(world_x, 0.0, world_z));

        let mut rs = RenderingServer::singleton();

        let mut arrays = VarArray::new();
        arrays.resize(ArrayType::MAX.ord() as usize, &Variant::nil());
        arrays.set(
            ArrayType::VERTEX.ord() as usize,
            &terrain_data.vertices.to_variant(),
        );
        arrays.set(
            ArrayType::NORMAL.ord() as usize,
            &terrain_data.normals.to_variant(),
        );
        arrays.set(
            ArrayType::TEX_UV.ord() as usize,
            &terrain_data.uvs.to_variant(),
        );

        let mesh_rid = rs.mesh_create();
        rs.mesh_add_surface_from_arrays(mesh_rid, RsPrimitiveType::TRIANGLES, &arrays);

        let instance = rs.instance_create2(mesh_rid, scenario);
        rs.instance_set_transform(instance, transform);

        if let Some(material) = &self.terrain_material {
            rs.instance_geometry_set_material_override(instance, material.get_rid());
        }

        let (collision_body, collision_shape) =
            Self::build_terrain_collision(&terrain_data.vertices, space, transform);

        let mut cliff_meshes: Vec<Rid> = Vec::new();
        let mut cliff_instances: Vec<Rid> = Vec::new();
        let mut overhang_meshes: Vec<Rid> = Vec::new();
        let mut overhang_instances: Vec<Rid> = Vec::new();

        for side in cliffs::nothing_sides(&config) {
            let cliff = cliffs::build_cliff_mesh(
                &side,
                coord,
                &config,
                &self.terrain_noise,
                &self.cliff_noise,
            );
            let (mesh_rid, inst_rid) = self.spawn_indexed_surface(
                scenario,
                transform,
                &cliff.vertices,
                &cliff.normals,
                &cliff.uvs,
                None,
                &cliff.indices,
                self.cliff_material.as_ref(),
            );
            cliff_meshes.push(mesh_rid);
            cliff_instances.push(inst_rid);

            let overhang = cliffs::build_overhang_mesh(
                &side,
                coord,
                &config,
                &self.terrain_noise,
                &self.cliff_noise,
            );
            let (mesh_rid, inst_rid) = self.spawn_indexed_surface(
                scenario,
                transform,
                &overhang.vertices,
                &overhang.normals,
                &overhang.uvs,
                Some(&overhang.colors),
                &overhang.indices,
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
            spawn_locations: terrain_data.spawn_locations,
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
        vertices: &godot::builtin::PackedVector3Array,
        normals: &godot::builtin::PackedVector3Array,
        uvs: &godot::builtin::PackedVector2Array,
        colors: Option<&godot::builtin::PackedColorArray>,
        indices: &godot::builtin::PackedInt32Array,
        material: Option<&Gd<Material>>,
    ) -> (Rid, Rid) {
        let mut rs = RenderingServer::singleton();
        let mut arrays = VarArray::new();
        arrays.resize(ArrayType::MAX.ord() as usize, &Variant::nil());
        arrays.set(ArrayType::VERTEX.ord() as usize, &vertices.to_variant());
        arrays.set(ArrayType::NORMAL.ord() as usize, &normals.to_variant());
        arrays.set(ArrayType::TEX_UV.ord() as usize, &uvs.to_variant());
        if let Some(c) = colors {
            arrays.set(ArrayType::COLOR.ord() as usize, &c.to_variant());
        }
        arrays.set(ArrayType::INDEX.ord() as usize, &indices.to_variant());

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
        faces: &godot::builtin::PackedVector3Array,
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

fn parcel_world_transform(coord: (i32, i32)) -> Transform3D {
    let x = coord.0 as f32 * PARCEL_SIZE + PARCEL_HALF_SIZE;
    let z = -(coord.1 as f32 * PARCEL_SIZE + PARCEL_HALF_SIZE);
    Transform3D::IDENTITY.translated(Vector3::new(x, 0.0, z))
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
