use godot::builtin::{Aabb, Basis, Rid, Transform3D, Vector3};
use godot::classes::physics_server_3d::{BodyMode, BodyState};
use godot::classes::{
    CollisionShape3D, MeshInstance3D, Node, PhysicsServer3D, RenderingServer, SceneTree, Shape3D,
};
use godot::obj::Gd;
use godot::prelude::*;

use super::{CornerConfig, ParcelState, SimpleRng, SpawnLocation};
use super::{PARCEL_FULL_HEIGHT, PARCEL_HALF_SIZE, PARCEL_HEIGHT_BOUND, PARCEL_SIZE};

const OBSTACLE_LAYER: u32 = 1 << 1;

const ROCK_MIN_FALLOFF: f32 = 0.5;
const ROCK_MIN: i32 = 0;
const ROCK_MAX: i32 = 1;

const TREE_MIN_FALLOFF: f32 = 0.5;
const TREE_MIN: i32 = 0;
const TREE_MAX: i32 = 3;
const TREE_MIN_SCALE: f32 = 1.0;
const TREE_MAX_SCALE: f32 = 2.0;

const PROP_MIN: i32 = 2;
const PROP_MAX: i32 = 3;
const PROP_MIN_SCALE: f32 = 0.8;
const PROP_MAX_SCALE: f32 = 1.2;

const CLIFF_ROCK_MIN: i32 = 0;
const CLIFF_ROCK_MAX: i32 = 3;
const CLIFF_ROCK_HEIGHT: f32 = 30.0;
const CLIFF_ROCK_H_RANGE: f32 = 7.0;

pub struct CachedVisual {
    pub mesh: Gd<godot::classes::Mesh>,
    pub transform: Transform3D,
}

pub struct CachedCollision {
    pub shape: Gd<Shape3D>,
    pub transform: Transform3D,
}

pub struct CachedProp {
    pub visuals: Vec<CachedVisual>,
    pub collisions: Vec<CachedCollision>,
    pub aabb: Aabb,
}

impl CachedProp {
    fn is_empty(&self) -> bool {
        self.visuals.is_empty() && self.collisions.is_empty()
    }
}

#[derive(Default)]
pub struct PropCache {
    pub props: Vec<CachedProp>,
    pub trees: Vec<CachedProp>,
    pub rocks: Vec<CachedProp>,
}

impl PropCache {
    pub fn is_populated(&self) -> bool {
        !self.props.is_empty() || !self.trees.is_empty() || !self.rocks.is_empty()
    }

    /// Walks `/root/EmptyParcelProps` and extracts meshes + collision shapes.
    /// Returns `None` if the autoload isn't ready yet (scene tree not built).
    pub fn load_from_autoload(tree: &Gd<SceneTree>) -> Option<Self> {
        let root = tree.clone().get_root()?;
        let autoload = root.upcast::<Node>().get_node_or_null("EmptyParcelProps")?;

        let props = load_category(&autoload, "%Props");
        let trees = load_category(&autoload, "%Trees");
        let rocks = load_category(&autoload, "%Rocks");

        Some(Self {
            props,
            trees,
            rocks,
        })
    }
}

fn load_category(autoload: &Gd<Node>, unique_name: &str) -> Vec<CachedProp> {
    let Some(parent) = autoload.get_node_or_null(unique_name) else {
        return Vec::new();
    };
    let child_count = parent.get_child_count();
    let mut out = Vec::with_capacity(child_count as usize);
    for i in 0..child_count {
        let Some(child) = parent.get_child(i) else {
            continue;
        };
        let prop = build_cached_prop(&child);
        if !prop.is_empty() {
            out.push(prop);
        }
    }
    out
}

fn build_cached_prop(root: &Gd<Node>) -> CachedProp {
    let mut visuals: Vec<CachedVisual> = Vec::new();
    let mut collisions: Vec<CachedCollision> = Vec::new();
    walk_for_resources(
        root,
        Transform3D::IDENTITY,
        &mut visuals,
        &mut collisions,
        true,
    );

    let mut aabb: Option<Aabb> = None;
    for v in &visuals {
        let mesh_aabb = v.mesh.clone().get_aabb();
        let transformed = v.transform * mesh_aabb;
        aabb = Some(match aabb {
            Some(current) => current.merge(transformed),
            None => transformed,
        });
    }

    CachedProp {
        visuals,
        collisions,
        aabb: aabb.unwrap_or_default(),
    }
}

fn walk_for_resources(
    node: &Gd<Node>,
    acc: Transform3D,
    visuals: &mut Vec<CachedVisual>,
    collisions: &mut Vec<CachedCollision>,
    is_root: bool,
) {
    let local = node
        .clone()
        .try_cast::<godot::classes::Node3D>()
        .ok()
        .map(|n| n.get_transform())
        .unwrap_or(Transform3D::IDENTITY);

    // The prop root's own transform is intentionally discarded so spawn-time
    // placement can fully define the world transform.
    let current = if is_root { acc } else { acc * local };

    if let Ok(mi) = node.clone().try_cast::<MeshInstance3D>() {
        if let Some(mesh) = mi.get_mesh() {
            visuals.push(CachedVisual {
                mesh,
                transform: current,
            });
        }
    }

    if let Ok(cs) = node.clone().try_cast::<CollisionShape3D>() {
        if let Some(shape) = cs.get_shape() {
            collisions.push(CachedCollision {
                shape,
                transform: current,
            });
        }
    }

    let child_count = node.get_child_count();
    for i in 0..child_count {
        if let Some(child) = node.get_child(i) {
            walk_for_resources(&child, current, visuals, collisions, false);
        }
    }
}

pub struct SpawnContext<'a> {
    pub scenario: Rid,
    pub space: Rid,
    pub parcel_world: Transform3D,
    pub parcel_world_origin: Vector3,
    pub prop_instances: &'a mut Vec<Rid>,
    pub prop_bodies: &'a mut Vec<Rid>,
}

pub fn spawn_rocks(
    cache: &PropCache,
    spawn_locations: &[SpawnLocation],
    rng: &mut SimpleRng,
    ctx: &mut SpawnContext,
) {
    if cache.rocks.is_empty() || spawn_locations.is_empty() {
        return;
    }
    let count = random_range(rng, ROCK_MIN, ROCK_MAX);
    if count == 0 {
        return;
    }
    let available = collect_indices_by_falloff(spawn_locations, ROCK_MIN_FALLOFF);
    let picks = pick_indices(&available, count as usize, rng);
    for idx in picks {
        let loc = &spawn_locations[idx];
        let rock = &cache.rocks[(rng.next_u32() as usize) % cache.rocks.len()];
        let scale = 1.0 + rng.next_f32();
        let local_transform = super::aligned_transform(loc.position, loc.normal, Some(rng), scale);
        instantiate_prop(rock, local_transform, ctx);
    }
}

pub fn spawn_trees(
    cache: &PropCache,
    config: &CornerConfig,
    spawn_locations: &[SpawnLocation],
    rng: &mut SimpleRng,
    ctx: &mut SpawnContext,
) {
    if cache.trees.is_empty() || spawn_locations.is_empty() {
        return;
    }
    let count = random_range(rng, TREE_MIN, TREE_MAX);
    if count == 0 {
        return;
    }
    let available = collect_indices_by_falloff(spawn_locations, TREE_MIN_FALLOFF);
    let picks = pick_indices(&available, count as usize, rng);
    for idx in picks {
        let loc = &spawn_locations[idx];
        let tree = &cache.trees[(rng.next_u32() as usize) % cache.trees.len()];
        let scale = TREE_MIN_SCALE + rng.next_f32() * (TREE_MAX_SCALE - TREE_MIN_SCALE);
        let local_transform = super::aligned_transform(loc.position, Vector3::UP, Some(rng), scale);
        if tree_would_overlap(tree, local_transform, ctx.parcel_world_origin, config) {
            continue;
        }
        instantiate_prop(tree, local_transform, ctx);
    }
}

pub fn spawn_generic_props(
    cache: &PropCache,
    spawn_locations: &[SpawnLocation],
    rng: &mut SimpleRng,
    ctx: &mut SpawnContext,
) {
    if cache.props.is_empty() || spawn_locations.is_empty() {
        return;
    }
    let count = random_range(rng, PROP_MIN, PROP_MAX);
    if count == 0 {
        return;
    }
    let all_indices: Vec<usize> = (0..spawn_locations.len()).collect();
    let picks = pick_indices(&all_indices, count as usize, rng);
    for idx in picks {
        let loc = &spawn_locations[idx];
        let prop = &cache.props[(rng.next_u32() as usize) % cache.props.len()];
        let tilt = loc.normal.slerp(Vector3::UP, 0.5);
        let scale = PROP_MIN_SCALE + rng.next_f32() * (PROP_MAX_SCALE - PROP_MIN_SCALE);
        let local_transform = super::aligned_transform(loc.position, tilt, Some(rng), scale);
        instantiate_prop(prop, local_transform, ctx);
    }
}

pub fn spawn_cliff_rocks(
    cache: &PropCache,
    config: &CornerConfig,
    rng: &mut SimpleRng,
    ctx: &mut SpawnContext,
) {
    if cache.rocks.is_empty() {
        return;
    }

    let sides: [(ParcelState, Vector3, Vector3); 4] = [
        (
            config.north,
            Vector3::new(0.0, 0.0, -PARCEL_HALF_SIZE),
            Vector3::new(0.0, 0.0, -1.0),
        ),
        (
            config.south,
            Vector3::new(0.0, 0.0, PARCEL_HALF_SIZE),
            Vector3::new(0.0, 0.0, 1.0),
        ),
        (
            config.east,
            Vector3::new(PARCEL_HALF_SIZE, 0.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
        ),
        (
            config.west,
            Vector3::new(-PARCEL_HALF_SIZE, 0.0, 0.0),
            Vector3::new(-1.0, 0.0, 0.0),
        ),
    ];

    for (state, edge_pos, outward) in sides {
        if state != ParcelState::Nothing {
            continue;
        }
        let count = random_range(rng, CLIFF_ROCK_MIN, CLIFF_ROCK_MAX);
        let is_horizontal = outward.z.abs() > 0.5;

        for _ in 0..count {
            let h = -CLIFF_ROCK_H_RANGE + rng.next_f32() * (CLIFF_ROCK_H_RANGE * 2.0);
            let v = -CLIFF_ROCK_HEIGHT * 0.8
                + rng.next_f32() * (CLIFF_ROCK_HEIGHT * 0.8 - CLIFF_ROCK_HEIGHT * 0.2);

            let mut rock_pos = if is_horizontal {
                edge_pos + Vector3::new(h, v, 0.0)
            } else {
                edge_pos + Vector3::new(0.0, v, h)
            };
            rock_pos -= outward * (0.5 * rng.next_f32());

            let rock = &cache.rocks[(rng.next_u32() as usize) % cache.rocks.len()];
            let scale = 0.8 + rng.next_f32() * 0.7;
            let yaw = rng.next_f32() * std::f32::consts::TAU;
            let up = -outward;
            let right = Vector3::UP.cross(up).normalized();
            let right = if right.length() < 0.01 {
                Vector3::RIGHT
            } else {
                right
            };
            let forward = right.cross(up).normalized();
            let mut basis = Basis::from_cols(right, up, forward);
            basis = Basis::from_axis_angle(up, yaw) * basis;
            basis = basis.scaled(Vector3::ONE * scale);

            let local_transform = Transform3D::new(basis, rock_pos);
            instantiate_prop(rock, local_transform, ctx);
        }
    }
}

fn instantiate_prop(prop: &CachedProp, local_transform: Transform3D, ctx: &mut SpawnContext) {
    let world_transform = ctx.parcel_world * local_transform;

    let mut rs = RenderingServer::singleton();
    for visual in &prop.visuals {
        let inst = rs.instance_create2(visual.mesh.get_rid(), ctx.scenario);
        rs.instance_set_transform(inst, world_transform * visual.transform);
        ctx.prop_instances.push(inst);
    }

    if prop.collisions.is_empty() {
        return;
    }
    let mut physics = PhysicsServer3D::singleton();
    let body = physics.body_create();
    physics.body_set_mode(body, BodyMode::STATIC);
    physics.body_set_space(body, ctx.space);
    for collision in &prop.collisions {
        physics
            .body_add_shape_ex(body, collision.shape.get_rid())
            .transform(collision.transform)
            .done();
    }
    physics.body_set_state(body, BodyState::TRANSFORM, &world_transform.to_variant());
    physics.body_set_collision_layer(body, OBSTACLE_LAYER);
    ctx.prop_bodies.push(body);
}

fn tree_would_overlap(
    tree: &CachedProp,
    local_transform: Transform3D,
    parcel_world_origin: Vector3,
    config: &CornerConfig,
) -> bool {
    if tree.aabb.size == Vector3::ZERO {
        return false;
    }
    let world_transform = Transform3D::new(Basis::IDENTITY, parcel_world_origin) * local_transform;
    let world_aabb = world_transform * tree.aabb;

    let size = Vector3::new(PARCEL_SIZE, PARCEL_FULL_HEIGHT, PARCEL_SIZE);
    let base_min =
        parcel_world_origin - Vector3::new(PARCEL_HALF_SIZE, PARCEL_HEIGHT_BOUND, PARCEL_HALF_SIZE);
    let base_max =
        parcel_world_origin + Vector3::new(PARCEL_HALF_SIZE, PARCEL_HEIGHT_BOUND, PARCEL_HALF_SIZE);

    let neighbors: [(ParcelState, Vector3); 8] = [
        (
            config.north,
            Vector3::new(base_min.x, base_min.y, base_min.z - PARCEL_SIZE),
        ),
        (
            config.south,
            Vector3::new(base_min.x, base_min.y, base_max.z),
        ),
        (
            config.east,
            Vector3::new(base_max.x, base_min.y, base_min.z),
        ),
        (
            config.west,
            Vector3::new(base_min.x - PARCEL_SIZE, base_min.y, base_min.z),
        ),
        (
            config.northwest,
            Vector3::new(
                base_min.x - PARCEL_SIZE,
                base_min.y,
                base_min.z - PARCEL_SIZE,
            ),
        ),
        (
            config.northeast,
            Vector3::new(base_max.x, base_min.y, base_min.z - PARCEL_SIZE),
        ),
        (
            config.southwest,
            Vector3::new(base_min.x - PARCEL_SIZE, base_min.y, base_max.z),
        ),
        (
            config.southeast,
            Vector3::new(base_max.x, base_min.y, base_max.z),
        ),
    ];

    for (state, origin) in neighbors {
        if state != ParcelState::Loaded {
            continue;
        }
        let neighbor = Aabb {
            position: origin,
            size,
        };
        if world_aabb.intersects(neighbor) {
            return true;
        }
    }
    false
}

fn collect_indices_by_falloff(spawn_locations: &[SpawnLocation], min_falloff: f32) -> Vec<usize> {
    spawn_locations
        .iter()
        .enumerate()
        .filter(|(_, loc)| loc.falloff > min_falloff)
        .map(|(i, _)| i)
        .collect()
}

fn pick_indices(source: &[usize], count: usize, rng: &mut SimpleRng) -> Vec<usize> {
    if source.is_empty() || count == 0 {
        return Vec::new();
    }
    let mut remaining: Vec<usize> = source.to_vec();
    let mut out: Vec<usize> = Vec::with_capacity(count.min(remaining.len()));
    while out.len() < count && !remaining.is_empty() {
        let idx = (rng.next_u32() as usize) % remaining.len();
        out.push(remaining.swap_remove(idx));
    }
    out
}

fn random_range(rng: &mut SimpleRng, min: i32, max: i32) -> i32 {
    if max <= min {
        return min;
    }
    let span = (max - min + 1) as u32;
    min + ((rng.next_u32() % span) as i32)
}
